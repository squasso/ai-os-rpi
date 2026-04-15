use egui::{Color32, FontDefinitions, FontFamily, Style, Visuals, Rounding, Stroke};
use crate::config::AppearanceConfig;

pub struct Theme {
    pub bg:      Color32,
    pub surface: Color32,
    pub border:  Color32,
    pub text:    Color32,
    pub muted:   Color32,
    pub accent:  Color32,
    pub success: Color32,
    pub warning: Color32,
    pub error:   Color32,
}

impl Theme {
    pub fn from_config(cfg: &AppearanceConfig) -> Self {
        match cfg.mode.as_str() {
            "light" => Self::light(cfg),
            _       => Self::dark(cfg),
        }
    }

    fn dark(cfg: &AppearanceConfig) -> Self {
        let accent = hex_to_color32(&cfg.accent_color)
            .unwrap_or(Color32::from_rgb(126, 200, 227));
        Self {
            bg:      Color32::from_rgb(26,  27,  38),
            surface: Color32::from_rgb(36,  37,  58),
            border:  Color32::from_rgb(46,  48,  80),
            text:    Color32::from_rgb(232, 233, 240),
            muted:   Color32::from_rgb(123, 127, 158),
            accent,
            success: Color32::from_rgb(107, 203, 139),
            warning: Color32::from_rgb(240, 192,  96),
            error:   Color32::from_rgb(224, 108, 117),
        }
    }

    fn light(cfg: &AppearanceConfig) -> Self {
        let accent = hex_to_color32(&cfg.accent_color)
            .unwrap_or(Color32::from_rgb(0, 120, 180));
        Self {
            bg:      Color32::from_rgb(245, 246, 250),
            surface: Color32::from_rgb(255, 255, 255),
            border:  Color32::from_rgb(210, 212, 230),
            text:    Color32::from_rgb(30,  30,  50),
            muted:   Color32::from_rgb(130, 130, 160),
            accent,
            success: Color32::from_rgb(50,  170, 90),
            warning: Color32::from_rgb(200, 140, 30),
            error:   Color32::from_rgb(190, 60,  60),
        }
    }

    pub fn apply(&self, ctx: &egui::Context) {
        let mut style = Style::default();
        let mut visuals = Visuals::dark();

        visuals.window_fill         = self.surface;
        visuals.panel_fill          = self.bg;
        visuals.faint_bg_color      = self.surface;
        visuals.extreme_bg_color    = self.bg;
        visuals.window_rounding     = Rounding::same(10.0);
        visuals.window_stroke       = Stroke::new(1.0, self.border);
        visuals.widgets.noninteractive.bg_fill   = self.surface;
        visuals.widgets.inactive.bg_fill         = self.surface;
        visuals.widgets.hovered.bg_fill          = self.accent.linear_multiply(0.15);
        visuals.widgets.active.bg_fill           = self.accent.linear_multiply(0.3);
        visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, self.text);
        visuals.override_text_color              = Some(self.text);
        visuals.selection.bg_fill               = self.accent.linear_multiply(0.4);

        style.visuals = visuals;
        style.spacing.item_spacing  = egui::vec2(8.0, 8.0);
        style.spacing.button_padding = egui::vec2(12.0, 6.0);

        ctx.set_style(style);
        ctx.set_fonts(build_fonts());
    }
}

fn build_fonts() -> FontDefinitions {
    // Usa i font di sistema — Inter e JetBrains Mono vanno installati sul device
    FontDefinitions::default()
}

fn hex_to_color32(hex: &str) -> Option<Color32> {
    let hex = hex.trim_start_matches('#');
    if hex.len() != 6 { return None; }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some(Color32::from_rgb(r, g, b))
}
