use egui::{Color32, ComboBox, Context, RichText, Slider, Window};
use crate::config::{AppConfig, AppEntry};
use crate::daemon_config::{DaemonConfig, WpCfg, GhostCfg, UpdateCfg};

const SECTIONS: &[&str] = &["Aspetto", "Layout", "Applicazioni", "AI & Budget", "Aggiornamenti"];

pub struct SettingsUi {
    /// Copia di lavoro della config shell
    draft:          AppConfig,
    /// Copia di lavoro della config daemon
    daemon:         DaemonConfig,
    /// Sezione attualmente selezionata
    section:        usize,
    /// Buffer per aggiungere una nuova app
    new_app:        AppEntry,
    /// Indice app selezionata per modifica (None = nessuna)
    edit_app:       Option<usize>,
    /// Feedback ultimo controllo aggiornamenti (mostrato nella UI)
    update_output:  Option<String>,
}

impl SettingsUi {
    pub fn new(config: AppConfig) -> Self {
        Self::open_at(config, 0)
    }

    /// Apre le impostazioni direttamente su una sezione specifica.
    /// Usato dall'onboarding per saltare a "AI & Budget" (sezione 3).
    pub fn open_at(config: AppConfig, section: usize) -> Self {
        Self {
            draft:         config,
            daemon:        DaemonConfig::load(),
            section,
            new_app:       default_new_app(),
            edit_app:      None,
            update_output: None,
        }
    }

    /// Imposta la sezione attiva dall'esterno (es. dopo apertura da onboarding).
    pub fn jump_to(&mut self, section: usize) {
        self.section = section;
        // Ricarica la daemon config per avere valori freschi
        self.daemon = DaemonConfig::load();
    }

