//! Error types for EvAgent core.
//!
//! Centralized error definitions so every module can return a consistent
//! `EvAgentError` and callers get structured context instead of bare strings.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum EvAgentError {
    #[error("config error: {0}")]
    Config(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("serde json error: {0}")]
    SerdeJson(#[from] serde_json::Error),

    #[error("serde yaml error: {0}")]
    SerdeYaml(#[from] serde_yaml::Error),

    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("websocket error: {0}")]
    WebSocket(String),

    #[error("intent routing failed: no domain matched above threshold")]
    RoutingNoMatch,

    #[error("agent not found: {0}")]
    AgentNotFound(String),

    #[error("skill not found: {0}")]
    SkillNotFound(String),

    #[error("permission denied: tool={tool} agent={agent}")]
    PermissionDenied { tool: String, agent: String },

    #[error("tool execution failed: {0}")]
    ToolExecution(String),

    #[error("llm error: {0}")]
    Llm(String),

    #[error("timeout after {0}s")]
    Timeout(u64),

    #[error("internal: {0}")]
    Internal(String),
}

pub type Result<T> = std::result::Result<T, EvAgentError>;

impl From<EvAgentError> for axum::response::Response<()> {
    fn from(_: EvAgentError) -> Self {
        unreachable!()
    }
}
