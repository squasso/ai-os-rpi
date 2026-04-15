use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub appearance: AppearanceConfig,
    pub panel:      PanelConfig,
    pub apps:       Vec<AppEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppearanceConfig {
    /// "dark" | "light" | "auto"
    pub mode:          String,
    pub accent_color:  String,
    pub wallpaper:     Option<String>,
    pub icon_theme:    String,
    pub font_size:     f32,
    pub scaling:       f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PanelConfig {
    /// "left" | "right" | "top" | "bottom"
    pub position:  String,
    pub icon_size: f32,
    pub autohide:  bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppEntry {
    pub name:       String,
    pub icon:       String,       // chiave icona (es. "chromium")
    pub icon_emoji: String,       // emoji visualizzata — modificabile dall'utente
    pub exec:       String,
    pub pinned:     bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            appearance: AppearanceConfig {
                mode:         "dark".to_string(),
                accent_color: "#7EC8E3".to_string(),
                wallpaper:    None,
                icon_theme:   "default".to_string(),
                font_size:    14.0,
                scaling:      1.0,
            },
            panel: PanelConfig {
                position:  "left".to_string(),
                icon_size: 48.0,
                autohide:  false,
            },
            apps: default_apps(),
        }
    }
}

fn default_apps() -> Vec<AppEntry> {
    vec![
        AppEntry { name: "Chromium".into(),    icon: "chromium".into(),           icon_emoji: "🌐".into(), exec: "chromium-browser".into(),  pinned: true },
        AppEntry { name: "Writer".into(),      icon: "libreoffice-writer".into(), icon_emoji: "📝".into(), exec: "libreoffice --writer".into(), pinned: true },
        AppEntry { name: "GIMP".into(),        icon: "gimp".into(),               icon_emoji: "🎨".into(), exec: "gimp".into(),               pinned: true },
        AppEntry { name: "File Manager".into(),icon: "file-manager".into(),       icon_emoji: "📁".into(), exec: "thunar".into(),             pinned: true },
        AppEntry { name: "Terminale".into(),   icon: "terminal".into(),           icon_emoji: "⬛".into(), exec: "foot".into(),               pinned: true },
        AppEntry { name: "Impostazioni".into(),icon: "settings".into(),           icon_emoji: "⚙️".into(), exec: "__settings__".into(),       pinned: true },
    ]
}

impl AppConfig {
    pub fn load() -> Self {
        let path = config_path();
        let text = std::fs::read_to_string(&path).unwrap_or_default();
        toml::from_str(&text).unwrap_or_default()
    }

    pub fn save(&self) {
        let path = config_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(text) = toml::to_string_pretty(self) {
            let _ = std::fs::write(path, text);
        }
    }
}

fn config_path() -> PathBuf {
    dirs_next::config_dir()
        .unwrap_or_default()
        .join("ai-os")
        .join("appearance.toml")
}
