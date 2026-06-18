//! Shared data types used across the EvAgent core.
//!
//! Everything that flows over the WebSocket boundary or between modules
//! is defined here so there are no cross-module cycles.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Messages the client (TUI or Web GUI) sends to the core.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "PascalCase")]
pub enum ClientMessage {
    DispatchTask {
        goal: String,
        #[serde(default)]
        context: Option<serde_json::Value>,
        #[serde(default = "default_domain")]
        domain: String,
    },
    Ping,
}

fn default_domain() -> String {
    "general".to_string()
}

/// Messages the core sends back to the client.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "PascalCase")]
pub enum ServerMessage {
    Pong,
    DispatchResult {
        session_id: String,
        outputs: Vec<SubAgentOutput>,
        aggregated: String,
    },
    SubAgentUpdate {
        task_id: String,
        agent_name: String,
        status: AgentStatus,
        progress: String,
        tokens_used: u64,
    },
    Error {
        message: String,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub enum AgentStatus {
    Running,
    Completed,
    Failed,
}

/// Per-sub-agent output collected by the dispatcher.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubAgentOutput {
    pub task_id: String,
    pub agent_name: String,
    pub output: String,
    pub tokens_used: u64,
    pub wall_clock_ms: u128,
    pub status: AgentStatus,
}

/// A dispatch task — internal representation used by the dispatcher.
#[derive(Debug, Clone)]
pub struct DispatchTask {
    pub goal: String,
    pub context: Option<serde_json::Value>,
    pub domain: String,
    pub session_id: Uuid,
}

/// Domain definition — populated by intent_router from regex patterns.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Domain {
    pub name: String,
    pub patterns: Vec<String>,
    #[serde(default)]
    pub agents: Vec<String>,
    #[serde(default)]
    pub skills: Vec<String>,
    #[serde(default = "default_priority")]
    pub priority: u32,
}

fn default_priority() -> u32 {
    1
}

/// Agent YAML definition (one file per agent in `domains/<name>/agents/`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDef {
    pub name: String,
    pub role: String,
    #[serde(default)]
    pub domain: String,
    #[serde(default)]
    pub tools: Vec<String>,
    #[serde(default = "default_token_budget")]
    pub token_budget: u32,
    #[serde(default)]
    pub skills: Vec<String>,
    #[serde(default)]
    pub system_prompt: Option<String>,
}

fn default_token_budget() -> u32 {
    4096
}

/// Skill loaded from `domains/<name>/skills/SKILL.md`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    pub name: String,
    pub version: u32,
    pub domain: String,
    pub description: String,
    #[serde(default)]
    pub triggers: Vec<String>,
    pub body: String,
}

/// LLM request/response types (OpenAI-compatible).
#[derive(Debug, Clone, Serialize)]
pub struct LlmRequest {
    pub model: String,
    pub messages: Vec<LlmMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LlmResponse {
    pub choices: Vec<LlmChoice>,
    #[serde(default)]
    pub usage: LlmUsage,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LlmChoice {
    pub message: LlmMessage,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct LlmUsage {
    #[serde(default)]
    pub total_tokens: u64,
}

/// Stored session row in SQLite.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionRow {
    pub id: String,
    pub domain: String,
    pub total_tokens: u64,
    pub total_cost: f64,
    pub summary: String,
    pub created_at: i64,
}
