#![allow(dead_code)]
use chrono::{DateTime, Utc};
use ratatui::style::Color;
use serde::{Deserialize, Serialize};
use std::time::Duration;

// ─── WebSocket Client Messages (TUI → Core) ───

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WsClientMessage {
    Ping,
    AgentList,
    SkillList,
    SessionList,
    DispatchTask {
        goal: String,
        context: Option<String>,
        domain: Option<String>,
    },
    ConfigUpdate {
        key: String,
        value: String,
    },
}

// ─── WebSocket Server Messages (Core → TUI) ───

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WsServerMessage {
    Pong,
    #[serde(rename = "SubAgentUpdate")]
    SubAgentUpdate {
        task_id: String,
        agent_name: String,
        status: String,
        progress: String,
        tokens_used: u64,
        wall_clock_ms: u64,
    },
    #[serde(rename = "DispatchResult")]
    DispatchResult {
        session_id: String,
        outputs: Vec<serde_json::Value>,
        aggregated: String,
    },
    #[serde(rename = "SessionUpdate")]
    SessionUpdate {
        session: serde_json::Value,
    },
    #[serde(rename = "AgentList")]
    AgentList {
        agents: Vec<serde_json::Value>,
    },
    #[serde(rename = "SkillList")]
    SkillList {
        skills: Vec<serde_json::Value>,
    },
    #[serde(rename = "SessionList")]
    SessionList {
        sessions: Vec<serde_json::Value>,
    },
    #[serde(rename = "Error")]
    Error {
        message: String,
    },
}

// ─── Internal State Types ───

#[derive(Debug, Clone)]
pub struct AgentStatus {
    pub task_id: String,
    pub agent_name: String,
    pub status: AgentState,
    pub progress: f32,
    pub progress_text: String,
    pub tokens_used: u64,
    pub wall_clock_ms: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AgentState {
    Running,
    Completed,
    Failed,
    Timeout,
    Idle,
}

impl AgentState {
    pub fn from_str(s: &str) -> Self {
        match s {
            "Completed" => AgentState::Completed,
            "Failed" => AgentState::Failed,
            "Timeout" => AgentState::Timeout,
            "Running" | "running" => AgentState::Running,
            _ => AgentState::Idle,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            AgentState::Running => "Running",
            AgentState::Completed => "Completed",
            AgentState::Failed => "Failed",
            AgentState::Timeout => "Timeout",
            AgentState::Idle => "Idle",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
    pub timestamp: DateTime<Utc>,
}

impl ChatMessage {
    pub fn new(role: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: role.into(),
            content: content.into(),
            timestamp: Utc::now(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SessionInfo {
    pub id: String,
    pub domain: String,
    pub total_tokens: u64,
    pub total_cost: f64,
    pub agent_count: usize,
    pub completed_agents: usize,
    pub status: SessionStatus,
}

impl Default for SessionInfo {
    fn default() -> Self {
        Self {
            id: String::new(),
            domain: String::from("general"),
            total_tokens: 0,
            total_cost: 0.0,
            agent_count: 0,
            completed_agents: 0,
            status: SessionStatus::Inactive,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum SessionStatus {
    Active,
    Inactive,
    Completed,
}

impl SessionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            SessionStatus::Active => "Active",
            SessionStatus::Inactive => "Inactive",
            SessionStatus::Completed => "Completed",
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct SessionStats {
    pub total_tokens: u64,
    pub total_cost: f64,
    pub domain: String,
    pub total_agents: usize,
    pub completed_agents: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionState {
    Connected,
    Disconnected,
    Connecting,
}

impl ConnectionState {
    pub fn as_str(&self) -> &'static str {
        match self {
            ConnectionState::Connected => "Connected",
            ConnectionState::Disconnected => "Disconnected",
            ConnectionState::Connecting => "Connecting...",
        }
    }
}

// ─── Domain Colors ───

pub fn domain_color(domain: &str) -> Color {
    match domain.to_lowercase().as_str() {
        "coding" => Color::Blue,
        "research" => Color::Green,
        "writing" => Color::Yellow,
        "trading" => Color::Magenta,
        "study" => Color::Cyan,
        "communication" => Color::LightMagenta,
        "media" => Color::Red,
        _ => Color::White,
    }
}

// ─── Neo-Terminal Command Center UI Types ───

/// A node in the agent tree hierarchy (flat with level for rendering).
#[derive(Debug, Clone)]
pub struct AgentTreeNode {
    pub name: String,
    pub level: usize,
    pub status: AgentState,
}

/// An event on the execution timeline.
#[derive(Debug, Clone)]
pub struct TimelineEvent {
    pub timestamp: String,
    pub agent_name: String,
    pub action: String,
    pub duration_ms: u64,
    pub status: AgentState,
    pub details: Vec<String>,
}

/// A recorded tool call.
#[derive(Debug, Clone)]
pub struct ToolCall {
    pub icon: String,
    pub tool_name: String,
    pub target: String,
    pub timestamp: String,
    pub duration_ms: u64,
}

/// A recorded file activity.
#[derive(Debug, Clone)]
pub struct FileActivity {
    pub path: String,
    pub action: String,
    pub timestamp: String,
}

// ─── Formatting Helpers ───

pub fn fmt_tokens(count: u64) -> String {
    if count >= 1_000_000 {
        format!("{:.1}M", count as f64 / 1_000_000.0)
    } else if count >= 1_000 {
        format!("{:.1}K", count as f64 / 1_000.0)
    } else {
        count.to_string()
    }
}

pub fn fmt_tokens_exact(count: u64) -> String {
    let s = count.to_string();
    let mut result = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.insert(0, ',');
        }
        result.insert(0, c);
    }
    result
}

pub fn fmt_duration(ms: u64) -> String {
    if ms >= 60_000 {
        format!("{}m{:02}s", ms / 60_000, (ms % 60_000) / 1000)
    } else if ms >= 1000 {
        format!("{:.1}s", ms as f64 / 1000.0)
    } else {
        format!("{}ms", ms)
    }
}

pub fn fmt_runtime(dur: Duration) -> String {
    let total_secs = dur.as_secs();
    let hours = total_secs / 3600;
    let minutes = (total_secs % 3600) / 60;
    let seconds = total_secs % 60;
    format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
}
