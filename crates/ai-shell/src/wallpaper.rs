use egui::{Context, CentralPanel, Color32};
use crate::config::AppearanceConfig;

pub fn draw(ctx: &Context, cfg: &AppearanceConfig) {
    CentralPanel::default()
        .frame(egui::Frame::none().fill(bg_color(cfg)))
        .show(ctx, |_ui| {
            // Il wallpaper immagine viene gestito dal compositor Wayland (swaybg).
            // Qui gestiamo solo il colore di sfondo fallback quando non c'è wallpaper.
        });
}

fn bg_color(cfg: &AppearanceConfig) -> Color32 {
    match cfg.mode.as_str() {
        "light" => Color32::from_rgb(245, 246, 250),
        _       => Color32::from_rgb(26, 27, 38),
    }
}
