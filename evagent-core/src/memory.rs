//! Smart memory — three-tier system per spec.
//!
//! 1. User profile (USER.md) — persistent per-user facts, injected everywhere.
//! 2. Agent notes (MEMORY.md) — durable cross-session knowledge.
//! 3. Conversation history — FTS5-indexed in SQLite (see `session.rs`).
//!
//! Both file-backed stores are read fresh on every dispatch (so the user can
//! edit them outside the agent and the change is picked up immediately).

use crate::session::SessionStore;
use std::path::{Path, PathBuf};

pub struct Memory {
    user_profile_path: PathBuf,
    agent_notes_path: PathBuf,
    store: SessionStore,
}

const MAX_USER_CHARS: usize = 2200;
const MAX_AGENT_CHARS: usize = 2200;

impl Memory {
    pub fn new(root: &Path, store: SessionStore) -> Self {
        Self {
            user_profile_path: root.join("USER.md"),
            agent_notes_path: root.join("MEMORY.md"),
            store,
        }
    }

    pub fn user_profile(&self) -> String {
        std::fs::read_to_string(&self.user_profile_path)
            .unwrap_or_default()
            .chars()
            .take(MAX_USER_CHARS)
            .collect()
    }

    pub fn agent_notes(&self) -> String {
        std::fs::read_to_string(&self.agent_notes_path)
            .unwrap_or_default()
            .chars()
            .take(MAX_AGENT_CHARS)
            .collect()
    }

    pub async fn write_user_note(&self, key: &str, content: &str) -> anyhow::Result<()> {
        self.store
            .memory_write(key, content, "user")
            .await
            .map_err(|e| anyhow::anyhow!(e.to_string()))
    }

    pub async fn write_agent_note(&self, key: &str, content: &str) -> anyhow::Result<()> {
        self.store
            .memory_write(key, content, "agent")
            .await
            .map_err(|e| anyhow::anyhow!(e.to_string()))
    }

    pub async fn read_notes(&self, key: Option<&str>) -> anyhow::Result<Vec<(String, String, String)>> {
        self.store
            .memory_read(key)
            .await
            .map_err(|e| anyhow::anyhow!(e.to_string()))
    }

    pub async fn forget(&self, key: &str) -> anyhow::Result<()> {
        self.store
            .memory_forget(key)
            .await
            .map_err(|e| anyhow::anyhow!(e.to_string()))
    }

    pub async fn session_search(&self, query: &str, limit: u32) -> anyhow::Result<Vec<crate::models::SessionRow>> {
        self.store
            .search_messages(query, limit)
            .await
            .map_err(|e| anyhow::anyhow!(e.to_string()))
    }

    /// Compose the full memory context string to inject into an agent's system prompt.
    pub fn context_block(&self, domain: &str) -> String {
        let mut parts: Vec<String> = Vec::new();
        let up = self.user_profile();
        if !up.trim().is_empty() {
            parts.push(format!("# User Profile\n\n{}", up));
        }
        let an = self.agent_notes();
        if !an.trim().is_empty() {
            parts.push(format!("# Agent Notes\n\n{}", an));
        }
        parts.push(format!("# Current Domain\n\n{}", domain));
        parts.join("\n\n---\n\n")
    }
}
