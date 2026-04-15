use anyhow::{bail, Result};
use blog_publisher::{Article, ArticleStatus, Publisher, WordPressPublisher, new_ghost};
use serde_json::{json, Value};
use std::process::Command;
use tracing::info;

pub struct ToolEngine {
    pub wp_config:    Option<WpConfig>,
    pub ghost_config: Option<GhostConfig>,
}

#[derive(Clone)]
pub struct WpConfig {
    pub url:          String,
    pub username:     String,
    pub app_password: String,
}

#[derive(Clone)]
pub struct GhostConfig {
    pub url:       String,
    pub admin_key: String,   // formato: {id}:{hex_secret}
}

impl ToolEngine {
    pub fn new(wp_config: Option<WpConfig>, ghost_config: Option<GhostConfig>) -> Self {
        Self { wp_config, ghost_config }
    }

    /// Esegui un tool per nome con i parametri JSON ricevuti da Claude
    pub fn execute(&self, name: &str, input: &Value) -> Result<Value> {
        info!("tool: {} — input: {}", name, input);
        match name {
            "launch_app"         => self.launch_app(input),
            "open_url"           => self.open_url(input),
            "write_file"         => self.write_file(input),
            "read_file"          => self.read_file(input),
            "run_shell"          => self.run_shell(input),
            "notify"             => self.notify(input),
            "publish_blog"       => self.publish_blog(input),
            "list_drafts"        => self.list_drafts(input),
            "update_system"      => self.update_system(input),
            _                    => bail!("Tool sconosciuto: {name}"),
        }
    }

    fn launch_app(&self, input: &Value) -> Result<Value> {
        let exec = input["exec"].as_str().ok_or_else(|| anyhow::anyhow!("exec mancante"))?;
        let args: Vec<&str> = input["args"]
            .as_array()
            .map(|a| a.iter().filter_map(|v| v.as_str()).collect())
            .unwrap_or_default();

        Command::new(exec).args(&args).spawn()?;
        Ok(json!({ "status": "ok", "message": format!("Avviato: {exec}") }))
    }

    fn open_url(&self, input: &Value) -> Result<Value> {
        let url = input["url"].as_str().ok_or_else(|| anyhow::anyhow!("url mancante"))?;
        // Su Linux: chromium-browser, su macOS: open
        #[cfg(target_os = "macos")]
        Command::new("open").arg(url).spawn()?;
        #[cfg(not(target_os = "macos"))]
        Command::new("chromium-browser").arg(url).spawn()?;
        Ok(json!({ "status": "ok", "message": format!("Aperto: {url}") }))
    }

