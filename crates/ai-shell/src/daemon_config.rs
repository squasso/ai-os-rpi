/// Legge e scrive ~/.config/ai-os/config.toml (la config del daemon).
/// Usato dalla settings UI per esporre API key, budget e blog config.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DaemonConfig {
    #[serde(default)]
    pub api_key: String,

    #[serde(default)]
    pub budget: BudgetCfg,

    pub wordpress: Option<WpCfg>,
    pub ghost:     Option<GhostCfg>,
    pub update:    Option<UpdateCfg>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetCfg {
    pub monthly_limit_usd:    f64,
    pub daily_soft_limit_usd: f64,
    pub alert_at_percent:     u8,
}

impl Default for BudgetCfg {
    fn default() -> Self {
        Self {
            monthly_limit_usd:    10.0,
            daily_soft_limit_usd: 0.50,
            alert_at_percent:     80,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WpCfg {
    pub url:          String,
    pub username:     String,
    pub app_password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GhostCfg {
    pub url:       String,
    pub admin_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateCfg {
    #[serde(default)]
    pub update_url: String,
    #[serde(default = "default_true")]
    pub auto_check: bool,
}

impl Default for UpdateCfg {
    fn default() -> Self {
        Self { update_url: String::new(), auto_check: true }
    }
}

fn default_true() -> bool { true }

impl DaemonConfig {
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

fn config_path() -> std::path::PathBuf {
    dirs_next::config_dir()
        .unwrap_or_default()
        .join("ai-os")
        .join("config.toml")
}