    /// Ritorna `Some(shell_config)` se l'utente ha salvato, altrimenti `None`.
    pub fn show(&mut self, ctx: &Context) -> Option<AppConfig> {
        let mut saved  = false;
        let mut closed = false;

        Window::new("⚙  Impostazioni")
            .collapsible(false)
            .resizable(true)
            .default_width(560.0)
            .default_height(420.0)
            .show(ctx, |ui| {
                ui.columns(2, |cols| {
                    // ── Colonna sinistra: menu sezioni ──────────────────────
                    cols[0].set_max_width(130.0);
                    cols[0].vertical(|ui| {
                        for (i, label) in SECTIONS.iter().enumerate() {
                            let selected = i == self.section;
                            if ui.selectable_label(selected, *label).clicked() {
                                self.section = i;
                            }
                        }
                    });

                    // ── Colonna destra: contenuto sezione ───────────────────
                    cols[1].vertical(|ui| {
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            match self.section {
                                0 => self.show_appearance(ui),
                                1 => self.show_layout(ui),
                                2 => self.show_apps(ui),
                                3 => self.show_ai_budget(ui),
                                4 => self.show_updates(ui),
                                _ => {}
                            }
                        });

                        ui.add_space(12.0);
                        ui.separator();
                        ui.horizontal(|ui| {
                            if ui.button(
                                RichText::new("Salva").color(Color32::WHITE)
                            ).clicked() {
                                saved = true;
                            }
                            if ui.button("Annulla").clicked() {
                                closed = true;
                            }
                        });
                    });
                });
            });

        if saved {
            self.daemon.save();
            Some(self.draft.clone())
        } else if closed {
            // Ricarica daemon config per scartare modifiche non salvate
            self.daemon = DaemonConfig::load();
            None
        } else {
            None
        }
    }

    // ── Sezione 0: Aspetto ─────────────────────────────────────────────────

    fn show_appearance(&mut self, ui: &mut egui::Ui) {
        ui.heading("Aspetto");
        ui.separator();

        ui.horizontal(|ui| {
            ui.label("Tema:");
            ComboBox::from_id_salt("theme_mode")
                .selected_text(&self.draft.appearance.mode)
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.draft.appearance.mode, "dark".into(),  "Scuro");
                    ui.selectable_value(&mut self.draft.appearance.mode, "light".into(), "Chiaro");
                    ui.selectable_value(&mut self.draft.appearance.mode, "auto".into(),  "Auto");
                });
        });

        ui.horizontal(|ui| {
            ui.label("Colore accent:");
            ui.text_edit_singleline(&mut self.draft.appearance.accent_color);
        });

        ui.horizontal(|ui| {
            ui.label("Wallpaper:");
            let mut wp = self.draft.appearance.wallpaper.clone().unwrap_or_default();
            if ui.text_edit_singleline(&mut wp).changed() {
                self.draft.appearance.wallpaper =
                    if wp.is_empty() { None } else { Some(wp) };
            }
        });
        ui.label(
            RichText::new("Inserisci il percorso completo del file immagine (es. /home/pi/sfondo.jpg)")
                .small()
                .color(Color32::GRAY),
        );

        ui.horizontal(|ui| {
            ui.label("Tema icone:");
            ComboBox::from_id_salt("icon_theme")
                .selected_text(&self.draft.appearance.icon_theme)
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.draft.appearance.icon_theme, "default".into(), "Default");
                    ui.selectable_value(&mut self.draft.appearance.icon_theme, "papirus".into(),  "Papirus");
                    ui.selectable_value(&mut self.draft.appearance.icon_theme, "numix".into(),    "Numix");
                });
        });

        ui.horizontal(|ui| {
            ui.label("Dimensione font:");
            ui.add(Slider::new(&mut self.draft.appearance.font_size, 10.0..=24.0).suffix(" pt"));
        });

        ui.horizontal(|ui| {
            ui.label("Scaling UI:");
            ui.add(Slider::new(&mut self.draft.appearance.scaling, 0.75..=2.0).step_by(0.25));
        });
    }

    // ── Sezione 1: Layout ──────────────────────────────────────────────────

    fn show_layout(&mut self, ui: &mut egui::Ui) {
        ui.heading("Layout");
        ui.separator();

        ui.horizontal(|ui| {
            ui.label("Barra app:");
            for (pos, label) in &[
                ("left",   "Sinistra"),
                ("right",  "Destra"),
                ("top",    "In alto"),
                ("bottom", "In basso"),
            ] {
                ui.selectable_value(&mut self.draft.panel.position, pos.to_string(), *label);
            }
        });

        ui.horizontal(|ui| {
            ui.label("Dimensione icone:");
            ui.add(Slider::new(&mut self.draft.panel.icon_size, 32.0..=72.0).suffix(" px"));
        });

        ui.checkbox(&mut self.draft.panel.autohide, "Nascondi automaticamente");
    }

    // ── Sezione 2: Applicazioni ────────────────────────────────────────────

    fn show_apps(&mut self, ui: &mut egui::Ui) {
        ui.heading("Applicazioni");
        ui.separator();

        // Lista app esistenti
        let mut to_remove: Option<usize> = None;
        let mut to_move_up: Option<usize> = None;

        for (i, app) in self.draft.apps.iter_mut().enumerate() {
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut app.icon_emoji)
                            .desired_width(32.0)
                            .hint_text("🖥"),
                    );
                    ui.add(
                        egui::TextEdit::singleline(&mut app.name)
                            .desired_width(120.0)
                            .hint_text("Nome"),
                    );
                    ui.add(
                        egui::TextEdit::singleline(&mut app.exec)
                            .desired_width(160.0)
                            .hint_text("Comando"),
                    );
                    ui.checkbox(&mut app.pinned, "Pin");

                    if i > 0 && ui.small_button("▲").clicked() {
                        to_move_up = Some(i);
                    }
                    if ui.small_button("🗑").on_hover_text("Rimuovi").clicked() {
                        to_remove = Some(i);
                    }
                });
            });
        }

        if let Some(i) = to_remove  { self.draft.apps.remove(i); }
        if let Some(i) = to_move_up { self.draft.apps.swap(i, i - 1); }

        ui.add_space(8.0);
        ui.separator();
        ui.label(RichText::new("Aggiungi app").strong());
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.new_app.icon_emoji)
                    .desired_width(32.0)
                    .hint_text("🖥"),
            );
            ui.add(
                egui::TextEdit::singleline(&mut self.new_app.name)
                    .desired_width(120.0)
                    .hint_text("Nome"),
            );
            ui.add(
                egui::TextEdit::singleline(&mut self.new_app.exec)
                    .desired_width(160.0)
                    .hint_text("Comando es. gimp"),
            );
            if ui.button("Aggiungi").clicked() && !self.new_app.name.is_empty() {
                self.draft.apps.push(self.new_app.clone());
                self.new_app = default_new_app();
            }
        });
    }

    // ── Sezione 3: AI & Budget ─────────────────────────────────────────────

    fn show_ai_budget(&mut self, ui: &mut egui::Ui) {
        ui.heading("AI & Budget");
        ui.separator();

        // API Key
        ui.label(RichText::new("Anthropic API Key").strong());
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.daemon.api_key)
                    .password(true)
                    .desired_width(320.0)
                    .hint_text("sk-ant-…"),
            );
        });
        ui.label(
            RichText::new("Ottieni la chiave su console.anthropic.com")
                .small()
                .color(Color32::GRAY),
        );

        ui.add_space(10.0);

        // Budget
        ui.label(RichText::new("Budget").strong());
        ui.horizontal(|ui| {
            ui.label("Limite mensile ($):");
            ui.add(
                egui::DragValue::new(&mut self.daemon.budget.monthly_limit_usd)
                    .speed(0.5)
                    .range(1.0..=500.0)
                    .suffix(" $"),
            );
        });
        ui.horizontal(|ui| {
            ui.label("Limite giornaliero soft ($):");
            ui.add(
                egui::DragValue::new(&mut self.daemon.budget.daily_soft_limit_usd)
                    .speed(0.05)
                    .range(0.1..=50.0)
                    .suffix(" $"),
            );
        });
        ui.horizontal(|ui| {
            ui.label("Avvisa al (%):");
            ui.add(
                egui::DragValue::new(&mut self.daemon.budget.alert_at_percent)
                    .speed(1)
                    .range(10..=99)
                    .suffix(" %"),
            );
        });

        ui.add_space(10.0);

        // WordPress
        ui.collapsing("WordPress", |ui| {
            let wp = self.daemon.wordpress.get_or_insert_with(WpCfg::default);

            ui.horizontal(|ui| {
                ui.label("URL sito:");
                ui.add(
                    egui::TextEdit::singleline(&mut wp.url)
                        .desired_width(260.0)
                        .hint_text("https://tuoblog.com"),
                );
            });
            ui.horizontal(|ui| {
                ui.label("Username:");
                ui.add(
                    egui::TextEdit::singleline(&mut wp.username)
                        .desired_width(160.0),
                );
            });
            ui.horizontal(|ui| {
                ui.label("App Password:");
                ui.add(
                    egui::TextEdit::singleline(&mut wp.app_password)
                        .password(true)
                        .desired_width(200.0)
                        .hint_text("xxxx xxxx xxxx xxxx xxxx xxxx"),
                );
            });
            ui.label(
                RichText::new("Genera la App Password da: WordPress → Utenti → Modifica profilo → Password applicazione")
                    .small()
                    .color(Color32::GRAY),
            );
        });

        ui.add_space(4.0);

        // Ghost
        ui.collapsing("Ghost", |ui| {
            let ghost = self.daemon.ghost.get_or_insert_with(GhostCfg::default);

            ui.horizontal(|ui| {
                ui.label("URL sito:");
                ui.add(
                    egui::TextEdit::singleline(&mut ghost.url)
                        .desired_width(260.0)
                        .hint_text("https://tuoblog.ghost.io"),
                );
            });
            ui.horizontal(|ui| {
                ui.label("Admin API Key:");
                ui.add(
                    egui::TextEdit::singleline(&mut ghost.admin_key)
                        .password(true)
                        .desired_width(280.0)
                        .hint_text("id:hex_secret"),
                );
            });
            ui.label(
                RichText::new("Genera la Admin API Key da: Ghost Admin → Impostazioni → Integrazioni → Custom Integration")
                    .small()
                    .color(Color32::GRAY),
            );
        });
    }

    // ── Sezione 4: Aggiornamenti ───────────────────────────────────────────

    fn show_updates(&mut self, ui: &mut egui::Ui) {
        ui.heading("Aggiornamenti");
        ui.separator();

        // Versione corrente
        let installed = std::fs::read_to_string("/usr/local/share/ai-os/VERSION")
            .unwrap_or_else(|_| "N/D".to_string());
        let installed = installed.trim();

        ui.horizontal(|ui| {
            ui.label("Versione installata:");
            ui.label(RichText::new(installed).strong().color(Color32::from_rgb(126, 200, 227)));
        });

        ui.add_space(8.0);

        // Endpoint aggiornamenti
        let upd = self.daemon.update.get_or_insert_with(UpdateCfg::default);

        ui.label(RichText::new("Sorgente aggiornamenti").strong());
        ui.horizontal(|ui| {
            ui.label("URL:");
            ui.add(
                egui::TextEdit::singleline(&mut upd.update_url)
                    .desired_width(280.0)
                    .hint_text("https://tuo-server.com/ai-os/"),
            );
        });
        ui.label(
            RichText::new("L'endpoint deve esporre: version.txt · ai-os-overlay.tar.gz · ai-os-overlay.tar.gz.sha256")
                .small()
                .color(Color32::GRAY),
        );

        ui.add_space(4.0);
        ui.checkbox(&mut upd.auto_check, "Controlla aggiornamenti automaticamente (ogni domenica alle 03:00)");

        ui.add_space(10.0);
        ui.separator();

        // Pulsanti azione
        ui.horizontal(|ui| {
            if ui.button("🔍  Controlla aggiornamenti").clicked() {
                self.update_output = Some(run_update_cmd("--check"));
            }
            if ui.button("⬇  Aggiorna AI-OS").clicked() {
                self.update_output = Some(run_update_cmd("--apply"));
            }
            if ui.button("📦  Aggiorna pacchetti di sistema").clicked() {
                self.update_output = Some(run_update_cmd("--apply-packages"));
            }
        });

        // Output dell'ultimo controllo
        if let Some(ref out) = self.update_output {
            ui.add_space(8.0);
            ui.separator();
            ui.label(RichText::new("Risultato:").strong());

            // Prova a fare il pretty-print del JSON
            let display = serde_json::from_str::<serde_json::Value>(out)
                .map(|v| {
                    let status  = v["status"].as_str().unwrap_or("").to_string();
                    let message = v["message"].as_str().unwrap_or("").to_string();
                    let cur     = v["current_version"].as_str().unwrap_or("").to_string();
                    let avail   = v["available_version"].as_str().unwrap_or("").to_string();
                    let mut s = format!("[{status}] {message}");
                    if !cur.is_empty() && !avail.is_empty() && cur != avail {
                        s.push_str(&format!("\n  Corrente: {cur}  →  Disponibile: {avail}"));
                    }
                    s
                })
                .unwrap_or_else(|_| out.clone());

            let color = if display.contains("error") {
                Color32::from_rgb(255, 100, 100)
            } else if display.contains("update_available") {
                Color32::from_rgb(255, 200, 60)
            } else {
                Color32::from_rgb(100, 220, 100)
            };

            egui::ScrollArea::vertical()
                .max_height(100.0)
                .show(ui, |ui| {
                    ui.label(RichText::new(&display).color(color).small());
                });
        }
    }
}

fn run_update_cmd(flag: &str) -> String {
    let result = std::process::Command::new("ai-os-update")
        .arg(flag)
        .output()
        .or_else(|_| std::process::Command::new("/usr/local/bin/ai-os-update").arg(flag).output());

    match result {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout).to_string();
            let stderr = String::from_utf8_lossy(&out.stderr).to_string();
            if stdout.trim().is_empty() { stderr } else { stdout }
        }
        Err(e) => format!("{{\"status\":\"error\",\"message\":\"ai-os-update non trovato: {e}\",\"current_version\":\"\",\"available_version\":\"\"}}"),
    }
}

fn default_new_app() -> AppEntry {
    AppEntry {
        name:       String::new(),
        icon:       "app".into(),
        icon_emoji: "🖥".into(),
        exec:       String::new(),
        pinned:     true,
    }
}
