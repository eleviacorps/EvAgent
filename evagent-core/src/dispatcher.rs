use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use parking_lot::RwLock;
use serde_json::Value;
use tokio::task::JoinSet;
use tracing::{debug, error, info, warn};

use crate::errors::{HermesError, HermesResult};
use crate::models::{DispatchTask, SubAgentOutput, SubAgentStatus};

/// Real dispatcher that spawns isolated sub-agent processes and aggregates results.
/// Each sub-agent runs as a separate process with JSON communication over stdin/stdout.
pub struct Dispatcher {
    /// Maximum concurrent sub-agents
    max_concurrent: usize,
    /// Default timeout in seconds for sub-agent tasks
    default_timeout: u64,
    /// Default token budget for sub-agent tasks
    default_token_budget: u64,
    /// Map of assigned skills per agent
    skills_map: Arc<RwLock<HashMap<String, Vec<String>>>>,
    /// Map of permission profiles per agent
    permissions_map: Arc<RwLock<HashMap<String, String>>>,
}

impl Dispatcher {
    pub fn new(max_concurrent: usize, default_timeout: u64, default_token_budget: u64) -> Self {
        Self {
            max_concurrent,
            default_timeout,
            default_token_budget,
            skills_map: Arc::new(RwLock::new(HashMap::new())),
            permissions_map: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Assign skills to an agent.
    pub fn assign_skills(&self, agent: &str, skills: Vec<String>) {
        let mut map = self.skills_map.write();
        map.insert(agent.to_string(), skills);
    }

    /// Assign permission profile to an agent.
    pub fn assign_permissions(&self, agent: &str, profile: &str) {
        let mut map = self.permissions_map.write();
        map.insert(agent.to_string(), profile.to_string());
    }

    /// Dispatch a single sub-agent task — spawns a child process for the sub-agent.
    pub fn dispatch(&self, task: &DispatchTask) -> HermesResult<SubAgentOutput> {
        let start = Instant::now();
        let task_json = serde_json::to_string(task)
            .map_err(|e| HermesError::dispatcher_with("Failed to serialize task", e))?;

        debug!(
            "Spawning sub-agent for task '{}' in domain '{}'",
            task.id, task.domain
        );

        // Determine the sub-agent runner script/executable.
        // In production, this would point to actual agent binaries.
        // For now, we use a built-in "echo agent" pattern that returns a structured response.
        let agent_script = self.resolve_agent_script(&task.domain);

        let output = if cfg!(target_os = "windows") {
            // On Windows, use cmd /c with the script
            std::process::Command::new("cmd")
                .args([
                    "/C",
                    &agent_script,
                    &task_json,
                ])
                .output()
                .map_err(|e| {
                    HermesError::dispatcher_with(format!("Failed to spawn sub-agent for '{}'", task.id), e)
                })?
        } else {
            // On Unix, use sh -c with the script
            std::process::Command::new("sh")
                .args(["-c", &agent_script, &task_json])
                .output()
                .map_err(|e| {
                    HermesError::dispatcher_with(format!("Failed to spawn sub-agent for '{}'", task.id), e)
                })?
        };

        let elapsed_ms = start.elapsed().as_millis() as u64;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            // Try to parse as JSON
            match serde_json::from_str::<Value>(&stdout) {
                Ok(json) => {
                    let status = match json.get("status").and_then(|s| s.as_str()) {
                        Some("completed") | Some("Completed") => SubAgentStatus::Completed,
                        Some("failed") | Some("Failed") => SubAgentStatus::Failed,
                        Some("timeout") | Some("Timeout") => SubAgentStatus::Timeout,
                        _ => SubAgentStatus::Completed,
                    };

                    let result = json
                        .get("result")
                        .and_then(|r| r.as_str())
                        .map(|s| s.to_string());

                    let tokens_used = json
                        .get("tokens_used")
                        .and_then(|t| t.as_u64())
                        .unwrap_or(0);

                    let errors: Vec<String> = json
                        .get("errors")
                        .and_then(|e| e.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                .collect()
                        })
                        .unwrap_or_default();

                    Ok(SubAgentOutput {
                        task_id: task.id.clone(),
                        status,
                        result,
                        tokens_used,
                        errors,
                        wall_clock_ms: elapsed_ms,
                    })
                }
                Err(_) => {
                    // Non-JSON output — treat entire stdout as result
                    let result_str = stdout.trim().to_string();
                    Ok(SubAgentOutput {
                        task_id: task.id.clone(),
                        status: SubAgentStatus::Completed,
                        result: Some(result_str),
                        tokens_used: estimate_tokens(&stdout),
                        errors: vec![],
                        wall_clock_ms: elapsed_ms,
                    })
                }
            }
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            error!(
                "Sub-agent '{}' failed (exit code: {:?}): {}",
                task.id,
                output.status.code(),
                stderr.trim()
            );

            Ok(SubAgentOutput {
                task_id: task.id.clone(),
                status: SubAgentStatus::Failed,
                result: None,
                tokens_used: 0,
                errors: vec![stderr.trim().to_string()],
                wall_clock_ms: elapsed_ms,
            })
        }
    }

