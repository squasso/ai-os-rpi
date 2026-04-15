use egui::{Context, SidePanel, TopBottomPanel, Vec2};
use std::process::Command;
use crate::config::{AppConfig, AppEntry};

// Emoji disponibili nel picker
const EMOJI_PICKER: &[(&str, &str)] = &[
    ("🌐", "Browser"),    ("📝", "Editor"),     ("🎨", "Grafica"),
    ("📁", "File"),       ("⬛", "Terminale"),   ("⚙️", "Impost."),
    ("📦", "App"),        ("🎵", "Musica"),      ("🎬", "Video"),
    ("📧", "Email"),      ("📅", "Calendario"),  ("🔒", "Sicurezza"),
    ("💻", "Codice"),     ("📊", "Tabelle"),     ("📰", "Notizie"),
    ("🗂️", "Archivio"),  ("🖨️", "Stampa"),     ("📷", "Foto"),
    ("🗒️", "Note"),      ("🔧", "Strumenti"),   ("🌍", "Mappa"),
    ("💬", "Chat"),       ("📡", "Rete"),        ("🎮", "Giochi"),
];

pub struct Panel {
    // Stato del picker: Some((indice app, posizione)) quando aperto
    picker_for: Option<usize>,
}

impl Panel {
    pub fn new(_config: &AppConfig) -> Self {
        Self { picker_for: None }
    }

    pub fn show(&mut self, ctx: &Context, config: &mut AppConfig, settings_open: &mut bool) {
        match config.panel.position.as_str() {
            "right"  => self.show_side(ctx, config, settings_open, false),
            "top"    => self.show_top(ctx, config, settings_open),
            "bottom" => self.show_bottom(ctx, config, settings_open),
            _        => self.show_side(ctx, config, settings_open, true),
        }

        // Finestra picker icona (aperta con tasto destro)
        self.show_picker(ctx, config);
    }

    fn show_side(&mut self, ctx: &Context, config: &mut AppConfig, settings_open: &mut bool, left: bool) {
        let size  = config.panel.icon_size;
        let panel = if left {
            SidePanel::left("panel").exact_width(size + 20.0)
        } else {
            SidePanel::right("panel").exact_width(size + 20.0)
        };
        panel.show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(10.0);
                for i in 0..config.apps.len() {
                    let action = self.icon_button(ui, &config.apps[i], i, size);
                    self.handle_action(action, i, &mut config.apps[i], settings_open);
                    ui.add_space(4.0);
                }
            });
        });
    }

    fn show_top(&mut self, ctx: &Context, config: &mut AppConfig, settings_open: &mut bool) {
        let size = config.panel.icon_size;
        TopBottomPanel::top("panel_top").exact_height(size + 12.0).show(ctx, |ui| {
            ui.horizontal_centered(|ui| {
                ui.add_space(8.0);
                for i in 0..config.apps.len() {
                    let action = self.icon_button(ui, &config.apps[i], i, size);
                    self.handle_action(action, i, &mut config.apps[i], settings_open);
                    ui.add_space(4.0);
                }
            });
        });
    }

    fn show_bottom(&mut self, ctx: &Context, config: &mut AppConfig, settings_open: &mut bool) {
        let size = config.panel.icon_size;
        TopBottomPanel::bottom("panel_bottom").exact_height(size + 12.0).show(ctx, |ui| {
            ui.horizontal_centered(|ui| {
                ui.add_space(8.0);
                for i in 0..config.apps.len() {
                    let action = self.icon_button(ui, &config.apps[i], i, size);
                    self.handle_action(action, i, &mut config.apps[i], settings_open);
                    ui.add_space(4.0);
                }
            });
        });
    }

    fn icon_button(&mut self, ui: &mut egui::Ui, app: &AppEntry, idx: usize, size: f32) -> IconAction {
        let mut action = IconAction::None;

        let btn = egui::Button::new(
            egui::RichText::new(&app.icon_emoji).size(size * 0.55),
        )
        .min_size(Vec2::splat(size))
        .rounding(size * 0.22)
        .frame(false);

        let response = ui.add(btn).on_hover_text(&app.name);

        if response.clicked() {
            action = IconAction::Launch;
        }

        response.context_menu(|ui| {
            ui.label(egui::RichText::new(&app.name).strong());
            ui.separator();
            if ui.button("▶  Apri").clicked() {
                action = IconAction::Launch;
                ui.close_menu();
            }
            if ui.button("🎨  Cambia icona").clicked() {
                action = IconAction::OpenPicker(idx);
                ui.close_menu();
            }
        });

        action
    }

    fn handle_action(&mut self, action: IconAction, idx: usize, app: &mut AppEntry, settings_open: &mut bool) {
        match action {
            IconAction::None => {}
            IconAction::Launch => self.launch(app, settings_open),
            IconAction::OpenPicker(i) => self.picker_for = Some(i),
        }
    }

    fn show_picker(&mut self, ctx: &Context, config: &mut AppConfig) {
        let idx = match self.picker_for {
            Some(i) => i,
            None    => return,
        };
        if idx >= config.apps.len() {
            self.picker_for = None;
            return;
        }

        let app_name = config.apps[idx].name.clone();
        let mut open = true;

        egui::Window::new(format!("Icona — {app_name}"))
            .collapsible(false)
            .resizable(false)
            .open(&mut open)
            .show(ctx, |ui| {
                ui.label("Scegli un'icona:");
                ui.add_space(8.0);

                // Griglia emoji
                egui::Grid::new("emoji_grid")
                    .num_columns(6)
                    .spacing([8.0, 8.0])
                    .show(ui, |ui| {
                        for (col, (emoji, label)) in EMOJI_PICKER.iter().enumerate() {
                            let btn = ui.add(
                                egui::Button::new(
                                    egui::RichText::new(*emoji).size(28.0)
                                )
                                .min_size(Vec2::splat(44.0))
                            ).on_hover_text(*label);

                            if btn.clicked() {
                                config.apps[idx].icon_emoji = emoji.to_string();
                                config.save();
                                self.picker_for = None;
                            }

                            if (col + 1) % 6 == 0 {
                                ui.end_row();
                            }
                        }
                    });

                ui.add_space(8.0);
                ui.separator();

                // Campo testo per emoji personalizzata
                ui.horizontal(|ui| {
                    ui.label("Oppure digita un'emoji:");
                    let mut custom = config.apps[idx].icon_emoji.clone();
                    if ui.text_edit_singleline(&mut custom).changed() {
                        config.apps[idx].icon_emoji = custom;
                    }
                    if ui.button("✓").clicked() {
                        config.save();
                        self.picker_for = None;
                    }
                });
            });

        if !open {
            self.picker_for = None;
        }
    }

    fn launch(&self, app: &AppEntry, settings_open: &mut bool) {
        if app.exec == "__settings__" {
            *settings_open = true;
            return;
        }
        let parts: Vec<&str> = app.exec.split_whitespace().collect();
        if let Some((cmd, args)) = parts.split_first() {
            let _ = Command::new(cmd).args(args).spawn();
        }
    }
}

enum IconAction {
    None,
    Launch,
    OpenPicker(usize),
}
