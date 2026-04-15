use anyhow::{bail, Context, Result};
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::{Article, Publisher};

pub struct GhostPublisher {
    base_url:  String,
    key_id:    String,          // parte prima dei ':' nella Admin API key
    secret:    Vec<u8>,         // parte dopo i ':', decodificata da hex
    http:      Client,
}

impl GhostPublisher {
    /// `admin_key` deve avere il formato `{id}:{hex_secret}` fornito
    /// da Ghost Admin → Integrazioni → Custom Integration → Admin API Key.
    pub fn new(base_url: &str, admin_key: &str) -> Result<Self> {
        let (id, hex_secret) = admin_key
            .split_once(':')
            .context("Ghost admin_key deve avere il formato 'id:hex_secret'")?;

        let secret = hex::decode(hex_secret)
            .context("Ghost admin_key: la parte secret non è hex valido")?;

        Ok(Self {
            base_url:  base_url.trim_end_matches('/').to_string(),
            key_id:    id.to_string(),
            secret,
            http:      Client::new(),
        })
    }

    /// Genera un JWT valido per 5 minuti, firmato con la chiave admin.
    fn jwt_token(&self) -> Result<String> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_secs();

        #[derive(Serialize)]
        struct Claims {
            iat: u64,
            exp: u64,
            aud: &'static str,
        }

        let claims = Claims {
            iat: now,
            exp: now + 300,
            aud: "/admin/",
        };

        let mut header = Header::new(Algorithm::HS256);
        header.kid = Some(self.key_id.clone());

        let token = encode(
            &header,
            &claims,
            &EncodingKey::from_secret(&self.secret),
        )?;

        Ok(token)
    }

    async fn upsert(&self, article: &Article, status: &str) -> Result<String> {
        let token = self.jwt_token()?;
        let url   = format!("{}/ghost/api/admin/posts/", self.base_url);

        let body = json!({
            "posts": [{
                "title":     article.title,
                "mobiledoc": markdown_to_mobiledoc(&article.content),
                "status":    status,
                "tags":      article.tags.iter().map(|t| json!({"name": t})).collect::<Vec<_>>(),
            }]
        });

        let resp = self.http
            .post(&url)
            .header("Authorization",  format!("Ghost {token}"))
            .header("Content-Type",   "application/json")
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            bail!("Ghost API error {}: {}", resp.status(), resp.text().await?);
        }

        let data: serde_json::Value = resp.json().await?;
        let post_url = data["posts"][0]["url"]
            .as_str()
            .unwrap_or("")
            .to_string();
        Ok(post_url)
    }
}

impl Publisher for GhostPublisher {
    async fn publish(&self, article: &Article) -> Result<String> {
        self.upsert(article, "published").await
    }

    async fn save_draft(&self, article: &Article) -> Result<String> {
        self.upsert(article, "draft").await
    }

    async fn list_drafts(&self) -> Result<Vec<String>> {
        let token = self.jwt_token()?;
        let url   = format!(
            "{}/ghost/api/admin/posts/?filter=status:draft&fields=title",
            self.base_url
        );

        let resp = self.http
            .get(&url)
            .header("Authorization", format!("Ghost {token}"))
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;

        let titles = resp["posts"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|p| p["title"].as_str().map(String::from))
            .collect();

        Ok(titles)
    }
}

/// Converte markdown in MobileDoc minimal (card markdown).
fn markdown_to_mobiledoc(md: &str) -> String {
    serde_json::json!({
        "version": "0.3.1",
        "markups": [],
        "atoms":   [],
        "cards":   [["markdown", {"markdown": md}]],
        "sections": [[10, 0]]
    })
    .to_string()
}