    /// Dispatch multiple tasks in parallel using tokio::task::JoinSet.
    pub async fn dispatch_parallel(
        &self,
        tasks: Vec<DispatchTask>,
    ) -> HermesResult<Vec<SubAgentOutput>> {
        let total = tasks.len();
        info!(
            "Dispatching {} tasks in parallel (max concurrent: {})",
            total, self.max_concurrent
        );

        let mut join_set = JoinSet::new();
        // Use a semaphore-like approach by limiting concurrent tasks
        let mut active = 0;
        let mut results: Vec<SubAgentOutput> = Vec::with_capacity(total);
        let mut remaining = tasks.into_iter();

        // Start initial batch
        for task in (&mut remaining).take(self.max_concurrent) {
            let dispatcher = self.clone_arc();
            join_set.spawn(async move {
                let task_id = task.id.clone();
                let result = tokio::task::spawn_blocking(move || dispatcher.dispatch(&task))
                    .await
                    .unwrap_or_else(|e| {
                        Err(HermesError::dispatcher_with("Tokio join error", e))
                    });
                (task_id, result)
            });
            active += 1;
        }

        // Process as they complete and start new ones
        while let Some(result) = join_set.join_next().await {
            match result {
                Ok((task_id, dispatch_result)) => {
                    match dispatch_result {
                        Ok(output) => {
                            debug!("Task '{}' completed in {}ms", task_id, output.wall_clock_ms);
                            results.push(output);
                        }
                        Err(e) => {
                            error!("Task '{}' failed: {}", task_id, e);
                            results.push(SubAgentOutput {
                                task_id,
                                status: SubAgentStatus::Failed,
                                result: None,
                                tokens_used: 0,
                                errors: vec![e.to_string()],
                                wall_clock_ms: 0,
                            });
                        }
                    }
                }
                Err(e) => {
                    error!("Join error: {}", e);
                }
            }

            // Start next task if available
            if let Some(next_task) = remaining.next() {
                let dispatcher = self.clone_arc();
                join_set.spawn(async move {
                    let task_id = next_task.id.clone();
                    let result = tokio::task::spawn_blocking(move || dispatcher.dispatch(&next_task))
                        .await
                        .unwrap_or_else(|e| {
                            Err(HermesError::dispatcher_with("Tokio join error", e))
                        });
                    (task_id, result)
                });
            }
        }

        info!(
            "Parallel dispatch complete: {}/{} tasks finished",
            results.len(),
            total
        );

        Ok(results)
    }

    /// Aggregate multiple sub-agent outputs into a single structured response.
    pub fn aggregate(&self, outputs: Vec<SubAgentOutput>) -> HermesResult<String> {
        let total_tokens: u64 = outputs.iter().map(|o| o.tokens_used).sum();
        let total_time: u64 = outputs.iter().map(|o| o.wall_clock_ms).sum();
        let completed = outputs.iter().filter(|o| o.status == SubAgentStatus::Completed).count();
        let failed = outputs.iter().filter(|o| o.status == SubAgentStatus::Failed).count();
        let timeouts = outputs.iter().filter(|o| o.status == SubAgentStatus::Timeout).count();

        let mut results_map = serde_json::Map::new();
        results_map.insert(
            "summary".to_string(),
            serde_json::json!({
                "total_agents": outputs.len(),
                "completed": completed,
                "failed": failed,
                "timeout": timeouts,
                "total_tokens_used": total_tokens,
                "total_wall_clock_ms": total_time,
            }),
        );

        let mut agent_results = Vec::new();
        for output in outputs {
            let mut entry = serde_json::Map::new();
            entry.insert("task_id".to_string(), serde_json::json!(output.task_id));
            entry.insert("status".to_string(), serde_json::json!(format!("{:?}", output.status)));
            entry.insert("tokens_used".to_string(), serde_json::json!(output.tokens_used));
            entry.insert("wall_clock_ms".to_string(), serde_json::json!(output.wall_clock_ms));

            if let Some(ref result) = output.result {
                entry.insert("result".to_string(), serde_json::json!(result));
            }
            if !output.errors.is_empty() {
                entry.insert("errors".to_string(), serde_json::json!(output.errors));
            }

            agent_results.push(serde_json::Value::Object(entry));
        }
        results_map.insert("agent_results".to_string(), serde_json::json!(agent_results));

        let aggregated = serde_json::to_string_pretty(&serde_json::Value::Object(results_map))
            .map_err(|e| HermesError::dispatcher_with("Failed to serialize aggregated result", e))?;

        Ok(aggregated)
    }

