//! Application state — cleaned and redesigned.
//! Removed: timeline_events, tool_calls, file_activities, token_limit, agent_tree.
//! Added: lifecycle_phase, mission_text.

#![allow(dead_code)]

use std::collections::HashMap;
use std::time::{Duration, Instant};
use ratatui::style::Color;
use tokio::sync::mpsc;

use crate::extract::{extract_diff_summary, extract_tool_info, parse_progress};
use crate::types::*;

/// Maximum number of chat messages to keep.
const MAX_CHAT_HISTORY: usize = 200;
/// Maximum number of agent status cards to show.
const MAX_AGENT_DISPLAY: usize = 20;

/// Main application state.
pub struct App {
    /// Current domain (e.g. "general", "coding")
    pub domain: String,
    /// Recent session info
    pub sessions: Vec<SessionInfo>,
    /// Currently tracked agent statuses (used for card rendering)
    pub active_agents: Vec<AgentStatus>,
    /// Chat conversation history (user + system + agent cards)
    pub chat_messages: Vec<ChatMessage>,
    /// Current user text input buffer
    pub input: String,
    /// Cursor position in input
    pub input_cursor: usize,
    /// Aggregated session stats (tokens, cost, etc.)
    pub stats: SessionStats,
    /// WebSocket connection status
    pub connection_status: ConnectionState,
    /// Last dispatch result text
    pub aggregated_result: Option<String>,
    /// Whether we should scroll chat to bottom
    pub scroll_chat: bool,
    /// Domain → color mapping (cached)
    pub domain_colors: HashMap<String, Color>,
    /// Available agents from server (metadata)
    pub available_agents: Vec<String>,
    /// Available skills from server (metadata)
    pub available_skills: Vec<String>,
    /// Track if we've requested initial data
    pub initialized: bool,
    /// Track if connection failure message has been shown
    pub connection_fail_shown: bool,
    /// Current lifecycle phase
    pub lifecycle_phase: LifecyclePhase,
    /// Current mission description
    pub mission_text: String,
    /// When the current session started
    pub session_start_time: Option<Instant>,
    /// Current session runtime
    pub runtime: Duration,
}

impl Default for App {
    fn default() -> Self {
        let mut domain_colors = HashMap::new();
        domain_colors.insert("coding".into(), Color::Rgb(79, 195, 247));
        domain_colors.insert("research".into(), Color::Rgb(74, 222, 128));
        domain_colors.insert("writing".into(), Color::Rgb(251, 191, 36));
        domain_colors.insert("trading".into(), Color::Rgb(239, 68, 68));
        domain_colors.insert("study".into(), Color::Rgb(79, 195, 247));
        domain_colors.insert("communication".into(), Color::Rgb(127, 136, 150));
        domain_colors.insert("media".into(), Color::Rgb(239, 68, 68));
        domain_colors.insert("general".into(), Color::Rgb(215, 220, 229));

        Self {
            domain: String::from("general"),
            sessions: Vec::new(),
            active_agents: Vec::new(),
            chat_messages: Vec::new(),
            input: String::new(),
            input_cursor: 0,
            stats: SessionStats::default(),
            connection_status: ConnectionState::Connecting,
            aggregated_result: None,
            scroll_chat: true,
            domain_colors,
            available_agents: Vec::new(),
            available_skills: Vec::new(),
            initialized: false,
            connection_fail_shown: false,
            lifecycle_phase: LifecyclePhase::Thinking,
            mission_text: String::from("Awaiting your prompt..."),
            session_start_time: None,
            runtime: Duration::ZERO,
        }
    }
}

impl App {
    pub fn new() -> Self {
        Self::default()
    }

