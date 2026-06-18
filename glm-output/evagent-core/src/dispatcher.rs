//! Parallel sub-agent dispatch engine.
//!
//! Per spec:
//! - All sub-agents run concurrently using `tokio::task::JoinSet`.
//! - Max 5 concurrent (configurable).
//! - Progress updates broadcast during execution.
//! - On timeout (default 120s): mark agent as Failed.
//! - On all complete: aggregate and return.

use crate::agent_registry::AgentRegistry;
use crate::config::Config;
use crate::errors::Result;
use crate::llm_client::LlmClient;
use crate::memory::Memory;
use crate::models::{
    AgentDef, AgentStatus, DispatchTask, LlmMessage, ServerMessage, SubAgentOutput,
};
use crate::permissions::PermissionEngine;
use crate::skill_loader::SkillLoader;
use crate::tools::ToolRegistry;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::task::JoinSet;
use uuid::Uuid;

pub struct Dispatcher {
    pub cfg: Arc<Config>,
    pub registry: AgentRegistry,
    pub skills: SkillLoader,
    pub permissions: PermissionEngine,
    pub tools: ToolRegistry,
    pub memory: Arc<Memory>,
    pub llm: Arc<dyn LlmClient>,
}

impl Dispatcher {
    pub fn new(
        cfg: Arc<Config>,
        registry: AgentRegistry,
        skills: SkillLoader,
        permissions: PermissionEngine,
        tools: ToolRegistry,
        memory: Arc<Memory>,
        llm: Arc<dyn LlmClient>,
    ) -> Self {
        Self {
            cfg,
            registry,
            skills,
            permissions,
            tools,
            memory,
            llm,
        }
    }

    /// Dispatch a task. Spawns all matching agents in parallel, streams
    /// SubAgentUpdate events through `tx`, returns the final aggregated outputs.
    pub async fn dispatch(
        &self,
        task: DispatchTask,
        tx: broadcast::Sender<ServerMessage>,
    ) -> Result<Vec<SubAgentOutput>> {
        let agents = self.select_agents(&task.domain);
        if agents.is_empty() {
            // Fall back to a synthetic "general-assistant" agent so the user
            // always gets a response even for unmatched domains.
            let fallback = AgentDef {
                name: "general-assistant".into(),
                role: "executor".into(),
                domain: task.domain.clone(),
                tools: vec![],
                token_budget: self.cfg.dispatch.default_token_budget,
                skills: vec![],
                system_prompt: None,
            };
            return self.run_agents(vec![fallback], task, tx).await;
        }
        self.run_agents(agents, task, tx).await
    }

    fn select_agents(&self, domain: &str) -> Vec<AgentDef> {
        let mut agents = self.registry.for_domain(domain);
        // Sort by role priority: planner → architect → executor → reviewer → tester → specialist
        agents.sort_by_key(|a| role_priority(&a.role));
        agents.truncate(self.cfg.core.max_concurrent_agents as usize);
        agents
    }

    async fn run_agents(
        &self,
        agents: Vec<AgentDef>,
        task: DispatchTask,
        tx: broadcast::Sender<ServerMessage>,
    ) -> Result<Vec<SubAgentOutput>> {
        let mut set: JoinSet<Result<SubAgentOutput>> = JoinSet::new();
        let timeout = std::time::Duration::from_secs(self.cfg.core.default_timeout_secs);

        for agent in agents.iter().cloned() {
            let task_id = Uuid::new_v4().to_string();
            let _ = tx.send(ServerMessage::SubAgentUpdate {
                task_id: task_id.clone(),
                agent_name: agent.name.clone(),
                status: AgentStatus::Running,
                progress: "starting".into(),
                tokens_used: 0,
            });

            let llm = self.llm.clone();
            let skills = self.skills.clone();
            let memory = self.memory.clone();
            let task_goal = task.goal.clone();
            let task_context = task.context.clone();
            let task_domain = task.domain.clone();
            let session_id = task.session_id;
            let agent_clone = agent.clone();
            let token_budget = agent.token_budget;
            let tx_clone = tx.clone();
            let task_id_clone = task_id.clone();

            set.spawn(async move {
                run_single_agent(
                    &agent_clone,
                    &task_goal,
                    task_context.as_ref(),
                    &task_domain,
                    session_id,
                    token_budget,
                    &skills,
                    &memory,
                    &llm,
                    &tx_clone,
                    &task_id_clone,
                )
                .await
            });
        }

        let mut outputs = Vec::new();
        while let Some(res) = tokio::time::timeout(timeout, set.join_next()).await.ok() {
            match res {
                Some(Ok(Ok(out))) => {
                    let _ = tx.send(ServerMessage::SubAgentUpdate {
                        task_id: out.task_id.clone(),
                        agent_name: out.agent_name.clone(),
                        status: out.status,
                        progress: format!("done in {} ms", out.wall_clock_ms),
                        tokens_used: out.tokens_used,
                    });
                    outputs.push(out);
                }
                Some(Ok(Err(e))) => {
                    tracing::warn!("agent failed: {}", e);
                    let _ = tx.send(ServerMessage::Error {
                        message: e.to_string(),
                    });
                }
                Some(Err(e)) => {
                    tracing::warn!("join error: {}", e);
                }
                None => break,
            }
        }
        // Any agents still in the set timed out — mark them failed.
        while let Some(_aborted) = set.join_next().await {
            let _ = tx.send(ServerMessage::Error {
                message: "agent timed out".into(),
            });
        }
        Ok(outputs)
    }
}

