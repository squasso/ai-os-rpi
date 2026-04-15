use anyhow::Result;
use rusqlite::Connection;

pub struct History;

impl History {
    pub fn init(conn: &Connection) -> Result<()> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS history (
                id        INTEGER PRIMARY KEY,
                prompt    TEXT NOT NULL,
                response  TEXT NOT NULL,
                ts        DATETIME DEFAULT CURRENT_TIMESTAMP
            );",
        )?;
        Ok(())
    }

    pub fn insert(conn: &Connection, prompt: &str, response: &str) -> Result<()> {
        conn.execute(
            "INSERT INTO history (prompt, response) VALUES (?1, ?2)",
            rusqlite::params![prompt, response],
        )?;
        Ok(())
    }
}
