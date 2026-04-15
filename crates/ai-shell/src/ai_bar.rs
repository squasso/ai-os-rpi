use egui::{Context, TopBottomPanel, TextEdit, RichText, Color32};
use std::sync::{Arc, Mutex};
use common::ipc::{ClientMsg, ServerMsg, SOCKET_PATH};
use common::types::AiRequest;

pub struct AiBar {
    input:    String,
    history:  Vec<ChatEntry>,
    state:    Arc<Mutex<AsyncState>>,
}

struct ChatEntry {
    role:  Role,
    text:  String,
    cost:  Option<f64>,
    model: Option<String>,
}

enum Role { User, Assistant }

/// Stato condiviso tra UI thread e worker thread
#[derive(Default)]
struct AsyncState {
    waiting:  bool,
    pending:  Option<ServerMsg>,   // risposta pronta da mostrare
}

impl AiBar {
    pub fn new() -> Self {
        Self {
            input:   String::new(),
            history: vec![ChatEntry {
                role:  Role::Assistant,
                text:  "Ciao! Come posso aiutarti?".into(),
                cost:  None,
                model: None,
            }],
            state: Arc::new(Mutex::new(AsyncState::default())),
        }
    }

    pub fn show(&mut self, ctx: &Context) {
        // Controlla se è arrivata una risposta dal worker thread
        self.poll_response(ctx);

        let waiting = self.state.lock().unwrap().waiting;

        TopBottomPanel::bottom("ai_bar")
            .min_height(52.0)
            .max_height(320.0)
            .resizable(true)
            .show(ctx, |ui| {
                // storico messaggi
                if self.history.len() > 1 {
                    egui::ScrollArea::vertical()
                        .stick_to_bottom(true)
                        .max_height(240.0)
                        .show(ui, |ui| {
                            for entry in &self.history {
                                self.draw_entry(ui, entry);
                            }
                        });
                    ui.separator();
                }

                // riga di input
                ui.horizontal(|ui| {
                    // spinner animato mentre aspetta
                    if waiting {
                        ui.spinner();
                    } else {
                        ui.label(
                            RichText::new("›")
                                .size(18.0)
                                .color(Color32::from_rgb(126, 200, 227)),
                        );
                    }

                    let response = ui.add_enabled(
                        !waiting,
                        TextEdit::singleline(&mut self.input)
                            .hint_text("Chiedi qualcosa al sistema…")
                            .desired_width(ui.available_width() - 60.0)
                            .frame(false),
                    );

                    let send = ui.add_enabled(
                        !waiting && !self.input.trim().is_empty(),
                        egui::Button::new("↵"),
                    );

                    if (send.clicked()
                        || (response.lost_focus()
                            && ui.input(|i| i.key_pressed(egui::Key::Enter))))
                        && !self.input.trim().is_empty()
                        && !waiting
                    {
                        self.submit(ctx);
                    }
                });
            });
    }

    fn draw_entry(&self, ui: &mut egui::Ui, entry: &ChatEntry) {
        ui.horizontal_wrapped(|ui| {
            match entry.role {
                Role::User => {
                    ui.label(RichText::new("Tu:").strong());
                    ui.label(&entry.text);
                }
                Role::Assistant => {
                    ui.label(
                        RichText::new("AI:")
                            .strong()
                            .color(Color32::from_rgb(126, 200, 227)),
                    );
                    ui.label(&entry.text);

                    // Mostra modello e costo accanto a ogni risposta
                    if entry.cost.is_some() || entry.model.is_some() {
                        let mut meta = String::new();
                        if let Some(m) = &entry.model {
                            meta.push_str(m);
                        }
                        if let Some(cost) = entry.cost {
                            if !meta.is_empty() { meta.push_str(" · "); }
                            meta.push_str(&format!("${:.4}", cost));
                        }
                        ui.label(
                            RichText::new(meta)
                                .small()
                                .color(Color32::GRAY),
                        );
                    }
                }
            }
        });
    }

    fn submit(&mut self, ctx: &Context) {
        let prompt = self.input.trim().to_string();
        if prompt.is_empty() { return; }

        self.history.push(ChatEntry {
            role:  Role::User,
            text:  prompt.clone(),
            cost:  None,
            model: None,
        });
        self.input.clear();

        // Segna come in attesa
        self.state.lock().unwrap().waiting = true;

        // Lancia la chiamata in un thread separato
        let state   = self.state.clone();
        let ctx     = ctx.clone();

        std::thread::spawn(move || {
            let result = send_to_daemon(prompt);
            let msg = match result {
                Ok(m)  => m,
                Err(e) => ServerMsg::Error { reason: e.to_string() },
            };
            {
                let mut s = state.lock().unwrap();
                s.pending = Some(msg);
                s.waiting = false;
            }
            // Sveglia la UI
            ctx.request_repaint();
        });
    }

    /// Controlla se il worker ha prodotto una risposta e la mostra
    fn poll_response(&mut self, _ctx: &Context) {
        let pending = {
            let mut s = self.state.lock().unwrap();
            s.pending.take()
        };
        if let Some(msg) = pending {
            match msg {
                ServerMsg::Response(r) => {
                    self.history.push(ChatEntry {
                        role:  Role::Assistant,
                        text:  r.text,
                        cost:  Some(r.cost_usd),
                        model: Some(r.model_used.display_name().to_string()),
                    });
                }
                ServerMsg::Error { reason } => {
                    // Distingui tra daemon offline e altri errori
                    let friendly = if reason.contains("Connection refused")
                        || reason.contains("No such file or directory")
                        || reason.contains("os error 111")
                        || reason.contains("os error 2")
                    {
                        "Il daemon non è in esecuzione.\n\
                         Avvialo con:  systemctl --user start ai-daemon\n\
                         oppure esegui direttamente: ai-daemon".to_string()
                    } else if reason.contains("401") || reason.contains("API key") {
                        "Chiave API non valida o mancante.\n\
                         Configurala in: Impostazioni → AI & Budget".to_string()
                    } else if reason.contains("Budget mensile") {
                        reason.clone()
                    } else {
                        format!("Errore: {reason}")
                    };
                    self.history.push(ChatEntry {
                        role:  Role::Assistant,
                        text:  friendly,
                        cost:  None,
                        model: None,
                    });
                }
                _ => {}
            }
        }
    }
}

fn send_to_daemon(prompt: String) -> anyhow::Result<ServerMsg> {
    use std::io::{Read, Write};
    use std::os::unix::net::UnixStream;

    let mut stream = UnixStream::connect(SOCKET_PATH)?;

    let req  = ClientMsg::Ask(AiRequest::new(prompt));
    let body = serde_json::to_vec(&req)?;
    let len  = (body.len() as u32).to_be_bytes();
    stream.write_all(&len)?;
    stream.write_all(&body)?;

    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf)?;
    let reply_len = u32::from_be_bytes(len_buf) as usize;
    let mut buf = vec![0u8; reply_len];
    stream.read_exact(&mut buf)?;

    Ok(serde_json::from_slice(&buf)?)
}
