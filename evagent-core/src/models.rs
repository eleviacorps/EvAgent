use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ── Agent ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDefinition {
    pub name: String,
    pub domain: String,
    pub description: String,
    #[serde(default)]
    pub tool_scope: Vec<String>,
    pub model_preference: Option<String>,
    #[serde(default = "default_permission_profile")]
    pub permission_profile: String,
}

fn default_permission_profile() -> String {
    "default".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentIndexEntry {
    pub name: String,
    pub domain: String,
    pub description: String,
    pub tool_scope: Vec<String>,
    pub permission_profile: String,
    pub source_path: String,
    pub last_modified: DateTime<Utc>,
}

// ── Skill ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillDefinition {
    pub name: String,
    pub domain: String,
    #[serde(default)]
    pub trigger_patterns: Vec<String>,
    #[serde(default)]
    pub applicable_agents: Vec<String>,
    #[serde(default)]
    pub steps: Vec<String>,
    #[serde(default)]
    pub examples: Vec<String>,
    #[serde(default)]
    pub anti_patterns: Vec<String>,
    #[serde(default)]
    pub version: u32,
}

// ── Routing ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouterOutput {
    pub domain: String,
    pub agent_candidates: Vec<String>,
    pub confidence: f64,
    pub matched_pattern: Option<String>,
    pub llm_fallback_used: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisteredDomain {
    pub name: String,
    pub patterns: Vec<String>,
    pub agents: Vec<String>,
}

// ── Dispatch ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DispatchTask {
    pub id: String,
    pub goal: String,
    #[serde(default)]
    pub context: String,
    #[serde(default)]
    pub assigned_skills: Vec<String>,
    #[serde(default)]
    pub permission_profile: String,
    pub timeout_secs: u64,
    pub token_budget: u64,
    pub domain: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubAgentOutput {
    pub task_id: String,
    pub status: SubAgentStatus,
    pub result: Option<String>,
    pub tokens_used: u64,
    #[serde(default)]
    pub errors: Vec<String>,
    pub wall_clock_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SubAgentStatus {
    Completed,
    Failed,
    Timeout,
}

// ── Session ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub domain: String,
    #[serde(default)]
    pub dispatch_agents: Vec<String>,
    pub total_cost: f64,
    pub total_tokens: u64,
    pub wall_clock_ms: u64,
    pub created_at: DateTime<Utc>,
    pub status: SessionStatus,
    pub message_count: u32,
    pub summary: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SessionStatus {
    Active,
    Archived,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub session_id: String,
    pub role: MessageRole,
    pub content: String,
    pub timestamp: DateTime<Utc>,
    pub tokens: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MessageRole {
    User,
    Assistant,
    SubAgent,
    System,
}

// ── Permissions ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionProfile {
    pub name: String,
    #[serde(default)]
    pub allow_tools: Vec<String>,
    #[serde(default)]
    pub allow_domains: Vec<String>,
    pub max_tokens: Option<u64>,
    pub timeout_secs: Option<u64>,
    #[serde(default = "default_network_access")]
    pub network_access: bool,
    #[serde(default = "default_filesystem_access")]
    pub filesystem_access_level: FilesystemAccessLevel,
}

fn default_network_access() -> bool {
    false
}

fn default_filesystem_access() -> FilesystemAccessLevel {
    FilesystemAccessLevel::None
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FilesystemAccessLevel {
    ReadOnly,
    ReadWrite,
    None,
}

// ── WebSocket Messages ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WsClientMessage {
    DispatchTask {
        goal: String,
        context: Option<String>,
        domain: Option<String>,
    },
    AgentList,
    SkillList,
    SessionList,
    SessionDetail {
        session_id: String,
    },
    ConfigUpdate {
        key: String,
        value: serde_json::Value,
    },
    Ping,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WsServerMessage {
    SubAgentUpdate {
        task_id: String,
        agent_name: String,
        status: SubAgentStatus,
        progress: Option<String>,
        tokens_used: u64,
        wall_clock_ms: u64,
    },
    DispatchResult {
        session_id: String,
        outputs: Vec<SubAgentOutput>,
        aggregated: Option<String>,
    },
    SessionUpdate {
        session: Session,
    },
    AgentList {
        agents: Vec<AgentIndexEntry>,
    },
    SkillList {
        skills: Vec<SkillDefinition>,
    },
    SessionList {
        sessions: Vec<Session>,
    },
    Error {
        message: String,
    },
    Pong,
}

// ── Config Schema ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HermesConfig {
    #[serde(default)]
    pub core: CoreConfig,
    #[serde(default)]
    pub store: StoreConfig,
    #[serde(default)]
    pub dispatch: DispatchConfig,
    #[serde(default)]
    pub security: SecurityConfig,
    #[serde(default)]
    pub server: ServerConfig,
    #[serde(default)]
    pub routing: RoutingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreConfig {
    #[serde(default = "default_max_concurrent")]
    pub max_concurrent_agents: usize,
    #[serde(default = "default_timeout_secs")]
    pub default_timeout_secs: u64,
}

fn default_max_concurrent() -> usize {
    5
}
fn default_timeout_secs() -> u64 {
    120
}

impl Default for CoreConfig {
    fn default() -> Self {
        Self {
            max_concurrent_agents: default_max_concurrent(),
            default_timeout_secs: default_timeout_secs(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreConfig {
    #[serde(default = "default_db_path")]
    pub db_path: String,
    #[serde(default = "default_session_ttl")]
    pub session_ttl_days: u32,
    #[serde(default = "default_archive_after")]
    pub archive_after_days: u32,
}

fn default_db_path() -> String {
    "hermes.db".to_string()
}
fn default_session_ttl() -> u32 {
    30
}
fn default_archive_after() -> u32 {
    7
}

impl Default for StoreConfig {
    fn default() -> Self {
        Self {
            db_path: default_db_path(),
            session_ttl_days: default_session_ttl(),
            archive_after_days: default_archive_after(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DispatchConfig {
    #[serde(default = "default_max_concurrent_dispatch")]
    pub max_concurrent: usize,
    #[serde(default = "default_token_budget")]
    pub default_token_budget: u64,
}

fn default_max_concurrent_dispatch() -> usize {
    5
}
fn default_token_budget() -> u64 {
    4096
}

impl Default for DispatchConfig {
    fn default() -> Self {
        Self {
            max_concurrent: default_max_concurrent_dispatch(),
            default_token_budget: default_token_budget(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    #[serde(default)]
    pub default_network_access: bool,
    #[serde(default)]
    pub allowed_domains: Vec<String>,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            default_network_access: false,
            allowed_domains: vec![],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
}

fn default_host() -> String {
    "127.0.0.1".to_string()
}
fn default_port() -> u16 {
    9753
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingConfig {
    #[serde(default = "default_min_confidence")]
    pub min_confidence: f64,
    #[serde(default = "default_max_fs_walk_depth")]
    pub max_fs_walk_depth: u32,
}

fn default_min_confidence() -> f64 {
    0.6
}
fn default_max_fs_walk_depth() -> u32 {
    3
}

impl Default for RoutingConfig {
    fn default() -> Self {
        Self {
            min_confidence: default_min_confidence(),
            max_fs_walk_depth: default_max_fs_walk_depth(),
        }
    }
}

impl Default for HermesConfig {
    fn default() -> Self {
        Self {
            core: CoreConfig::default(),
            store: StoreConfig::default(),
            dispatch: DispatchConfig::default(),
            security: SecurityConfig::default(),
            server: ServerConfig::default(),
            routing: RoutingConfig::default(),
        }
    }
}
