use std::fmt;
use thiserror::Error;

/// Central error type for the entire Hermes engine.
/// Every fallible function returns Result<T, HermesError>.
#[derive(Error, Debug)]
pub enum HermesError {
    #[error("[Config] {message}")]
    Config {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    #[error("[Router] {message}")]
    Router {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    #[error("[Dispatcher] {message}")]
    Dispatcher {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    #[error("[Session] {message}")]
    Session {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    #[error("[Permission] {message}")]
    Permission {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    #[error("[Skill] {message}")]
    Skill {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    #[error("[Agent] {message}")]
    Agent {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    #[error("[Store] {message}")]
    Store {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    #[error("[Io] {message}")]
    Io {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    #[error("[WebSocket] {message}")]
    WebSocket {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },
}

impl HermesError {
    pub fn config(msg: impl Into<String>) -> Self {
        HermesError::Config {
            message: msg.into(),
            source: None,
        }
    }

    pub fn config_with<E>(msg: impl Into<String>, err: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        HermesError::Config {
            message: msg.into(),
            source: Some(Box::new(err)),
        }
    }

    pub fn router(msg: impl Into<String>) -> Self {
        HermesError::Router {
            message: msg.into(),
            source: None,
        }
    }

    pub fn router_with<E>(msg: impl Into<String>, err: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        HermesError::Router {
            message: msg.into(),
            source: Some(Box::new(err)),
        }
    }

    pub fn dispatcher(msg: impl Into<String>) -> Self {
        HermesError::Dispatcher {
            message: msg.into(),
            source: None,
        }
    }

    pub fn dispatcher_with<E>(msg: impl Into<String>, err: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        HermesError::Dispatcher {
            message: msg.into(),
            source: Some(Box::new(err)),
        }
    }

    pub fn session(msg: impl Into<String>) -> Self {
        HermesError::Session {
            message: msg.into(),
            source: None,
        }
    }

    pub fn session_with<E>(msg: impl Into<String>, err: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        HermesError::Session {
            message: msg.into(),
            source: Some(Box::new(err)),
        }
    }

    pub fn permission(msg: impl Into<String>) -> Self {
        HermesError::Permission {
            message: msg.into(),
            source: None,
        }
    }

    pub fn permission_with<E>(msg: impl Into<String>, err: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        HermesError::Permission {
            message: msg.into(),
            source: Some(Box::new(err)),
        }
    }

    pub fn skill(msg: impl Into<String>) -> Self {
        HermesError::Skill {
            message: msg.into(),
            source: None,
        }
    }

    pub fn skill_with<E>(msg: impl Into<String>, err: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        HermesError::Skill {
            message: msg.into(),
            source: Some(Box::new(err)),
        }
    }

    pub fn agent(msg: impl Into<String>) -> Self {
        HermesError::Agent {
            message: msg.into(),
            source: None,
        }
    }

    pub fn agent_with<E>(msg: impl Into<String>, err: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        HermesError::Agent {
            message: msg.into(),
            source: Some(Box::new(err)),
        }
    }

    pub fn store(msg: impl Into<String>) -> Self {
        HermesError::Store {
            message: msg.into(),
            source: None,
        }
    }

    pub fn store_with<E>(msg: impl Into<String>, err: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        HermesError::Store {
            message: msg.into(),
            source: Some(Box::new(err)),
        }
    }

    pub fn io(msg: impl Into<String>) -> Self {
        HermesError::Io {
            message: msg.into(),
            source: None,
        }
    }

    pub fn io_with<E>(msg: impl Into<String>, err: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        HermesError::Io {
            message: msg.into(),
            source: Some(Box::new(err)),
        }
    }

    pub fn websocket(msg: impl Into<String>) -> Self {
        HermesError::WebSocket {
            message: msg.into(),
            source: None,
        }
    }

    pub fn websocket_with<E>(msg: impl Into<String>, err: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        HermesError::WebSocket {
            message: msg.into(),
            source: Some(Box::new(err)),
        }
    }
}

/// Convenience alias for Results using HermesError.
pub type HermesResult<T> = Result<T, HermesError>;

impl From<std::io::Error> for HermesError {
    fn from(err: std::io::Error) -> Self {
        HermesError::Io {
            message: err.to_string(),
            source: Some(Box::new(err)),
        }
    }
}

impl From<serde_yaml::Error> for HermesError {
    fn from(err: serde_yaml::Error) -> Self {
        HermesError::Config {
            message: format!("YAML parse error: {}", err),
            source: Some(Box::new(err)),
        }
    }
}

impl From<serde_json::Error> for HermesError {
    fn from(err: serde_json::Error) -> Self {
        HermesError::Store {
            message: format!("JSON serialization error: {}", err),
            source: Some(Box::new(err)),
        }
    }
}

impl From<rusqlite::Error> for HermesError {
    fn from(err: rusqlite::Error) -> Self {
        HermesError::Store {
            message: format!("SQLite error: {}", err),
            source: Some(Box::new(err)),
        }
    }
}