    /// Process an incoming server message and update state.
    pub fn update_from_ws(&mut self, msg: WsServerMessage) {
        match msg {
            WsServerMessage::Pong => {
                self.connection_status = ConnectionState::Connected;
                self.connection_fail_shown = false;
            }
            WsServerMessage::SubAgentUpdate {
                task_id,
                agent_name,
                status,
                progress,
                tokens_used,
                wall_clock_ms,
            } => {
                let progress_val = parse_progress(&progress);
                let agent_state = AgentState::from_str(&status);

                // Extract tool info and diff from progress text
                let tool_info = extract_tool_info(&progress);
                let diff = extract_diff_summary(&progress);

                // Find existing agent or add new one
                if let Some(existing) = self
                    .active_agents
                    .iter_mut()
                    .find(|a| a.task_id == task_id
                        || (a.agent_name == agent_name && a.status == AgentState::Running))
                {
                    existing.status = agent_state.clone();
                    existing.progress = progress_val;
                    existing.progress_text = progress.clone();
                    existing.tokens_used = tokens_used;
                    existing.wall_clock_ms = wall_clock_ms;
                    // Add tool if new
                    if let Some(ti) = tool_info {
                        if !existing.tools_used.iter().any(|t| t.name == ti.name && t.target == ti.target) {
                            existing.tools_used.push(ti);
                        }
                    }
                    if !diff.is_empty() {
                        existing.diff_summary = diff.clone();
                    }
                } else {
                    let mut tools = Vec::new();
                    if let Some(ti) = tool_info {
                        tools.push(ti);
                    }
                    self.active_agents.push(AgentStatus {
                        task_id,
                        agent_name: agent_name.clone(),
                        status: agent_state.clone(),
                        progress: progress_val,
                        progress_text: progress.clone(),
                        tokens_used,
                        wall_clock_ms,
                        tools_used: tools,
                        diff_summary: diff,
                    });
                }

                // Update aggregate stats
                self.stats.total_tokens = self
                    .active_agents
                    .iter()
                    .map(|a| a.tokens_used)
                    .sum();
                self.stats.completed_agents = self
                    .active_agents
                    .iter()
                    .filter(|a| matches!(a.status, AgentState::Completed | AgentState::Failed | AgentState::Timeout))
                    .count();
                self.stats.total_agents = self.active_agents.len();

                // Limit displayed agents
                if self.active_agents.len() > MAX_AGENT_DISPLAY {
                    let running_ids: Vec<String> = self.active_agents.iter()
                        .filter(|a| a.status == AgentState::Running)
                        .map(|a| a.task_id.clone())
                        .collect();
                    self.active_agents.retain(|a| {
                        a.status == AgentState::Running || running_ids.contains(&a.task_id)
                    });
                }

                // Start session timer on first agent activity
                if self.session_start_time.is_none() {
                    self.session_start_time = Some(Instant::now());
                }

                // Auto-detect lifecycle phase from running agents
                self.detect_lifecycle_phase();

                // Add/update chat message for this agent's activity
                // Only add card if status changed
                let should_add_card = match agent_state {
                    AgentState::Completed => {
                        self.add_chat_message(
                            "system",
                            &format!("✅ Agent **{}** completed — {} tokens in {}ms",
                                agent_name, tokens_used, wall_clock_ms),
                        );
                        true
                    }
                    AgentState::Failed => {
                        self.add_chat_message(
                            "system",
                            &format!("❌ Agent **{}** failed: {}", agent_name, progress),
                        );
                        true
                    }
                    AgentState::Timeout => {
                        self.add_chat_message(
                            "system",
                            &format!("⏱️ Agent **{}** timed out", agent_name),
                        );
                        true
                    }
                    AgentState::Running => {
                        // Add/update running card
                        self.update_agent_card(&agent_name, &progress, progress_val);
                        false
                    }
                    AgentState::Idle => false,
                };

                if should_add_card {
                    // Find the agent and add a final card
                    if let Some(agent) = self.active_agents.iter()
                        .find(|a| a.agent_name == agent_name)
                    {
                        self.chat_messages.push(ChatMessage::agent_card(
                            &agent.agent_name,
                            &agent.progress_text,
                            agent.progress,
                            agent.tools_used.clone(),
                            agent.diff_summary.clone(),
                        ));
                        while self.chat_messages.len() > MAX_CHAT_HISTORY {
                            self.chat_messages.remove(0);
                        }
                        self.scroll_chat = true;
                    }
                }
            }
            WsServerMessage::DispatchResult {
                session_id,
                outputs,
                aggregated,
            } => {
                self.aggregated_result = Some(aggregated.clone());
                self.stats.domain = self.domain.clone();

                let resp_text = aggregated.clone();
                self.add_chat_message("assistant", &resp_text);

                if !outputs.is_empty() {
                    self.add_chat_message("system", &format!("📦 Received {} agent output(s)", outputs.len()));
                }

                for agent in self.active_agents.iter_mut() {
                    if agent.status == AgentState::Running {
                        agent.status = AgentState::Completed;
                        agent.progress = 100.0;
                    }
                }
                self.stats.completed_agents = self.stats.total_agents;
                self.lifecycle_phase = LifecyclePhase::Merging;
            }
            WsServerMessage::SessionUpdate { session } => {
                if let Some(_id) = session.get("id").and_then(|v| v.as_str()) {
                    let domain = session
                        .get("domain")
                        .and_then(|v| v.as_str())
                        .unwrap_or("general")
                        .to_string();
                    let tokens = session
                        .get("total_tokens")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0);
                    let cost = session
                        .get("total_cost")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.0);

                    self.stats.total_tokens = tokens;
                    self.stats.total_cost = cost;
                    if !domain.is_empty() {
                        self.domain = domain.clone();
                        self.stats.domain = domain;
                    }
                }
            }
            WsServerMessage::AgentList { agents } => {
                self.available_agents = agents
                    .iter()
                    .filter_map(|a| a.get("name").and_then(|v| v.as_str()).map(|s| s.to_string()))
                    .collect();
            }
            WsServerMessage::SkillList { skills } => {
                self.available_skills = skills
                    .iter()
                    .filter_map(|s| s.get("name").and_then(|v| v.as_str()).map(|s| s.to_string()))
                    .collect();
            }
            WsServerMessage::SessionList { sessions } => {
                self.sessions = sessions
                    .iter()
                    .filter_map(|s| {
                        Some(SessionInfo {
                            id: s.get("id").and_then(|v| v.as_str())?.to_string(),
                            domain: s.get("domain").and_then(|v| v.as_str()).unwrap_or("general").to_string(),
                            total_tokens: s.get("total_tokens").and_then(|v| v.as_u64()).unwrap_or(0),
                            total_cost: s.get("total_cost").and_then(|v| v.as_f64()).unwrap_or(0.0),
                            agent_count: s.get("agent_count").and_then(|v| v.as_u64()).unwrap_or(0) as usize,
                            completed_agents: s.get("completed_agents").and_then(|v| v.as_u64()).unwrap_or(0) as usize,
                            status: SessionStatus::Inactive,
                        })
                    })
                    .collect();
                self.sessions.truncate(20);
            }
            WsServerMessage::Error { message } => {
                match message.as_str() {
                    "connect" => {
                        self.connection_status = ConnectionState::Disconnected;
                        if !self.connection_fail_shown {
                            self.add_chat_message("system", "Cannot reach EvAgent core engine. Retrying...");
                            self.connection_fail_shown = true;
                        }
                    }
                    "reconnect" => {
                        self.connection_status = ConnectionState::Disconnected;
                        self.add_chat_message("system", "Connection lost. Reconnecting...");
                    }
                    _ => {
                        self.add_chat_message("system", &message);
                    }
                }
            }
        }
    }

    /// Auto-detect the current lifecycle phase from running agents.
    fn detect_lifecycle_phase(&mut self) {
        let running = self.active_agents.iter()
            .filter(|a| a.status == AgentState::Running)
            .collect::<Vec<_>>();

        if running.is_empty() {
            // Look at completed agents to determine phase
            let completed_count = self.active_agents.iter()
                .filter(|a| matches!(a.status, AgentState::Completed)).count();
            if completed_count > 0 && completed_count == self.active_agents.len() {
                self.lifecycle_phase = LifecyclePhase::Verifying;
            } else if self.active_agents.is_empty() {
                self.lifecycle_phase = LifecyclePhase::Thinking;
            }
            return;
        }

        // Check based on agent names
        for agent in &running {
            let name_lower = agent.agent_name.to_lowercase();
            if name_lower.contains("think") || name_lower.contains("analyze") {
                self.lifecycle_phase = LifecyclePhase::Thinking;
                return;
            }
            if name_lower.contains("plan") || name_lower.contains("architect") || name_lower.contains("design") {
                self.lifecycle_phase = LifecyclePhase::Planning;
                return;
            }
            if name_lower.contains("code") || name_lower.contains("write") || name_lower.contains("implement") {
                self.lifecycle_phase = LifecyclePhase::Executing;
                return;
            }
            if name_lower.contains("verif") || name_lower.contains("test") || name_lower.contains("review") {
                self.lifecycle_phase = LifecyclePhase::Verifying;
                return;
            }
            if name_lower.contains("merge") || name_lower.contains("deploy") || name_lower.contains("release") {
                self.lifecycle_phase = LifecyclePhase::Merging;
                return;
            }
        }

        self.lifecycle_phase = LifecyclePhase::Executing;
    }

    /// Update an existing agent card in the chat, or add a new one.
    fn update_agent_card(&mut self, agent_name: &str, progress_text: &str, progress_val: f32) {
        // Find the most recent message for this agent and update it
        if let Some(msg) = self.chat_messages.iter_mut()
            .rev()
            .find(|m| m.agent_name.as_deref() == Some(agent_name))
        {
            msg.content = progress_text.to_string();
            msg.agent_progress = Some(progress_val);
            // Update tools from the active_agents record
            if let Some(agent) = self.active_agents.iter()
                .find(|a| a.agent_name == agent_name)
            {
                msg.agent_tools = agent.tools_used.clone();
                msg.agent_diff = agent.diff_summary.clone();
            }
        } else {
            // Add a new card
            let tools = self.active_agents.iter()
                .find(|a| a.agent_name == agent_name)
                .map(|a| a.tools_used.clone())
                .unwrap_or_default();
            let diff = self.active_agents.iter()
                .find(|a| a.agent_name == agent_name)
                .map(|a| a.diff_summary.clone())
                .unwrap_or_default();

            self.chat_messages.push(ChatMessage::agent_card(
                agent_name,
                progress_text,
                progress_val,
                tools,
                diff,
            ));
            while self.chat_messages.len() > MAX_CHAT_HISTORY {
                self.chat_messages.remove(0);
            }
        }
        self.scroll_chat = true;
    }

    /// Add a message to the chat history.
    pub fn add_chat_message(&mut self, role: &str, content: &str) {
        self.chat_messages.push(ChatMessage::new(role, content));
        if self.chat_messages.len() > MAX_CHAT_HISTORY {
            self.chat_messages.remove(0);
        }
        self.scroll_chat = true;
    }

    /// Dispatch the current input as a task to the server.
    pub fn dispatch_prompt(&mut self, ws_send: &mpsc::UnboundedSender<String>) {
        let prompt = self.input.trim().to_string();
        if prompt.is_empty() {
            return;
        }

        self.add_chat_message("user", &prompt);
        self.active_agents.clear();

        // Clear previous agents when dispatching a new task
        self.active_agents.clear();

        let msg = serde_json::json!({
            "type": "DispatchTask",
            "goal": prompt,
            "context": null,
            "domain": self.domain,
        });

        let _ = ws_send.send(msg.to_string());

        // Start session timer
        self.session_start_time = Some(Instant::now());

        self.input.clear();
        self.input_cursor = 0;
    }

    /// Set the domain for the next dispatch.
    pub fn set_domain(&mut self, domain: &str) {
        self.domain = domain.to_string();
        self.stats.domain = domain.to_string();
    }

    /// Request initial data from server.
    pub fn request_initial_data(&mut self, ws_send: &mpsc::UnboundedSender<String>) {
        if !self.initialized {
            self.initialized = true;
            let _ = ws_send.send(r#"{"type":"AgentList"}"#.into());
            let _ = ws_send.send(r#"{"type":"SessionList"}"#.into());
            let _ = ws_send.send(r#"{"type":"SkillList"}"#.into());
        }
    }

    /// Update tick - called each frame for animations/progress.
    pub fn tick(&mut self) {
        if let Some(start) = self.session_start_time {
            self.runtime = start.elapsed();
        }
    }
}
