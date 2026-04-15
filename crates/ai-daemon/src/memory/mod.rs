mod history;
mod profile;
mod session;

pub use history::History;
pub use profile::UserProfile;
pub use session::Session;

use anyhow::Result;
use rusqlite::Connection;
use std::path::PathBuf;

pub struct Memory {
    conn:    Connection,
    session: Session,
    profile: UserProfile,
}

impl Memory {
    pub fn open() -> Result<Self> {
        let path = db_path();
        std::fs::create_dir_all(path.parent().unwrap())?;
        let conn = Connection::open(&path)?;
        History::init(&conn)?;
        UserProfile::init(&conn)?;

        let profile = UserProfile::load(&conn)?;
        Ok(Self {
            session: Session::new(),
            profile,
            conn,
        })
    }

    /// Costruisce un contesto rilevante da passare al system prompt
    pub fn relevant_context(&self, _prompt: &str) -> Option<String> {
        let mut parts = vec![];

        if !self.profile.writing_style.is_empty() {
            parts.push(format!("Stile di scrittura: {}", self.profile.writing_style));
        }
        if !self.profile.blog_platform.is_empty() {
            parts.push(format!("Piattaforma blog: {}", self.profile.blog_platform));
        }
        if !self.session.recent_actions.is_empty() {
            let recent = self.session.recent_actions.join(", ");
            parts.push(format!("Azioni recenti in sessione: {recent}"));
        }

        if parts.is_empty() { None } else { Some(parts.join("\n")) }
    }

    pub fn record_interaction(&mut self, prompt: &str, response: &str) {
        self.session.add(prompt);
        let _ = History::insert(&self.conn, prompt, response);
    }
}

fn db_path() -> PathBuf {
    dirs_next::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("/var/lib"))
        .join("ai-os")
        .join("memory.db")
}
