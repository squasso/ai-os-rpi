mod budget_manager;
mod claude_client;
mod memory;
mod task_router;
mod tool_engine;

use anyhow::Result;
use std::sync::Arc;
use tokio::net::UnixListener;
use tokio::sync::Mutex;
use tracing::info;

use common::ipc::{ClientMsg, ServerMsg, SOCKET_PATH};
use budget_manager::BudgetManager;
use claude_client::ClaudeClient;
use memory::Memory;
use task_router::TaskRouter;
use tool_engine::{ToolEngine, WpConfig, GhostConfig};

pub struct Daemon {
    budget:  Arc<Mutex<BudgetManager>>,
    memory:  Arc<Mutex<Memory>>,
    claude:  Arc<ClaudeClient>,
    router:  TaskRouter,
    tools:   Arc<ToolEngine>,
}

impl Daemon {
    pub async fn new() -> Result<Self> {
        let config = Config::load()?;
        let budget = Arc::new(Mutex::new(BudgetManager::new(&config.budget)?));
        let memory = Arc::new(Mutex::new(Memory::open()?));
        let claude = Arc::new(ClaudeClient::new(&config.api_key));
        let router = TaskRouter::new();
        let wp_config = config.wordpress.as_ref()
            .filter(|w| !w.url.is_empty())
            .map(|w| WpConfig {
                url:          w.url.clone(),
                username:     w.username.clone(),
                app_password: w.app_password.clone(),
            });
        let ghost_config = config.ghost.as_ref()
            .filter(|g| !g.url.is_empty() && !g.admin_key.is_empty())
            .map(|g| GhostConfig {
                url:       g.url.clone(),
                admin_key: g.admin_key.clone(),
            });
        let tools = Arc::new(ToolEngine::new(wp_config, ghost_config));

        Ok(Self { budget, memory, claude, router, tools })
    }

    pub async fn run(self) -> Result<()> {
        // rimuovi socket precedente se esiste
        let _ = std::fs::remove_file(SOCKET_PATH);
        let listener = UnixListener::bind(SOCKET_PATH)?;
        info!("ai-daemon in ascolto su {}", SOCKET_PATH);

        let daemon = Arc::new(self);

        loop {
            let (stream, _) = listener.accept().await?;
            let d = daemon.clone();
            tokio::spawn(async move {
                if let Err(e) = d.handle_connection(stream).await {
                    tracing::error!("errore connessione: {e}");
                }
            });
        }
    }

    async fn handle_connection(
        &self,
        mut stream: tokio::net::UnixStream,
    ) -> Result<()> {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        loop {
            // leggi 4 byte lunghezza
            let mut len_buf = [0u8; 4];
            if stream.read_exact(&mut len_buf).await.is_err() {
                break;
            }
            let len = u32::from_be_bytes(len_buf) as usize;

            // leggi payload JSON
            let mut buf = vec![0u8; len];
            stream.read_exact(&mut buf).await?;

            let msg: ClientMsg = serde_json::from_slice(&buf)?;
            let reply = self.dispatch(msg).await;

            let payload = serde_json::to_vec(&reply)?;
            let plen = (payload.len() as u32).to_be_bytes();
            stream.write_all(&plen).await?;
            stream.write_all(&payload).await?;
        }
        Ok(())
    }

    async fn dispatch(&self, msg: ClientMsg) -> ServerMsg {
        match msg {
            ClientMsg::Ping => ServerMsg::Pong,

            ClientMsg::GetBudget => {
                let b = self.budget.lock().await;
                ServerMsg::Budget(b.status())
            }

            ClientMsg::Ask(req) => {
                // 1. controlla budget
                {
                    let b = self.budget.lock().await;
                    if b.is_blocked() {
                        return ServerMsg::Error {
                            reason: "Budget mensile esaurito. Aggiorna il limite in Impostazioni.".into(),
                        };
                    }
                }

                // 2. scegli modello
                let model = self.router.classify(&req.prompt).await;

                // 3. recupera contesto memoria
                let ctx = {
                    let m = self.memory.lock().await;
                    m.relevant_context(&req.prompt)
                };

                // 4. chiama Claude con tool use
                match self.claude.ask(&req, &model, ctx.as_deref(), &self.tools).await {
                    Err(e) => ServerMsg::Error { reason: e.to_string() },
                    Ok(resp) => {
                        // 5. aggiorna budget
                        {
                            let mut b = self.budget.lock().await;
                            b.record(resp.tokens_in, resp.tokens_out, &model);
                        }
                        // 6. salva in memoria
                        {
                            let mut m = self.memory.lock().await;
                            m.record_interaction(&req.prompt, &resp.text);
                        }
                        ServerMsg::Response(resp)
                    }
                }
            }
        }
    }
}

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(serde::Deserialize)]
struct Config {
    api_key:   String,
    budget:    BudgetConfig,
    wordpress: Option<WordPressConfig>,
    ghost:     Option<GhostCfg>,
    update:    Option<UpdateCfg>,
}

#[derive(serde::Deserialize)]
pub struct WordPressConfig {
    pub url:          String,
    pub username:     String,
    pub app_password: String,
}

#[derive(serde::Deserialize)]
pub struct GhostCfg {
    pub url:       String,
    pub admin_key: String,   // formato: {id}:{hex_secret}
}

#[derive(serde::Deserialize)]
pub struct UpdateCfg {
    /// URL base da cui scaricare aggiornamenti.
    /// Deve esporre: version.txt, ai-os-overlay.tar.gz, ai-os-overlay.tar.gz.sha256
    pub update_url:  Option<String>,
    /// Controlla aggiornamenti al boot automaticamente (default: true)
    #[serde(default = "default_true")]
    pub auto_check:  bool,
}

fn default_true() -> bool { true }

#[derive(serde::Deserialize)]
pub struct BudgetConfig {
    pub monthly_limit_usd: f64,
    pub daily_soft_limit_usd: f64,
    pub alert_at_percent: u8,
}

impl Config {
    fn load() -> Result<Self> {
        // Usa sempre ~/.config su tutte le piattaforme per semplicità
        let path = dirs_next::home_dir()
            .unwrap_or_default()
            .join(".config")
            .join("ai-os")
            .join("config.toml");

        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|_| DEFAULT_CONFIG.to_string());

        Ok(toml::from_str(&text)?)
    }
}

const DEFAULT_CONFIG: &str = r#"
api_key = ""   # chiave Anthropic: https://console.anthropic.com

[budget]
monthly_limit_usd    = 10.0
daily_soft_limit_usd = 0.50
alert_at_percent     = 80

[update]
update_url = ""     # URL base degli aggiornamenti, es. https://tuo-server.com/ai-os/
auto_check = true   # controlla aggiornamenti ogni domenica alle 03:00

# Decommenta e compila per usare WordPress
# [wordpress]
# url          = ""   # es. https://tuoblog.com
# username     = ""
# app_password = ""

# Decommenta e compila per usare Ghost
# [ghost]
# url          = ""   # es. https://tuoblog.ghost.io
# admin_key    = ""   # Admin API Key dal pannello Ghost (formato id:hex_secret)
"#;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    info!("AI-OS daemon v{VERSION} avviato");
    let daemon = Daemon::new().await?;
    daemon.run().await
}
