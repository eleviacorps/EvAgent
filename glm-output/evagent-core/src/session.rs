//! SQLite session store with FTS5 for full-text search across past sessions.

use crate::errors::Result;
use crate::models::SessionRow;
use rusqlite::Connection;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Thin async wrapper around a single rusqlite Connection.
/// Writes are serialized via the mutex; reads take a brief lock too.
/// Good enough for EvAgent's modest throughput.
#[derive(Clone)]
pub struct SessionStore {
    conn: Arc<Mutex<Connection>>,
}

impl SessionStore {
    pub fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS sessions (
                id           TEXT PRIMARY KEY,
                domain       TEXT NOT NULL,
                total_tokens INTEGER NOT NULL DEFAULT 0,
                total_cost   REAL NOT NULL DEFAULT 0.0,
                summary      TEXT NOT NULL DEFAULT '',
                created_at   INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS messages (
                id         INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id TEXT NOT NULL,
                role       TEXT NOT NULL,
                content    TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
            );

            CREATE VIRTUAL TABLE IF NOT EXISTS messages_fts USING fts5(
                content,
                content='messages',
                content_rowid='id'
            );

            CREATE TRIGGER IF NOT EXISTS messages_ai AFTER INSERT ON messages BEGIN
                INSERT INTO messages_fts(rowid, content) VALUES (new.id, new.content);
            END;
            CREATE TRIGGER IF NOT EXISTS messages_ad AFTER DELETE ON messages BEGIN
                INSERT INTO messages_fts(messages_fts, rowid, content) VALUES('delete', old.id, old.content);
            END;
            CREATE TRIGGER IF NOT EXISTS messages_au AFTER UPDATE ON messages BEGIN
                INSERT INTO messages_fts(messages_fts, rowid, content) VALUES('delete', old.id, old.content);
                INSERT INTO messages_fts(rowid, content) VALUES (new.id, new.content);
            END;

            CREATE TABLE IF NOT EXISTS memory (
                key       TEXT PRIMARY KEY,
                content   TEXT NOT NULL,
                kind      TEXT NOT NULL,
                updated_at INTEGER NOT NULL
            );
            ",
        )?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub async fn create_session(&self, id: &str, domain: &str) -> Result<()> {
        let conn = self.conn.lock().await;
        let now = chrono::Utc::now().timestamp();
        conn.execute(
            "INSERT INTO sessions (id, domain, created_at) VALUES (?1, ?2, ?3)",
            rusqlite::params![id, domain, now],
        )?;
        Ok(())
    }

    pub async fn append_message(&self, session_id: &str, role: &str, content: &str) -> Result<()> {
        let conn = self.conn.lock().await;
        let now = chrono::Utc::now().timestamp();
        conn.execute(
            "INSERT INTO messages (session_id, role, content, created_at) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![session_id, role, content, now],
        )?;
        Ok(())
    }

    pub async fn finalize_session(
        &self,
        id: &str,
        total_tokens: u64,
        total_cost: f64,
        summary: &str,
    ) -> Result<()> {
        let conn = self.conn.lock().await;
        conn.execute(
            "UPDATE sessions SET total_tokens=?1, total_cost=?2, summary=?3 WHERE id=?4",
            rusqlite::params![total_tokens, total_cost, summary, id],
        )?;
        Ok(())
    }

    pub async fn search_messages(&self, query: &str, limit: u32) -> Result<Vec<SessionRow>> {
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare(
            "SELECT s.id, s.domain, s.total_tokens, s.total_cost, s.summary, s.created_at
             FROM messages_fts f
             JOIN messages m ON m.id = f.rowid
             JOIN sessions s ON s.id = m.session_id
             WHERE messages_fts MATCH ?1
             GROUP BY s.id
             ORDER BY s.created_at DESC
             LIMIT ?2",
        )?;
        let rows = stmt.query_map(rusqlite::params![query, limit], |row| {
            Ok(SessionRow {
                id: row.get(0)?,
                domain: row.get(1)?,
                total_tokens: row.get(2)?,
                total_cost: row.get(3)?,
                summary: row.get(4)?,
                created_at: row.get(5)?,
            })
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    pub async fn memory_write(&self, key: &str, content: &str, kind: &str) -> Result<()> {
        let conn = self.conn.lock().await;
        let now = chrono::Utc::now().timestamp();
        conn.execute(
            "INSERT INTO memory (key, content, kind, updated_at) VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(key) DO UPDATE SET content=excluded.content, kind=excluded.kind, updated_at=excluded.updated_at",
            rusqlite::params![key, content, kind, now],
        )?;
        Ok(())
    }

    pub async fn memory_read(&self, key: Option<&str>) -> Result<Vec<(String, String, String)>> {
        let conn = self.conn.lock().await;
        let mut stmt = if let Some(k) = key {
            let mut s = conn.prepare("SELECT key, content, kind FROM memory WHERE key=?1")?;
            let rows = s.query_map(rusqlite::params![k], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?))
            })?;
            let mut out = Vec::new();
            for r in rows {
                out.push(r?);
            }
            return Ok(out);
        } else {
            conn.prepare("SELECT key, content, kind FROM memory ORDER BY updated_at DESC")?
        };
        let rows = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    pub async fn memory_forget(&self, key: &str) -> Result<()> {
        let conn = self.conn.lock().await;
        conn.execute("DELETE FROM memory WHERE key=?1", rusqlite::params![key])?;
        Ok(())
    }
}