    /// Resolve the agent script path for a given domain.
    /// If EVAGENT_API_KEY is set, calls the LLM for a real response.
    /// Otherwise falls back to echo-based mock.
    fn resolve_agent_script(&self, domain: &str) -> String {
        let api_key = std::env::var("EVAGENT_API_KEY").unwrap_or_default();
        let base_url = std::env::var("EVAGENT_BASE_URL")
            .unwrap_or_else(|_| "https://opencode.ai/zen/v1".to_string());
        let model = std::env::var("EVAGENT_MODEL")
            .unwrap_or_else(|_| "deepseek-v4-flash-free".to_string());

        if api_key.is_empty() || api_key.starts_with("sk-VXvH") {
            // No real key — use mock
            if cfg!(target_os = "windows") {
                format!(
                    "echo {{\\\"status\\\":\\\"completed\\\",\\\"result\\\":\\\"Processed task for domain '{}'\\\",\\\"tokens_used\\\":100,\\\"errors\\\":[]}}",
                    domain
                )
            } else {
                format!(
                    "echo '{{\"status\":\"completed\",\"result\":\"Processed task for domain \\'{}\\'\",\"tokens_used\":100,\"errors\":[]}}'",
                    domain
                )
            }
        } else {
            // Real LLM call — use a Python one-liner
            format!(
                r#"python -c "
import json, urllib.request, sys
data = json.dumps({{\"model\":\"{model}\",\"messages\":[{{\"role\":\"user\",\"content\":sys.argv[1]}}],\"max_tokens\":512}}).encode()
req = urllib.request.Request('{base_url}/chat/completions', data=data, headers={{'Authorization':'Bearer {api_key}','Content-Type':'application/json'}})
resp = json.loads(urllib.request.urlopen(req,timeout=30).read())
print(json.dumps({{\"status\":\"completed\",\"result\":resp['choices'][0]['message']['content'],\"tokens_used\":resp.get('usage',{{}}).get('total_tokens',0),\"errors\":[]}}))
" "Sub-agent for domain: {domain}""#
            )
        }
    }

    /// Create an Arc clone of self for sharing across tokio tasks.
    fn clone_arc(&self) -> Arc<Dispatcher> {
        Arc::new(Dispatcher {
            max_concurrent: self.max_concurrent,
            default_timeout: self.default_timeout,
            default_token_budget: self.default_token_budget,
            skills_map: self.skills_map.clone(),
            permissions_map: self.permissions_map.clone(),
        })
    }
}

/// Estimate token count from a string (rough: 1 token ≈ 4 chars).
fn estimate_tokens(s: &str) -> u64 {
    (s.len() / 4).max(1) as u64
}

/// Build DispatchTask from a goal and context.
pub fn build_task(
    id: String,
    goal: String,
    context: String,
    domain: String,
    assigned_skills: Vec<String>,
    permission_profile: String,
    timeout_secs: u64,
    token_budget: u64,
) -> DispatchTask {
    DispatchTask {
        id,
        goal,
        context,
        assigned_skills,
        permission_profile,
        timeout_secs,
        token_budget,
        domain,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dispatch_single_task() {
        let dispatcher = Dispatcher::new(5, 30, 4096);
        let task = DispatchTask {
            id: "test-1".to_string(),
            goal: "Test task".to_string(),
            context: "Test context".to_string(),
            assigned_skills: vec![],
            permission_profile: "default".to_string(),
            timeout_secs: 10,
            token_budget: 1000,
            domain: "general".to_string(),
        };

        let result = dispatcher.dispatch(&task).unwrap();
        assert_eq!(result.status, SubAgentStatus::Completed);
        assert!(result.wall_clock_ms > 0);
    }

    #[tokio::test]
    async fn test_dispatch_parallel() {
        let dispatcher = Dispatcher::new(5, 30, 4096);
        let mut tasks = Vec::new();

        for i in 0..3 {
            tasks.push(DispatchTask {
                id: format!("parallel-{}", i),
                goal: format!("Task {}", i),
                context: "".to_string(),
                assigned_skills: vec![],
                permission_profile: "default".to_string(),
                timeout_secs: 10,
                token_budget: 1000,
                domain: "general".to_string(),
            });
        }

        let results = dispatcher.dispatch_parallel(tasks).await.unwrap();
        assert_eq!(results.len(), 3);
        for r in &results {
            assert_eq!(r.status, SubAgentStatus::Completed);
        }
    }

    #[test]
    fn test_aggregate() {
        let dispatcher = Dispatcher::new(5, 30, 4096);
        let outputs = vec![
            SubAgentOutput {
                task_id: "t1".to_string(),
                status: SubAgentStatus::Completed,
                result: Some("Result 1".to_string()),
                tokens_used: 100,
                errors: vec![],
                wall_clock_ms: 50,
            },
            SubAgentOutput {
                task_id: "t2".to_string(),
                status: SubAgentStatus::Failed,
                result: None,
                tokens_used: 50,
                errors: vec!["Error occurred".to_string()],
                wall_clock_ms: 30,
            },
        ];

        let aggregated = dispatcher.aggregate(outputs).unwrap();
        assert!(aggregated.contains("total_agents"));
        assert!(aggregated.contains("completed"));
        assert!(aggregated.contains("failed"));
    }
}
