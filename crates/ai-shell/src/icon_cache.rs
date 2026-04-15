use egui::{Color32, ColorImage, Context, TextureHandle, TextureOptions, Vec2};
use std::collections::HashMap;

/// Gestisce le icone del panel.
/// Prima cerca PNG in ~/.config/ai-os/icons/<name>.png
/// Se non trovata usa un'icona lettera colorata (sempre disponibile)
pub struct IconCache {
    textures: HashMap<String, TextureHandle>,
}

impl IconCache {
    pub fn new() -> Self {
        Self { textures: HashMap::new() }
    }

    /// Disegna l'icona per `app_name` centrata in `rect`.
    /// Usa PNG da disco se disponibile, altrimenti icona lettera.
    pub fn draw(
        &mut self,
        ui:       &mut egui::Ui,
        app_name: &str,
        icon_key: &str,
        size:     f32,
    ) -> egui::Response {
        let (rect, response) = ui.allocate_exact_size(
            Vec2::splat(size),
            egui::Sense::click(),
        );

        if ui.is_rect_visible(rect) {
            // Prova a caricare PNG da disco
            if let Some(tex) = self.load_png(ui.ctx(), icon_key) {
                ui.painter().image(
                    tex.id(),
                    rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    Color32::WHITE,
                );
            } else {
                // Fallback: icona lettera colorata
                draw_letter_icon(ui.painter(), rect, app_name, size);
            }
        }

        response
    }

    fn load_png(&mut self, ctx: &Context, key: &str) -> Option<&TextureHandle> {
        if self.textures.contains_key(key) {
            return self.textures.get(key);
        }

        let path = dirs_next::home_dir()?
            .join(".config/ai-os/icons")
            .join(format!("{key}.png"));

        if !path.exists() {
            return None;
        }

        let bytes = std::fs::read(&path).ok()?;
        let img   = image::load_from_memory(&bytes).ok()?.to_rgba8();
        let size  = [img.width() as usize, img.height() as usize];
        let pixels: Vec<Color32> = img
            .pixels()
            .map(|p| Color32::from_rgba_unmultiplied(p[0], p[1], p[2], p[3]))
            .collect();

        let color_img = ColorImage { size, pixels };
        let texture   = ctx.load_texture(key, color_img, TextureOptions::LINEAR);
        self.textures.insert(key.to_string(), texture);
        self.textures.get(key)
    }
}

/// Disegna un'icona lettera: quadrato arrotondato colorato + iniziale in bianco
fn draw_letter_icon(painter: &egui::Painter, rect: egui::Rect, name: &str, size: f32) {
    let bg    = app_color(name);
    let letter = name.chars().next().unwrap_or('?').to_uppercase().next().unwrap_or('?');
    let rounding = size * 0.22;

    painter.rect_filled(rect, rounding, bg);
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        letter.to_string(),
        egui::FontId::proportional(size * 0.48),
        Color32::WHITE,
    );
}

/// Colore deterministico basato sul nome dell'app
fn app_color(name: &str) -> Color32 {
    // Palette di colori Material Design
    const PALETTE: &[Color32] = &[
        Color32::from_rgb(66,  133, 244),  // blu Google
        Color32::from_rgb(52,  168, 83),   // verde
        Color32::from_rgb(251, 188, 4),    // giallo
        Color32::from_rgb(234, 67,  53),   // rosso
        Color32::from_rgb(103, 58,  183),  // viola
        Color32::from_rgb(0,   150, 136),  // teal
        Color32::from_rgb(255, 87,  34),   // arancio
        Color32::from_rgb(33,  150, 243),  // azzurro
    ];

    // Override per app specifiche
    match name.to_lowercase().as_str() {
        n if n.contains("chromium") || n.contains("chrome") || n.contains("browser") =>
            Color32::from_rgb(66, 133, 244),
        n if n.contains("writer") || n.contains("libreoffice") =>
            Color32::from_rgb(0, 100, 200),
        n if n.contains("gimp") =>
            Color32::from_rgb(100, 60, 140),
        n if n.contains("terminal") || n.contains("console") =>
            Color32::from_rgb(40, 40, 40),
        n if n.contains("file") =>
            Color32::from_rgb(255, 160, 0),
        n if n.contains("impostazioni") || n.contains("settings") =>
            Color32::from_rgb(96, 96, 96),
        _ => {
            // Hash deterministico sul nome
            let hash: usize = name.bytes().fold(0usize, |acc, b| acc.wrapping_mul(31).wrapping_add(b as usize));
            PALETTE[hash % PALETTE.len()]
        }
    }
}
