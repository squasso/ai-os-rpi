use anyhow::{bail, Result};
use pulldown_cmark::{html, Options, Parser};
use reqwest::Client;
use serde_json::json;
use crate::{Article, Publisher};

pub struct WordPressPublisher {
    base_url:     String,
    username:     String,
    app_password: String,
    http:         Client,
}

impl WordPressPublisher {
    pub fn new(base_url: &str, username: &str, app_password: &str) -> Self {
        Self {
            base_url:     base_url.trim_end_matches('/').to_string(),
            username:     username.to_string(),
            app_password: app_password.to_string(),
            http:         Client::new(),
        }
    }

    fn auth_header(&self) -> String {
        use base64::Engine;
        let cred = format!("{}:{}", self.username, self.app_password);
        format!("Basic {}", base64::engine::general_purpose::STANDARD.encode(cred))
    }
}

impl Publisher for WordPressPublisher {
    async fn publish(&self, article: &Article) -> Result<String> {
        self.create_post(article, "publish").await
    }

    async fn save_draft(&self, article: &Article) -> Result<String> {
        self.create_post(article, "draft").await
    }

    async fn list_drafts(&self) -> Result<Vec<String>> {
        let url = format!("{}/wp-json/wp/v2/posts?status=draft&per_page=20", self.base_url);
        let resp = self.http
            .get(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;

        let titles = resp.as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|p| p["title"]["rendered"].as_str().map(String::from))
            .collect();
        Ok(titles)
    }
}

impl WordPressPublisher {
    async fn create_post(&self, article: &Article, status: &str) -> Result<String> {
        let url  = format!("{}/wp-json/wp/v2/posts", self.base_url);
        let html = markdown_to_html(&article.content);

        // Recupera gli ID dei tag
        let tag_ids = self.get_or_create_tags(&article.tags).await?;

        let body = json!({
            "title":   article.title,
            "content": html,
            "status":  status,
            "tags":    tag_ids,
            "format":  "standard",
        });

        let resp = self.http
            .post(&url)
            .header("Authorization", self.auth_header())
            .header("Content-Type",  "application/json")
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            bail!("WordPress API error {}: {}", resp.status(), resp.text().await?);
        }

        let data: serde_json::Value = resp.json().await?;
        Ok(data["link"].as_str().unwrap_or("").to_string())
    }

    /// Recupera ID tag esistenti o li crea se non esistono
    async fn get_or_create_tags(&self, tags: &[String]) -> Result<Vec<u64>> {
        let mut ids = vec![];
        for tag in tags {
            let url = format!("{}/wp-json/wp/v2/tags?search={}", self.base_url, tag);
            let resp: serde_json::Value = self.http
                .get(&url)
                .header("Authorization", self.auth_header())
                .send()
                .await?
                .json()
                .await?;

            if let Some(id) = resp[0]["id"].as_u64() {
                ids.push(id);
            } else {
                // crea il tag
                let create_url = format!("{}/wp-json/wp/v2/tags", self.base_url);
                let created: serde_json::Value = self.http
                    .post(&create_url)
                    .header("Authorization", self.auth_header())
                    .json(&json!({ "name": tag }))
                    .send()
                    .await?
                    .json()
                    .await?;
                if let Some(id) = created["id"].as_u64() {
                    ids.push(id);
                }
            }
        }
        Ok(ids)
    }
}

fn markdown_to_html(md: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_FOOTNOTES);

    let parser = Parser::new_ext(md, options);
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);
    html_output
}
