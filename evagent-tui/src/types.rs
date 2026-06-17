//! EvAgent TUI type definitions — cleaned and redesigned.
//! Removed: TimelineEvent, ToolCall, FileActivity, AgentTreeNode.
//! Added: LifecyclePhase, ToolInfo (embedded in AgentStatus).

#![allow(dead_code)]

// ─── WebSocket Messages ───

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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
    SessionUpdate { session: serde_json::Value },
    #[serde(rename = "AgentList")]
    AgentList { agents: Vec<serde_json::Value> },
    #[serde(rename = "SkillList")]
    SkillList { skills: Vec<serde_json::Value> },
    #[serde(rename = "SessionList")]
    SessionList { sessions: Vec<serde_json::Value> },
    #[serde(rename = "Error")]
    Error { message: String },
}

// ─── Agent State ───

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

// ─── Lifecycle Phase ───

#[derive(Debug, Clone, PartialEq)]
pub enum LifecyclePhase {
    Thinking,
    Planning,
    Executing,
    Verifying,
    Merging,
}

impl LifecyclePhase {
    pub fn as_str(&self) -> &'static str {
        match self {
            LifecyclePhase::Thinking => "THINKING",
            LifecyclePhase::Planning => "PLANNING",
            LifecyclePhase::Executing => "EXECUTING",
            LifecyclePhase::Verifying => "VERIFYING",
            LifecyclePhase::Merging => "MERGING",
        }
    }

    pub fn all() -> Vec<LifecyclePhase> {
        vec![
            LifecyclePhase::Thinking,
            LifecyclePhase::Planning,
            LifecyclePhase::Executing,
            LifecyclePhase::Verifying,
            LifecyclePhase::Merging,
        ]
    }

    pub fn index(&self) -> usize {
        match self {
            LifecyclePhase::Thinking => 0,
            LifecyclePhase::Planning => 1,
            LifecyclePhase::Executing => 2,
            LifecyclePhase::Verifying => 3,
            LifecyclePhase::Merging => 4,
        }
    }
}

// ─── Tool Info (embedded in AgentStatus cards) ───

#[derive(Debug, Clone)]
pub struct ToolInfo {
    pub name: String,
    pub target: String,
}

// ─── Agent Status (richer for card rendering) ───

#[derive(Debug, Clone)]
pub struct AgentStatus {
    pub task_id: String,
    pub agent_name: String,
    pub status: AgentState,
    pub progress: f32,
    pub progress_text: String,
    pub tokens_used: u64,
    pub wall_clock_ms: u64,
    /// Tool calls used by this agent (embedded in card)
    pub tools_used: Vec<ToolInfo>,
    /// Diff summary like "+28 -6"
    pub diff_summary: String,
}

// ─── Chat Message ───

#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: String,   // "user", "assistant", "system"
    pub content: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Agent name if this is an agent card message
    pub agent_name: Option<String>,
    /// Progress info for agent cards
    pub agent_progress: Option<f32>,
    /// Tools used (for agent cards)
    pub agent_tools: Vec<ToolInfo>,
    /// Diff summary (for agent cards)
    pub agent_diff: String,
}

impl ChatMessage {
    pub fn new(role: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: role.into(),
            content: content.into(),
            timestamp: chrono::Utc::now(),
            agent_name: None,
            agent_progress: None,
            agent_tools: Vec::new(),
            agent_diff: String::new(),
        }
    }

    pub fn agent_card(
        agent_name: impl Into<String>,
        content: impl Into<String>,
        progress: f32,
        tools: Vec<ToolInfo>,
        diff: String,
    ) -> Self {
        Self {
            role: "agent".to_string(),
            content: content.into(),
            timestamp: chrono::Utc::now(),
            agent_name: Some(agent_name.into()),
            agent_progress: Some(progress),
            agent_tools: tools,
            agent_diff: diff,
        }
    }
}

// ─── Session Info ───

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

// ─── Session Stats ───

#[derive(Debug, Clone, Default)]
pub struct SessionStats {
    pub total_tokens: u64,
    pub total_cost: f64,
    pub domain: String,
    pub total_agents: usize,
    pub completed_agents: usize,
}

// ─── Connection State ───

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

pub fn fmt_cost(cost: f64) -> String {
    if cost >= 1.0 {
        format!("${:.2}", cost)
    } else if cost >= 0.01 {
        format!("${:.3}", cost)
    } else {
        format!("${:.4}", cost)
    }
}
