/// Percorso Unix socket per IPC tra ai-shell e ai-daemon
pub const SOCKET_PATH: &str = "/tmp/ai-os.sock";

/// Messaggi sul socket (framing: 4 byte len + JSON)
use serde::{Deserialize, Serialize};
use crate::types::{AiRequest, AiResponse, BudgetStatus};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "msg", rename_all = "snake_case")]
pub enum ClientMsg {
    Ask(AiRequest),
    GetBudget,
    Ping,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "msg", rename_all = "snake_case")]
pub enum ServerMsg {
    Response(AiResponse),
    Budget(BudgetStatus),
    Error { reason: String },
    Pong,
}
