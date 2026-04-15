use anyhow::{bail, Result};
use chrono::Utc;
use common::types::{AiRequest, AiResponse, Model};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use crate::tool_engine::ToolEngine;

const API_URL: &str = "https://api.anthropic.com/v1/messages";
const ANTHROPIC_VERSION: &str = "2023-06-01";
const MAX_TOOL_ITERATIONS: u8 = 10;

pub struct ClaudeClient {
    api_key: String,
    http:    Client,
}

impl ClaudeClient {
    pub fn new(api_key: &str) -> Self {
        Self {
            api_key: api_key.to_string(),
            http:    Client::new(),
        }
    }

    pub async fn ask(
        &self,
        req:     &AiRequest,
        model:   &Model,
        ctx:     Option<&str>,
        tools:   &ToolEngine,
    ) -> Result<AiResponse> {
        let system = build_system_prompt(ctx);

        // Conversazione multi-turno per il tool use
        let mut messages: Vec<Value> = vec![
            json!({ "role": "user", "content": req.prompt })
        ];

        let mut total_in:  u32 = 0;
        let mut total_out: u32 = 0;
        let mut final_text = String::new();
        let mut iterations: u8 = 0;

        loop {
            iterations += 1;
            if iterations > MAX_TOOL_ITERATIONS {
                bail!("Troppi cicli di tool use ({MAX_TOOL_ITERATIONS})");
            }

            let body = json!({
                "model":      model.api_id(),
                "max_tokens": model.max_tokens(),
                "system": [{
                    "type": "text",
                    "text": system,
                    "cache_control": { "type": "ephemeral" }
                }],
                "tools":    ToolEngine::tool_definitions(),
                "messages": messages,
            });

            let resp = self.http
                .post(API_URL)
                .header("x-api-key",        &self.api_key)
                .header("anthropic-version", ANTHROPIC_VERSION)
                .header("anthropic-beta",    "prompt-caching-2024-07-31")
                .header("content-type",      "application/json")
                .json(&body)
                .send()
                .await?;

            if !resp.status().is_success() {
                bail!("Errore API Claude: {}", resp.text().await?);
            }

            let api_resp: ApiResponse = resp.json().await?;
            total_in  += api_resp.usage.input_tokens;
            total_out += api_resp.usage.output_tokens;

            // Aggiungi la risposta dell'assistente alla conversazione
            messages.push(json!({
                "role":    "assistant",
                "content": api_resp.content,
            }));

            match api_resp.stop_reason.as_deref() {
                Some("end_turn") => {
                    // Estrai il testo finale
                    final_text = api_resp.content
                        .iter()
                        .filter(|b| b.get("type").and_then(|t| t.as_str()) == Some("text"))
                        .filter_map(|b| b.get("text").and_then(|t| t.as_str()))
                        .collect::<Vec<_>>()
                        .join("\n");
                    break;
                }

                Some("tool_use") => {
                    // Esegui i tool richiesti e raccogli i risultati
                    let mut tool_results: Vec<Value> = vec![];

                    for block in &api_resp.content {
                        if block.get("type").and_then(|t| t.as_str()) != Some("tool_use") {
                            continue;
                        }
                        let tool_id   = block["id"].as_str().unwrap_or("").to_string();
                        let tool_name = block["name"].as_str().unwrap_or("");
                        let tool_input = &block["input"];

                        let result = match tools.execute(tool_name, tool_input) {
                            Ok(v)  => json!({
                                "type":        "tool_result",
                                "tool_use_id": tool_id,
                                "content":     v.to_string(),
                            }),
                            Err(e) => json!({
                                "type":        "tool_result",
                                "tool_use_id": tool_id,
                                "is_error":    true,
                                "content":     e.to_string(),
                            }),
                        };
                        tool_results.push(result);
                    }

                    // Aggiungi i risultati alla conversazione e continua il loop
                    messages.push(json!({
                        "role":    "user",
                        "content": tool_results,
                    }));
                }

                other => bail!("stop_reason inatteso: {:?}", other),
            }
        }

        let (cin, cout) = model.cost_per_mtok();
        let cost = (total_in  as f64 / 1_000_000.0) * cin
                 + (total_out as f64 / 1_000_000.0) * cout;

        Ok(AiResponse {
            id:         req.id,
            text:       final_text,
            actions:    vec![],
            model_used: model.clone(),
            tokens_in:  total_in,
            tokens_out: total_out,
            cost_usd:   cost,
            timestamp:  Utc::now(),
        })
    }
}

fn build_system_prompt(ctx: Option<&str>) -> String {
    let platform = if cfg!(target_os = "macos") {
        "macOS (sviluppo). Per aprire app usa launch_app con exec='open' e args=['-a','NomeApp']. \
         Per aprire URL usa open_url. Esempi: Safari → exec='open', args=['-a','Safari']; \
         Firefox → exec='open', args=['-a','Firefox']."
    } else {
        "Linux / Raspberry Pi 4. Browser predefinito: chromium-browser."
    };

    let mut s = format!(
        "Sei l'assistente AI integrato in AI-OS. \
         Sistema attuale: {platform} \
         Puoi aprire applicazioni, gestire file, navigare sul web, eseguire comandi shell \
         e scrivere articoli per il blog. \
         Quando l'utente ti chiede di fare qualcosa sul sistema, usa i tool disponibili — \
         non limitarti a descrivere cosa fare, fallo direttamente. \
         Rispondi sempre in italiano. Sii conciso e pratico.",
    );
    if let Some(c) = ctx {
        s.push_str("\n\nCONTESTO UTENTE:\n");
        s.push_str(c);
    }
    s
}

// ── Strutture JSON per l'API ────────────────────────────────────────────────

#[derive(Deserialize)]
struct ApiResponse {
    content:     Vec<Value>,
    stop_reason: Option<String>,
    usage:       Usage,
}

#[derive(Deserialize)]
struct Usage {
    input_tokens:  u32,
    output_tokens: u32,
}
