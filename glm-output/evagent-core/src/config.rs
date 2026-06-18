//! Configuration loading from `config.yaml` + `.env`.

use crate::errors::{EvAgentError, Result};
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub core: CoreConfig,
    pub store: StoreConfig,
    pub dispatch: DispatchConfig,
    pub security: SecurityConfig,
    pub server: ServerConfig,
    pub routing: RoutingConfig,
    #[serde(default)]
    pub llm: LlmConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CoreConfig {
    pub max_concurrent_agents: u32,
    pub default_timeout_secs: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StoreConfig {
    pub db_path: String,
    pub session_ttl_days: u32,
    pub archive_after_days: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DispatchConfig {
    pub max_concurrent: u32,
    pub default_token_budget: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SecurityConfig {
    pub default_network_access: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RoutingConfig {
    pub min_confidence: f32,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct LlmConfig {
    #[serde(default = "default_provider")]
    pub provider: String,
    #[serde(default)]
    pub base_url: String,
    #[serde(default)]
    pub model: String,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    #[serde(default = "default_temperature")]
    pub temperature: f32,
}

fn default_provider() -> String {
    "mock".to_string()
}
fn default_max_tokens() -> u32 {
    4096
}
fn default_temperature() -> f32 {
    0.7
}

impl Config {
    /// Load from a YAML file. `.env` is also read for `EVAGENT_*` overrides.
    pub fn load(path: &Path) -> Result<Self> {
        let _ = dotenvy::dotenv();
        let raw = std::fs::read_to_string(path)
            .map_err(|e| EvAgentError::Config(format!("read {}: {}", path.display(), e)))?;
        let mut cfg: Config = serde_yaml::from_str(&raw)
            .map_err(|e| EvAgentError::Config(format!("parse {}: {}", path.display(), e)))?;

        // Env overrides
        if let Ok(p) = std::env::var("EVAGENT_BASE_URL") {
            cfg.llm.base_url = p;
        }
        if let Ok(m) = std::env::var("EVAGENT_MODEL") {
            cfg.llm.model = m;
        }
        if let Ok(p) = std::env::var("EVAGENT_LLM_PROVIDER") {
            cfg.llm.provider = p;
        }
        if let Ok(port) = std::env::var("EVAGENT_PORT") {
            if let Ok(p) = port.parse::<u16>() {
                cfg.server.port = p;
            }
        }

        Ok(cfg)
    }

    pub fn default_path() -> &'static str {
        "config.yaml"
    }
}
