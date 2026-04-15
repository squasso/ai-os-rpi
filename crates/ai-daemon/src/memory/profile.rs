use anyhow::Result;
use rusqlite::Connection;

pub struct UserProfile {
    pub writing_style: String,
    pub blog_platform: String,
}

impl UserProfile {
    pub fn init(conn: &Connection) -> Result<()> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS profile (
                key   TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );",
        )?;
        Ok(())
    }

    pub fn load(conn: &Connection) -> Result<Self> {
        let get = |key: &str| -> String {
            conn.query_row(
                "SELECT value FROM profile WHERE key = ?1",
                rusqlite::params![key],
                |r| r.get(0),
            ).unwrap_or_default()
        };
        Ok(Self {
            writing_style: get("writing_style"),
            blog_platform: get("blog_platform"),
        })
    }

    pub fn set(conn: &Connection, key: &str, value: &str) -> Result<()> {
        conn.execute(
            "INSERT INTO profile (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            rusqlite::params![key, value],
        )?;
        Ok(())
    }
}