fn role_priority(role: &str) -> u8 {
    match role.to_lowercase().as_str() {
        "planner" => 0,
        "architect" => 1,
        "executor" => 2,
        "reviewer" => 3,
        "tester" => 4,
        "specialist" => 5,
        _ => 9,
    }
}

#[allow(clippy::too_many_arguments)]
async fn run_single_agent(
    agent: &AgentDef,
    goal: &str,
    context: Option<&serde_json::Value>,
    domain: &str,
    session_id: Uuid,
    token_budget: u32,
    skills: &SkillLoader,
    memory: &Memory,
    llm: &Arc<dyn LlmClient>,
    tx: &broadcast::Sender<ServerMessage>,
    task_id: &str,
) -> Result<SubAgentOutput> {
    let started = std::time::Instant::now();
    let _ = tx.send(ServerMessage::SubAgentUpdate {
        task_id: task_id.into(),
        agent_name: agent.name.clone(),
        status: AgentStatus::Running,
        progress: "loading skills".into(),
        tokens_used: 0,
    });

    // Compose system prompt: agent's own prompt + memory + skills.
    let memory_block = memory.context_block(domain);
    let domain_skills = skills.for_domain(domain);
    let trigger_skills = skills.matching_triggers(goal);
    let mut skill_blocks = Vec::new();
    for s in &domain_skills {
        skill_blocks.push(format!("# Skill: {} (v{})\n\n{}", s.name, s.version, s.body));
    }
    for s in &trigger_skills {
        if !domain_skills.iter().any(|d| d.name == s.name) {
            skill_blocks.push(format!(
                "# Skill (trigger-match): {} (v{})\n\n{}",
                s.name, s.version, s.body
            ));
        }
    }
    let skills_block = skill_blocks.join("\n\n---\n\n");

    let system_prompt = format!(
        "# Agent: {} (role: {})\n\n{}\n\n{}\n\n{}",
        agent.name,
        agent.role,
        agent.system_prompt.clone().unwrap_or_default(),
        memory_block,
        skills_block
    );

    let user_content = if let Some(ctx) = context {
        format!(
            "Goal: {}\n\nDomain: {}\n\nContext: {}\n\nToken budget: {}",
            goal, domain, ctx, token_budget
        )
    } else {
        format!(
            "Goal: {}\n\nDomain: {}\n\nToken budget: {}",
            goal, domain, token_budget
        )
    };

    let _ = tx.send(ServerMessage::SubAgentUpdate {
        task_id: task_id.into(),
        agent_name: agent.name.clone(),
        status: AgentStatus::Running,
        progress: "calling LLM".into(),
        tokens_used: 0,
    });

    let msgs = vec![
        LlmMessage {
            role: "system".into(),
            content: system_prompt,
        },
        LlmMessage {
            role: "user".into(),
            content: user_content,
        },
    ];

    let resp = llm.complete(msgs).await?;
    let output_text = resp
        .choices
        .first()
        .map(|c| c.message.content.clone())
        .unwrap_or_default();

    let _ = tx.send(ServerMessage::SubAgentUpdate {
        task_id: task_id.into(),
        agent_name: agent.name.clone(),
        status: AgentStatus::Running,
        progress: "storing result".into(),
        tokens_used: resp.usage.total_tokens,
    });

    // Persist message to session store via memory's store.
    let session_str = session_id.to_string();
    memory
        .write_agent_note(
            &format!("session:{}:{}", session_str, agent.name),
            &output_text.chars().take(2000).collect::<String>(),
        )
        .await
        .ok();

    let wall = started.elapsed().as_millis();
    Ok(SubAgentOutput {
        task_id: task_id.into(),
        agent_name: agent.name.clone(),
        output: output_text,
        tokens_used: resp.usage.total_tokens,
        wall_clock_ms: wall,
        status: AgentStatus::Completed,
    })
}