    fn write_file(&self, input: &Value) -> Result<Value> {
        let path    = input["path"].as_str().ok_or_else(|| anyhow::anyhow!("path mancante"))?;
        let content = input["content"].as_str().unwrap_or("");
        // espandi ~ nel percorso
        let path = shellexpand::tilde(path).to_string();
        if let Some(parent) = std::path::Path::new(&path).parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, content)?;
        Ok(json!({ "status": "ok", "path": path, "bytes": content.len() }))
    }

    fn read_file(&self, input: &Value) -> Result<Value> {
        let path = input["path"].as_str().ok_or_else(|| anyhow::anyhow!("path mancante"))?;
        let path = shellexpand::tilde(path).to_string();
        let content = std::fs::read_to_string(&path)?;
        Ok(json!({ "status": "ok", "content": content }))
    }

    fn run_shell(&self, input: &Value) -> Result<Value> {
        let cmd = input["command"].as_str().ok_or_else(|| anyhow::anyhow!("command mancante"))?;
        let out = Command::new("sh").arg("-c").arg(cmd).output()?;
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let stderr = String::from_utf8_lossy(&out.stderr).to_string();
        Ok(json!({
            "status":   if out.status.success() { "ok" } else { "error" },
            "stdout":   stdout,
            "stderr":   stderr,
            "exit_code": out.status.code().unwrap_or(-1),
        }))
    }

    fn notify(&self, input: &Value) -> Result<Value> {
        let title = input["title"].as_str().unwrap_or("AI-OS");
        let body  = input["body"].as_str().unwrap_or("");
        // notify-send su Linux, osascript su macOS
        #[cfg(target_os = "macos")]
        Command::new("osascript")
            .arg("-e")
            .arg(format!("display notification \"{body}\" with title \"{title}\""))
            .spawn()?;
        #[cfg(not(target_os = "macos"))]
        Command::new("notify-send").arg(title).arg(body).spawn()?;
        Ok(json!({ "status": "ok" }))
    }

    fn publish_blog(&self, input: &Value) -> Result<Value> {
        let title   = input["title"].as_str().unwrap_or("Senza titolo").to_string();
        let content = input["content"].as_str().unwrap_or("").to_string();
        let status  = input["status"].as_str().unwrap_or("draft");
        let platform = input["platform"].as_str().unwrap_or("wordpress");
        let tags: Vec<String> = input["tags"]
            .as_array()
            .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();

        let article = Article {
            title:   title.clone(),
            content,
            tags,
            status: if status == "publish" { ArticleStatus::Published } else { ArticleStatus::Draft },
        };

        let url = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                match platform {
                    "ghost" => {
                        let g = self.ghost_config.as_ref()
                            .ok_or_else(|| anyhow::anyhow!("Ghost non configurato in ~/.config/ai-os/config.toml"))?;
                        let pub_ = new_ghost(&g.url, &g.admin_key)?;
                        if status == "publish" { pub_.publish(&article).await } else { pub_.save_draft(&article).await }
                    }
                    _ => {
                        let wp = self.wp_config.as_ref()
                            .ok_or_else(|| anyhow::anyhow!("WordPress non configurato in ~/.config/ai-os/config.toml"))?;
                        let pub_ = WordPressPublisher::new(&wp.url, &wp.username, &wp.app_password);
                        if status == "publish" { pub_.publish(&article).await } else { pub_.save_draft(&article).await }
                    }
                }
            })
        })?;

        Ok(json!({ "status": "ok", "action": status, "platform": platform, "title": title, "url": url }))
    }

    fn list_drafts(&self, input: &Value) -> Result<Value> {
        let platform = input["platform"].as_str().unwrap_or("wordpress");

        let drafts = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                match platform {
                    "ghost" => {
                        let g = self.ghost_config.as_ref()
                            .ok_or_else(|| anyhow::anyhow!("Ghost non configurato"))?;
                        let pub_ = new_ghost(&g.url, &g.admin_key)?;
                        pub_.list_drafts().await
                    }
                    _ => {
                        let wp = self.wp_config.as_ref()
                            .ok_or_else(|| anyhow::anyhow!("WordPress non configurato"))?;
                        let pub_ = WordPressPublisher::new(&wp.url, &wp.username, &wp.app_password);
                        pub_.list_drafts().await
                    }
                }
            })
        })?;

        Ok(json!({ "drafts": drafts, "platform": platform }))
    }

    fn update_system(&self, input: &Value) -> Result<Value> {
        let action = input["action"].as_str().unwrap_or("check");

        let flag = match action {
            "apply"           => "--apply",
            "apply-packages"  => "--apply-packages",
            _                 => "--check",
        };

        let out = Command::new("ai-os-update")
            .arg(flag)
            .output()
            .or_else(|_| {
                // Fallback: cerca lo script nella posizione di installazione
                Command::new("/usr/local/bin/ai-os-update").arg(flag).output()
            })?;

        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        let stderr = String::from_utf8_lossy(&out.stderr).to_string();

        // L'output è JSON — lo passiamo direttamente
        let parsed: serde_json::Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| json!({
                "status": if out.status.success() { "ok" } else { "error" },
                "message": format!("{stdout}{stderr}"),
            }));

        Ok(parsed)
    }

    /// Definizioni dei tool da inviare all'API Claude
    pub fn tool_definitions() -> Vec<serde_json::Value> {
        vec![
            json!({
                "name": "launch_app",
                "description": "Apre un'applicazione sul sistema operativo. Usalo quando l'utente vuole aprire un programma.",
                "input_schema": {
                    "type": "object",
                    "properties": {
                        "exec": { "type": "string", "description": "Comando da eseguire (es. 'chromium-browser', 'libreoffice', 'gimp')" },
                        "args": { "type": "array", "items": { "type": "string" }, "description": "Argomenti opzionali" }
                    },
                    "required": ["exec"]
                }
            }),
            json!({
                "name": "open_url",
                "description": "Apre un URL nel browser Chromium. Usalo quando l'utente vuole visitare un sito web.",
                "input_schema": {
                    "type": "object",
                    "properties": {
                        "url": { "type": "string", "description": "URL da aprire (completo di https://)" }
                    },
                    "required": ["url"]
                }
            }),
            json!({
                "name": "write_file",
                "description": "Scrive o crea un file sul filesystem. Usalo per salvare articoli, note, codice.",
                "input_schema": {
                    "type": "object",
                    "properties": {
                        "path":    { "type": "string",  "description": "Percorso del file (es. ~/Documenti/articolo.md)" },
                        "content": { "type": "string",  "description": "Contenuto del file" }
                    },
                    "required": ["path", "content"]
                }
            }),
            json!({
                "name": "read_file",
                "description": "Legge il contenuto di un file. Usalo per vedere file esistenti prima di modificarli.",
                "input_schema": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "Percorso del file da leggere" }
                    },
                    "required": ["path"]
                }
            }),
            json!({
                "name": "run_shell",
                "description": "Esegue un comando shell. Usalo per operazioni di sistema, informazioni sull'hardware, gestione processi.",
                "input_schema": {
                    "type": "object",
                    "properties": {
                        "command": { "type": "string", "description": "Comando shell da eseguire" }
                    },
                    "required": ["command"]
                }
            }),
            json!({
                "name": "notify",
                "description": "Mostra una notifica di sistema all'utente.",
                "input_schema": {
                    "type": "object",
                    "properties": {
                        "title": { "type": "string", "description": "Titolo della notifica" },
                        "body":  { "type": "string", "description": "Testo della notifica" }
                    },
                    "required": ["title", "body"]
                }
            }),
            json!({
                "name": "publish_blog",
                "description": "Pubblica o salva come bozza un articolo sul blog (WordPress o Ghost). \
                                Usalo quando l'utente vuole pubblicare o salvare un articolo.",
                "input_schema": {
                    "type": "object",
                    "properties": {
                        "title":    { "type": "string", "description": "Titolo dell'articolo" },
                        "content":  { "type": "string", "description": "Contenuto in markdown" },
                        "status":   {
                            "type": "string",
                            "enum": ["draft", "publish"],
                            "description": "'draft' per bozza, 'publish' per pubblicare subito"
                        },
                        "platform": {
                            "type": "string",
                            "enum": ["wordpress", "ghost"],
                            "description": "Piattaforma blog: 'wordpress' (default) o 'ghost'"
                        },
                        "tags": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "Lista di tag per l'articolo"
                        }
                    },
                    "required": ["title", "content", "status"]
                }
            }),
            json!({
                "name": "list_drafts",
                "description": "Elenca le bozze presenti sul blog (WordPress o Ghost).",
                "input_schema": {
                    "type": "object",
                    "properties": {
                        "platform": {
                            "type": "string",
                            "enum": ["wordpress", "ghost"],
                            "description": "Piattaforma blog: 'wordpress' (default) o 'ghost'"
                        }
                    }
                }
            }),
            json!({
                "name": "update_system",
                "description": "Controlla o installa aggiornamenti di AI-OS. \
                                Usalo quando l'utente chiede di aggiornare il sistema, \
                                controllare la versione o installare una nuova versione.",
                "input_schema": {
                    "type": "object",
                    "properties": {
                        "action": {
                            "type": "string",
                            "enum": ["check", "apply", "apply-packages"],
                            "description": "'check' controlla se ci sono aggiornamenti disponibili; \
                                           'apply' scarica e installa i nuovi binari AI-OS; \
                                           'apply-packages' aggiorna i pacchetti di sistema (apt)"
                        }
                    },
                    "required": ["action"]
                }
            }),
        ]
    }
}
