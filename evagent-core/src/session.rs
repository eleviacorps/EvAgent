use std::sync::{Arc, Mutex};

use chrono::{DateTime, Utc};
use rusqlite::Connection;
use uuid::Uuid;

use crate::errors::{HermesError, HermesResult};
use crate::models::{Message, MessageRole, Session, SessionStatus};
use tracing::{debug, info, warn};

/// SQLite-backed session store with pagination, archiving, and pruning.
pub struct SessionStore {
    pub(crate) db: Arc<Mutex<Connection>>,
    session_ttl_days: u32,
    archive_after_days: u32,
}

impl SessionStore {
    pub fn new(
        conn: Arc<Mutex<Connection>>,
        session_ttl_days: u32,
        archive_after_days: u32,
    ) -> HermesResult<Self> {
        let store = Self {
            db: conn,
            session_ttl_days,
            archive_after_days,
        };
        store.init_schema()?;
        Ok(store)
    }

    fn init_schema(&self) -> HermesResult<()> {
        let db = self.db.lock().map_err(|e| HermesError::store(e.to_string()))?;
        db.execute_batch(
            "CREATE TABLE IF NOT EXISTS sessions (
                id TEXT PRIMARY KEY,
                domain TEXT NOT NULL,
                dispatch_agents TEXT NOT NULL DEFAULT '[]',
                total_cost REAL NOT NULL DEFAULT 0.0,
                total_tokens INTEGER NOT NULL DEFAULT 0,
                wall_clock_ms INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'Active',
                message_count INTEGER NOT NULL DEFAULT 0,
                summary TEXT
            );
            CREATE TABLE IF NOT EXISTS messages (
                id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                tokens INTEGER NOT NULL DEFAULT 0,
                FOREIGN KEY (session_id) REFERENCES sessions(id)
            );
            CREATE INDEX IF NOT EXISTS idx_messages_session ON messages(session_id, timestamp);
            CREATE INDEX IF NOT EXISTS idx_sessions_status ON sessions(status);
            CREATE INDEX IF NOT EXISTS idx_sessions_created ON sessions(created_at);",
        )
        .map_err(|e| HermesError::store_with("Failed to init session schema", e))?;
        Ok(())
    }

    pub fn create(&self, domain: &str) -> HermesResult<Session> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        let session = Session {
            id: id.clone(),
            domain: domain.to_string(),
            dispatch_agents: vec![],
            total_cost: 0.0,
            total_tokens: 0,
            wall_clock_ms: 0,
            created_at: Utc::now(),
            status: SessionStatus::Active,
            message_count: 0,
            summary: None,
        };

        let db = self.db.lock().map_err(|e| HermesError::store(e.to_string()))?;
        db.execute(
            "INSERT INTO sessions (id, domain, dispatch_agents, total_cost, total_tokens, wall_clock_ms, created_at, status, message_count, summary)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            rusqlite::params![
                id,
                domain,
                serde_json::to_string(&Vec::<String>::new()).map_err(|e| HermesError::store_with("Serialize error", e))?,
                0.0,
                0u64,
                0u64,
                now,
                "Active",
                0u32,
                None::<String>,
            ],
        )
        .map_err(|e| HermesError::store_with("Failed to create session", e))?;

        debug!("Created session {} for domain '{}'", id, domain);
        Ok(session)
    }

    pub fn append_message(&self, session_id: &str, message: Message) -> HermesResult<()> {
        let db = self.db.lock().map_err(|e| HermesError::store(e.to_string()))?;
        db.execute(
            "INSERT INTO messages (id, session_id, role, content, timestamp, tokens)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![
                message.id,
                session_id,
                format!("{:?}", message.role),
                message.content,
                message.timestamp.to_rfc3339(),
                message.tokens,
            ],
        )
        .map_err(|e| HermesError::store_with("Failed to append message", e))?;

        db.execute(
            "UPDATE sessions SET message_count = message_count + 1, total_tokens = total_tokens + ?1 WHERE id = ?2",
            rusqlite::params![message.tokens, session_id],
        )
        .map_err(|e| HermesError::store_with("Failed to update session counters", e))?;

        debug!(
            "Appended {} message to session {} ({} tokens)",
            format!("{:?}", message.role),
            session_id,
            message.tokens
        );
        Ok(())
    }

    pub fn get(&self, session_id: &str) -> HermesResult<Session> {
        let db = self.db.lock().map_err(|e| HermesError::store(e.to_string()))?;
        let mut stmt = db
            .prepare(
                "SELECT id, domain, dispatch_agents, total_cost, total_tokens, wall_clock_ms, created_at, status, message_count, summary
                 FROM sessions WHERE id = ?1",
            )
            .map_err(|e| HermesError::store_with("Failed to prepare session query", e))?;

        let result = stmt.query_row(rusqlite::params![session_id], |row| {
            let agents_str: String = row.get(2)?;
            let status_str: String = row.get(7)?;
            let summary: Option<String> = row.get(9)?;
            let created_at_str: String = row.get(6)?;

            let dispatch_agents: Vec<String> = serde_json::from_str(&agents_str).unwrap_or_default();
            let status = match status_str.as_str() {
                "Archived" => SessionStatus::Archived,
                _ => SessionStatus::Active,
            };
            let created_at = DateTime::parse_from_rfc3339(&created_at_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());

            Ok(Session {
                id: row.get(0)?,
                domain: row.get(1)?,
                dispatch_agents,
                total_cost: row.get(3)?,
                total_tokens: row.get(4)?,
                wall_clock_ms: row.get(5)?,
                created_at,
                status,
                message_count: row.get(8)?,
                summary,
            })
        });

        match result {
            Ok(session) => Ok(session),
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                Err(HermesError::session(format!("Session '{}' not found", session_id)))
            }
            Err(e) => Err(HermesError::store_with("Failed to query session", e)),
        }
    }

    pub fn list(&self, status_filter: Option<SessionStatus>) -> HermesResult<Vec<Session>> {
        let db = self.db.lock().map_err(|e| HermesError::store(e.to_string()))?;

        let (query, params): (String, Vec<Box<dyn rusqlite::types::ToSql>>) = match status_filter {
            Some(ref s) => (
                "SELECT id, domain, dispatch_agents, total_cost, total_tokens, wall_clock_ms, created_at, status, message_count, summary
                 FROM sessions WHERE status = ?1 ORDER BY created_at DESC".to_string(),
                vec![Box::new(format!("{:?}", s)) as Box<dyn rusqlite::types::ToSql>],
            ),
            None => (
                "SELECT id, domain, dispatch_agents, total_cost, total_tokens, wall_clock_ms, created_at, status, message_count, summary
                 FROM sessions ORDER BY created_at DESC".to_string(),
                vec![],
            ),
        };

        let mut stmt = db
            .prepare(&query)
            .map_err(|e| HermesError::store_with("Failed to prepare list query", e))?;

        let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();

        let sessions = stmt
            .query_map(param_refs.as_slice(), |row| {
                let agents_str: String = row.get(2)?;
                let status_str: String = row.get(7)?;
                let summary: Option<String> = row.get(9)?;
                let created_at_str: String = row.get(6)?;

                let dispatch_agents: Vec<String> = serde_json::from_str(&agents_str).unwrap_or_default();
                let status = match status_str.as_str() {
                    "Archived" => SessionStatus::Archived,
                    _ => SessionStatus::Active,
                };
                let created_at = DateTime::parse_from_rfc3339(&created_at_str)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now());

                Ok(Session {
                    id: row.get(0)?,
                    domain: row.get(1)?,
                    dispatch_agents,
                    total_cost: row.get(3)?,
                    total_tokens: row.get(4)?,
                    wall_clock_ms: row.get(5)?,
                    created_at,
                    status,
                    message_count: row.get(8)?,
                    summary,
                })
            })
            .map_err(|e| HermesError::store_with("Failed to query sessions", e))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(sessions)
    }

    pub fn get_messages(
        &self,
        session_id: &str,
        page: u32,
        page_size: u32,
    ) -> HermesResult<Vec<Message>> {
        let offset = (page.saturating_sub(1)) * page_size;
        let db = self.db.lock().map_err(|e| HermesError::store(e.to_string()))?;

        let mut stmt = db
            .prepare(
                "SELECT id, session_id, role, content, timestamp, tokens
                 FROM messages WHERE session_id = ?1
                 ORDER BY timestamp ASC
                 LIMIT ?2 OFFSET ?3",
            )
            .map_err(|e| HermesError::store_with("Failed to prepare messages query", e))?;

        let messages = stmt
            .query_map(rusqlite::params![session_id, page_size, offset], |row| {
                let role_str: String = row.get(2)?;
                let ts_str: String = row.get(4)?;
                let role = match role_str.as_str() {
                    "User" => MessageRole::User,
                    "Assistant" => MessageRole::Assistant,
                    "SubAgent" => MessageRole::SubAgent,
                    _ => MessageRole::System,
                };
                let timestamp = DateTime::parse_from_rfc3339(&ts_str)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now());

                Ok(Message {
                    id: row.get(0)?,
                    session_id: row.get(1)?,
                    role,
                    content: row.get(3)?,
                    timestamp,
                    tokens: row.get(5)?,
                })
            })
            .map_err(|e| HermesError::store_with("Failed to query messages", e))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(messages)
    }

    pub fn archive(&self, session_id: &str) -> HermesResult<()> {
        let db = self.db.lock().map_err(|e| HermesError::store(e.to_string()))?;
        let affected = db
            .execute(
                "UPDATE sessions SET status = 'Archived' WHERE id = ?1 AND status = 'Active'",
                rusqlite::params![session_id],
            )
            .map_err(|e| HermesError::store_with("Failed to archive session", e))?;

        if affected == 0 {
            return Err(HermesError::session(format!(
                "Session '{}' not found or already archived",
                session_id
            )));
        }

        info!("Archived session {}", session_id);
        Ok(())
    }

    pub fn archive_older_than(&self, days: u32) -> HermesResult<u32> {
        let cutoff = (Utc::now() - chrono::Duration::days(days as i64)).to_rfc3339();
        let db = self.db.lock().map_err(|e| HermesError::store(e.to_string()))?;
        let affected = db
            .execute(
                "UPDATE sessions SET status = 'Archived' WHERE status = 'Active' AND created_at < ?1",
                rusqlite::params![cutoff],
            )
            .map_err(|e| HermesError::store_with("Failed to archive old sessions", e))?;

        if affected > 0 {
            info!("Archived {} old sessions ({} days)", affected, days);
        }
        Ok(affected as u32)
    }

    pub fn prune(&self, days: u32) -> HermesResult<u32> {
        let cutoff = (Utc::now() - chrono::Duration::days(days as i64)).to_rfc3339();
        let db = self.db.lock().map_err(|e| HermesError::store(e.to_string()))?;

        db.execute(
            "DELETE FROM messages WHERE session_id IN (SELECT id FROM sessions WHERE created_at < ?1)",
            rusqlite::params![cutoff],
        )
        .map_err(|e| HermesError::store_with("Failed to prune messages", e))?;

        let affected = db
            .execute(
                "DELETE FROM sessions WHERE created_at < ?1",
                rusqlite::params![cutoff],
            )
            .map_err(|e| HermesError::store_with("Failed to prune sessions", e))?;

        if affected > 0 {
            info!("Pruned {} sessions older than {} days", affected, days);
        }
        Ok(affected as u32)
    }

    pub fn run_maintenance(&self) -> HermesResult<()> {
        self.archive_older_than(self.archive_after_days)?;
        self.prune(self.session_ttl_days)?;
        Ok(())
    }

    pub fn update_summary(&self, session_id: &str, summary: &str) -> HermesResult<()> {
        let db = self.db.lock().map_err(|e| HermesError::store(e.to_string()))?;
        db.execute(
            "UPDATE sessions SET summary = ?1 WHERE id = ?2",
            rusqlite::params![summary, session_id],
        )
        .map_err(|e| HermesError::store_with("Failed to update session summary", e))?;
        Ok(())
    }

    pub fn update_dispatch_agents(&self, session_id: &str, agents: &[String]) -> HermesResult<()> {
        let agents_json = serde_json::to_string(agents)
            .map_err(|e| HermesError::store_with("Serialize error", e))?;
        let db = self.db.lock().map_err(|e| HermesError::store(e.to_string()))?;
        db.execute(
            "UPDATE sessions SET dispatch_agents = ?1 WHERE id = ?2",
            rusqlite::params![agents_json, session_id],
        )
        .map_err(|e| HermesError::store_with("Failed to update dispatch agents", e))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_store() -> SessionStore {
        let conn = Connection::open_in_memory().unwrap();
        SessionStore::new(Arc::new(Mutex::new(conn)), 30, 7).unwrap()
    }

    #[test]
    fn test_create_and_get_session() {
        let store = test_store();
        let session = store.create("general").unwrap();
        assert_eq!(session.domain, "general");
        assert_eq!(session.status, SessionStatus::Active);

        let loaded = store.get(&session.id).unwrap();
        assert_eq!(loaded.id, session.id);
    }

    #[test]
    fn test_append_and_get_messages() {
        let store = test_store();
        let session = store.create("general").unwrap();

        let msg = Message {
            id: Uuid::new_v4().to_string(),
            session_id: session.id.clone(),
            role: MessageRole::User,
            content: "Hello".to_string(),
            timestamp: Utc::now(),
            tokens: 5,
        };
        store.append_message(&session.id, msg).unwrap();

        let messages = store.get_messages(&session.id, 1, 10).unwrap();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].content, "Hello");

        let loaded = store.get(&session.id).unwrap();
        assert_eq!(loaded.message_count, 1);
    }

    #[test]
    fn test_archive_and_prune() {
        let store = test_store();
        let session = store.create("general").unwrap();
        store.archive(&session.id).unwrap();
        let loaded = store.get(&session.id).unwrap();
        assert_eq!(loaded.status, SessionStatus::Archived);
    }
}
