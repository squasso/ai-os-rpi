use egui::{Context, TopBottomPanel, menu, RichText, Color32};
use chrono::Local;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use common::ipc::{ClientMsg, ServerMsg, SOCKET_PATH};
use common::types::BudgetStatus;
use crate::config::AppConfig;

pub struct MenuBar {
    focused_app:   String,
    budget:        Arc<Mutex<BudgetState>>,
    last_poll:     Instant,
}

#[derive(Default)]
struct BudgetState {
    status:   Option<BudgetStatus>,
    fetching: bool,
}

impl MenuBar {
    pub fn new() -> Self {
        let budget = Arc::new(Mutex::new(BudgetState::default()));
        // Prima fetch immediata
        Self::spawn_fetch(budget.clone(), egui::Context::default());
        Self {
            focused_app: "AI-OS".to_string(),
            budget,
            last_poll: Instant::now(),
        }
    }

    pub fn show(&mut self, ctx: &Context, _config: &AppConfig) {
        // Aggiorna il budget ogni 30 secondi
        if self.last_poll.elapsed() > Duration::from_secs(30) {
            self.last_poll = Instant::now();
            Self::spawn_fetch(self.budget.clone(), ctx.clone());
        }

        TopBottomPanel::top("menubar")
            .exact_height(28.0)
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    // ── sinistra: logo + menu app ───────────────────────
                    ui.add_space(8.0);
                    ui.label(RichText::new("").size(16.0));
                    ui.separator();
                    ui.label(RichText::new(&self.focused_app).strong());
                    ui.separator();

                    menu::bar(ui, |ui| {
                        ui.menu_button("File", |ui| {
                            if ui.button("Nuovo documento").clicked() { ui.close_menu(); }
                            if ui.button("Apri…").clicked() { ui.close_menu(); }
                            ui.separator();
                            if ui.button("Esci").clicked() { std::process::exit(0); }
                        });
                        ui.menu_button("Modifica", |ui| {
                            if ui.button("Copia").clicked() { ui.close_menu(); }
                            if ui.button("Incolla").clicked() { ui.close_menu(); }
                        });
                        ui.menu_button("Visualizza", |ui| {
                            if ui.button("Schermo intero").clicked() { ui.close_menu(); }
                        });
                    });

                    // ── destra: orologio + budget ───────────────────────
                    ui.with_layout(
                        egui::Layout::right_to_left(egui::Align::Center),
                        |ui| {
                            ui.add_space(12.0);

                            // orologio
                            let now = Local::now().format("%H:%M").to_string();
                            ui.label(RichText::new(now).monospace());
                            ui.separator();

                            // budget
                            self.draw_budget(ui);
                            ui.separator();

                            // stato rete (placeholder)
                            ui.label(
                                RichText::new("●")
                                    .color(Color32::from_rgb(107, 203, 139))
                                    .small(),
                            );
                        },
                    );
                });
            });
    }

    fn draw_budget(&self, ui: &mut egui::Ui) {
        let state = self.budget.lock().unwrap();

        match &state.status {
            None => {
                // In attesa della prima risposta
                if state.fetching {
                    ui.spinner();
                } else {
                    ui.label(RichText::new("💰 —").small().color(Color32::GRAY));
                }
            }
            Some(b) => {
                let pct  = b.spent_month_usd / b.limit_month_usd * 100.0;
                let color = if b.blocked {
                    Color32::from_rgb(224, 108, 117)   // rosso — limite raggiunto
                } else if b.warning {
                    Color32::from_rgb(240, 192, 96)    // giallo — vicino al limite
                } else {
                    Color32::from_rgb(107, 203, 139)   // verde — ok
                };

                let label = format!(
                    "💰 ${:.2} / ${:.0}",
                    b.spent_month_usd,
                    b.limit_month_usd,
                );

                let text = RichText::new(label).small().color(color);

                // Tooltip con dettagli
                ui.label(text).on_hover_ui(|ui| {
                    ui.label(RichText::new("Budget API").strong());
                    ui.separator();
                    ui.label(format!("Oggi:       ${:.4}", b.spent_today_usd));
                    ui.label(format!("Questo mese: ${:.4}", b.spent_month_usd));
                    ui.label(format!("Limite mese: ${:.2}", b.limit_month_usd));
                    ui.label(format!("Limite giorno: ${:.2}", b.limit_daily_usd));
                    ui.separator();
                    ui.label(format!("Utilizzo: {:.1}%", pct));
                    if b.warning {
                        ui.label(
                            RichText::new("⚠ Vicino al limite mensile")
                                .color(Color32::from_rgb(240, 192, 96)),
                        );
                    }
                    if b.blocked {
                        ui.label(
                            RichText::new("✗ Limite mensile raggiunto")
                                .color(Color32::from_rgb(224, 108, 117)),
                        );
                    }
                });
            }
        }
    }

    pub fn set_focused_app(&mut self, name: &str) {
        self.focused_app = name.to_string();
    }

    fn spawn_fetch(budget: Arc<Mutex<BudgetState>>, ctx: Context) {
        {
            let mut s = budget.lock().unwrap();
            if s.fetching { return; }
            s.fetching = true;
        }
        std::thread::spawn(move || {
            let result = fetch_budget();
            let mut s = budget.lock().unwrap();
            s.fetching = false;
            match result {
                Ok(status) => s.status = Some(status),
                Err(_)     => {} // daemon non ancora disponibile — riproverà
            }
            ctx.request_repaint();
        });
    }
}

fn fetch_budget() -> anyhow::Result<BudgetStatus> {
    use std::io::{Read, Write};
    use std::os::unix::net::UnixStream;

    let mut stream = UnixStream::connect(SOCKET_PATH)?;
    stream.set_read_timeout(Some(Duration::from_secs(3)))?;

    let body = serde_json::to_vec(&ClientMsg::GetBudget)?;
    let len  = (body.len() as u32).to_be_bytes();
    stream.write_all(&len)?;
    stream.write_all(&body)?;

    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf)?;
    let reply_len = u32::from_be_bytes(len_buf) as usize;
    let mut buf = vec![0u8; reply_len];
    stream.read_exact(&mut buf)?;

    match serde_json::from_slice(&buf)? {
        ServerMsg::Budget(b) => Ok(b),
        other => anyhow::bail!("risposta inattesa: {:?}", other),
    }
}
