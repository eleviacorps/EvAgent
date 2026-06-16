#![allow(dead_code)]
use std::collections::HashMap;
use ratatui::style::Color;
use tokio::sync::mpsc;

use crate::types::*;

/// Maximum number of chat messages to keep.
const MAX_CHAT_HISTORY: usize = 200;
/// Maximum number of agent status cards to show.
const MAX_AGENT_DISPLAY: usize = 12;

/// Main application state.
pub struct App {
    /// Current domain
    pub domain: String,
    /// Recent session info
    pub sessions: Vec<SessionInfo>,
    /// Currently tracked agent statuses
    pub active_agents: Vec<AgentStatus>,
    /// Chat conversation history
    pub chat_messages: Vec<ChatMessage>,
    /// Current user text input buffer
    pub input: String,
    /// Cursor position in input
    pub input_cursor: usize,
    /// Aggregated session stats (tokens, cost, etc.)
    pub stats: SessionStats,
    /// WebSocket connection status
    pub connection_status: ConnectionState,
    /// Last dispatch result text (to show in chat)
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
    /// Track if connection failure message has been shown (to avoid spam)
    pub connection_fail_shown: bool,
}

impl Default for App {
    fn default() -> Self {
        let mut domain_colors = HashMap::new();
        domain_colors.insert("coding".into(), Color::Blue);
        domain_colors.insert("research".into(), Color::Green);
        domain_colors.insert("writing".into(), Color::Yellow);
        domain_colors.insert("trading".into(), Color::Magenta);
        domain_colors.insert("study".into(), Color::Cyan);
        domain_colors.insert("communication".into(), Color::LightMagenta);
        domain_colors.insert("media".into(), Color::Red);
        domain_colors.insert("general".into(), Color::White);

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
        }
    }
}

impl App {
    /// Create a new app with default state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Process an incoming server message and update state.
    pub fn update_from_ws(&mut self, msg: WsServerMessage) {
        match msg {
            WsServerMessage::Pong => {
                self.connection_status = ConnectionState::Connected;
                self.connection_fail_shown = false;
                // Request initial data on first connection
            }
            WsServerMessage::SubAgentUpdate {
                task_id,
                agent_name,
                status,
                progress,
                tokens_used,
                wall_clock_ms,
            } => {
                // Parse progress percentage from progress text if possible
                let progress_val = parse_progress(&progress);
                let agent_state = AgentState::from_str(&status);

                // Find existing or add new
                if let Some(existing) = self
                    .active_agents
                    .iter_mut()
                    .find(|a| a.task_id == task_id || (a.agent_name == agent_name && a.status == AgentState::Running))
                {
                    existing.status = agent_state.clone();
                    existing.progress = progress_val;
                    existing.progress_text = progress.clone();
                    existing.tokens_used = tokens_used;
                    existing.wall_clock_ms = wall_clock_ms;
                } else {
                    // Need to clone progress for the new AgentStatus since it's used later
                    let progress_clone = progress.clone();
                    self.active_agents.push(AgentStatus {
                        task_id,
                        agent_name: agent_name.clone(),
                        status: agent_state.clone(),
                        progress: progress_val,
                        progress_text: progress_clone,
                        tokens_used,
                        wall_clock_ms,
                    });
                }

                // Update aggregate stats
                self.stats.total_tokens = self
                    .active_agents
                    .iter()
                    .map(|a| a.tokens_used)
                    .sum();

                // Count completed
                self.stats.completed_agents = self
                    .active_agents
                    .iter()
                    .filter(|a| matches!(a.status, AgentState::Completed | AgentState::Failed | AgentState::Timeout))
                    .count();
                self.stats.total_agents = self.active_agents.len();

                // Limit displayed agents by removing old completed ones
                let running_ids: Vec<String> = self.active_agents.iter()
                    .filter(|a| a.status == AgentState::Running)
                    .map(|a| a.task_id.clone())
                    .collect();
                if self.active_agents.len() > MAX_AGENT_DISPLAY {
                    self.active_agents.retain(|a| {
                        a.status == AgentState::Running
                            || running_ids.contains(&a.task_id)
                    });
                }

                // Add a chat message for completion/failure
                if agent_state == AgentState::Completed {
                    self.add_chat_message(
                        "system",
                        &format!("✅ Agent **{}** completed — {} tokens in {}ms",
                            agent_name, tokens_used, wall_clock_ms),
                    );
                } else if agent_state == AgentState::Failed {
                    self.add_chat_message(
                        "system",
                        &format!("❌ Agent **{}** failed: {}", agent_name, progress),
                    );
                } else if agent_state == AgentState::Timeout {
                    self.add_chat_message(
                        "system",
                        &format!("⏱️ Agent **{}** timed out", agent_name),
                    );
                }
            }
            WsServerMessage::DispatchResult {
                session_id,
                outputs,
                aggregated,
            } => {
                self.aggregated_result = Some(aggregated.clone());
                self.stats.domain = self.domain.clone();

                // Add the result to chat
                self.add_chat_message("assistant", &format!("**Result (session {}):**\n{}", session_id, aggregated));

                // Log outputs summary
                if !outputs.is_empty() {
                    self.add_chat_message("system", &format!("📦 Received {} agent output(s)", outputs.len()));
                }

                // Mark all running agents as completed
                for agent in self.active_agents.iter_mut() {
                    if agent.status == AgentState::Running {
                        agent.status = AgentState::Completed;
                        agent.progress = 100.0;
                    }
                }
                self.stats.completed_agents = self.stats.total_agents;
            }
            WsServerMessage::SessionUpdate { session } => {
                // Try to extract useful fields from session JSON
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

                    // Update current session stats
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
                // Keep last 20 sessions
                self.sessions.truncate(20);
            }
            WsServerMessage::Error { message } => {
                match message.as_str() {
                    "connect" => {
                        self.connection_status = ConnectionState::Disconnected;
                        // Only set chat message once on first connection failure
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
                        // Show the error in chat but don't disconnect
                        self.add_chat_message("system", &message);
                    }
                }
            }
        }
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

        // Add the user message to chat
        self.add_chat_message("user", &prompt);

        // Build and send the dispatch message
        let msg = serde_json::json!({
            "type": "DispatchTask",
            "goal": prompt,
            "context": null,
            "domain": self.domain,
        });

        let _ = ws_send.send(msg.to_string());

        // Clear input
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
        // Nothing periodic needed currently
    }
}

/// Extract a percentage (0.0–100.0) from a progress string like "70%" or "searching... 55%".
fn parse_progress(text: &str) -> f32 {
    // Try to find a percentage number in the text
    if let Some(percent_str) = text.split('%').next() {
        if let Some(last_num) = percent_str.split_whitespace().last() {
            if let Ok(val) = last_num.parse::<f32>() {
                return val.clamp(0.0, 100.0);
            }
        }
    }
    // Fallback: try to find any number followed by %
    if let Some(pos) = text.find('%') {
        let before = &text[..pos];
        let num_part = before.split_whitespace().last().unwrap_or("");
        if let Ok(val) = num_part.parse::<f32>() {
            return val.clamp(0.0, 100.0);
        }
    }
    // Heuristic based on keywords
    let lower = text.to_lowercase();
    if lower.contains("complete") || lower.contains("done") || lower.contains("finished") {
        100.0
    } else if lower.contains("search") || lower.contains("research") {
        // random-ish estimate; actual progress comes from server
        50.0
    } else if lower.contains("start") || lower.contains("begin") {
        10.0
    } else {
        0.0
    }
}
