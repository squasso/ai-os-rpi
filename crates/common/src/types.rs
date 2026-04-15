use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Modello LLM da usare per un task
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Model {
    /// Task veloci e semplici — basso costo
    Haiku,
    /// Scrittura, analisi, task complessi
    Sonnet,
    /// Ragionamento approfondito, piani multi-step, analisi lunghe
    Opus,
}

impl Model {
    pub fn api_id(&self) -> &'static str {
        match self {
            Model::Haiku  => "claude-haiku-4-5-20251001",
            Model::Sonnet => "claude-sonnet-4-6",
            Model::Opus   => "claude-opus-4-6",
        }
    }

    /// Costo stimato per milione di token (input, output)
    pub fn cost_per_mtok(&self) -> (f64, f64) {
        match self {
            Model::Haiku  => (0.80,  4.0),
            Model::Sonnet => (3.0,  15.0),
            Model::Opus   => (15.0, 75.0),
        }
    }

    /// Limite massimo di token in output per questo modello
    /// — calibrato sul tipo di task, non sul massimo teorico
    pub fn max_tokens(&self) -> u32 {
        match self {
            Model::Haiku  => 512,   // risposte brevi: comandi, fatti, status
            Model::Sonnet => 4096,  // articoli, codice, analisi medie
            Model::Opus   => 8192,  // ragionamenti profondi, piani complessi
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Model::Haiku  => "Haiku",
            Model::Sonnet => "Sonnet",
            Model::Opus   => "Opus",
        }
    }
}

/// Categoria di un task — usata dal router per scegliere il modello
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskKind {
    /// Comandi semplici: apri app, info rapide
    Simple,
    /// Ricerche, riassunti, task medi
    Medium,
    /// Scrittura articoli, revisione, analisi complessa
    Writing,
}

/// Richiesta dall'ai-shell al daemon
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiRequest {
    pub id:      Uuid,
    pub prompt:  String,
    pub context: Option<String>,
}

impl AiRequest {
    pub fn new(prompt: impl Into<String>) -> Self {
        Self {
            id:      Uuid::new_v4(),
            prompt:  prompt.into(),
            context: None,
        }
    }
}

/// Risposta dal daemon all'ai-shell
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiResponse {
    pub id:          Uuid,
    pub text:        String,
    pub actions:     Vec<Action>,
    pub model_used:  Model,
    pub tokens_in:   u32,
    pub tokens_out:  u32,
    pub cost_usd:    f64,
    pub timestamp:   DateTime<Utc>,
}

/// Azione che il daemon chiede alla shell di eseguire
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Action {
    LaunchApp    { exec: String, args: Vec<String> },
    OpenFile     { path: String },
    OpenUrl      { url: String },
    Notify       { title: String, body: String },
    ShowBudget,
    OpenSettings,
}

/// Snapshot dello stato budget — inviato periodicamente alla shell
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetStatus {
    pub spent_today_usd:   f64,
    pub spent_month_usd:   f64,
    pub limit_month_usd:   f64,
    pub limit_daily_usd:   f64,
    pub warning:           bool,
    pub blocked:           bool,
}
