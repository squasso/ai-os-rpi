mod ghost;
mod wordpress;

use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Article {
    pub title:   String,
    pub content: String,   // markdown
    pub tags:    Vec<String>,
    pub status:  ArticleStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ArticleStatus {
    Draft,
    Published,
}

pub trait Publisher: Send + Sync {
    async fn publish(&self, article: &Article) -> Result<String>;  // ritorna URL
    async fn save_draft(&self, article: &Article) -> Result<String>;
    async fn list_drafts(&self) -> Result<Vec<String>>;
}

pub use ghost::GhostPublisher;
pub use wordpress::WordPressPublisher;

/// Costruisce un GhostPublisher gestendo l'errore di parsing della chiave.
pub fn new_ghost(base_url: &str, admin_key: &str) -> anyhow::Result<GhostPublisher> {
    GhostPublisher::new(base_url, admin_key)
}
