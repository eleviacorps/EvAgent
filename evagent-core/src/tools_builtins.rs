//! Built-in tool handlers: File / Execution / Web / Knowledge / LLM families.
//!
//! `register_builtins` wires them all into a ToolRegistry at startup.

use crate::errors::{EvAgentError, Result};
use crate::llm_client::LlmClient;
use crate::models::LlmMessage;
use crate::session::SessionStore;
use crate::tools::{Tool, ToolRegistry};
use serde_json::Value;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

pub fn register_builtins(
    reg: &ToolRegistry,
    llm: Arc<dyn LlmClient>,
    store: SessionStore,
    root: PathBuf,
) {
    reg.register(file_read_tool(root.clone()));
    reg.register(file_write_tool(root.clone()));
    reg.register(file_list_tool(root.clone()));
    reg.register(file_search_tool(root.clone()));
    reg.register(terminal_tool());
    reg.register(web_fetch_tool());
    reg.register(llm_complete_tool(llm.clone()));
    reg.register(memory_read_tool(store.clone()));
    reg.register(memory_write_tool(store.clone()));
    reg.register(session_search_tool(store.clone()));
}

fn file_read_tool(root: PathBuf) -> Tool {
    Tool {
        name: "ReadFile".into(),
        description: "Read file contents with pagination. Lines are 1-indexed.".into(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {"path": {"type": "string"}, "offset": {"type": "integer"}, "limit": {"type": "integer"}},
            "required": ["path"]
        }),
        domain: "core".into(),
        handler: Arc::new(move |args| {
            let root = root.clone();
            Box::pin(async move {
                let path = args["path"].as_str().ok_or_else(|| EvAgentError::ToolExecution("missing path".into()))?;
                let abs = resolve_under_root(&root, path)?;
                let offset = args["offset"].as_u64().unwrap_or(0) as usize;
                let limit = args["limit"].as_u64().map(|n| n as usize);
                let raw = tokio::fs::read_to_string(&abs).await
                    .map_err(|e| EvAgentError::ToolExecution(format!("read {}: {}", abs.display(), e)))?;
                let lines: Vec<&str> = raw.lines().collect();
                let start = offset.min(lines.len());
                let end = limit.map(|n| (start + n).min(lines.len())).unwrap_or(lines.len());
                Ok(serde_json::json!({
                    "path": path, "lines": (start + 1) as u64,
                    "content": lines[start..end].join("\n"), "total_lines": lines.len()
                }))
            })
        }),
    }
}

fn file_write_tool(root: PathBuf) -> Tool {
    Tool {
        name: "WriteFile".into(),
        description: "Write content to a file (overwrites entire file).".into(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {"path": {"type": "string"}, "content": {"type": "string"}},
            "required": ["path", "content"]
        }),
        domain: "core".into(),
        handler: Arc::new(move |args| {
            let root = root.clone();
            Box::pin(async move {
                let path = args["path"].as_str().ok_or_else(|| EvAgentError::ToolExecution("missing path".into()))?;
                let content = args["content"].as_str().ok_or_else(|| EvAgentError::ToolExecution("missing content".into()))?;
                let abs = resolve_under_root(&root, path)?;
                if let Some(parent) = abs.parent() { tokio::fs::create_dir_all(parent).await.ok(); }
                tokio::fs::write(&abs, content).await
                    .map_err(|e| EvAgentError::ToolExecution(format!("write: {}", e)))?;
                Ok(serde_json::json!({"path": path, "bytes": content.len()}))
            })
        }),
    }
}

fn file_list_tool(root: PathBuf) -> Tool {
    Tool {
        name: "ListDirectory".into(),
        description: "List directory contents, sorted by modification time.".into(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {"path": {"type": "string"}}, "required": ["path"]
        }),
        domain: "core".into(),
        handler: Arc::new(move |args| {
            let root = root.clone();
            Box::pin(async move {
                let path = args["path"].as_str().ok_or_else(|| EvAgentError::ToolExecution("missing path".into()))?;
                let abs = resolve_under_root(&root, path)?;
                let mut entries = Vec::new();
                let mut rd = tokio::fs::read_dir(&abs).await
                    .map_err(|e| EvAgentError::ToolExecution(format!("readdir: {}", e)))?;
                while let Some(e) = rd.next_entry().await
                    .map_err(|e| EvAgentError::ToolExecution(format!("readdir next: {}", e)))? {
                    let meta = e.metadata().await.ok();
                    let mtime = meta.as_ref()
                        .and_then(|m| m.modified().ok())
                        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                        .map(|d| d.as_secs()).unwrap_or(0);
                    entries.push(serde_json::json!({
                        "name": e.file_name().to_string_lossy(),
                        "is_dir": meta.as_ref().map(|m| m.is_dir()).unwrap_or(false),
                        "size": meta.as_ref().map(|m| m.len()).unwrap_or(0),
                        "mtime": mtime,
                    }));
                }
                entries.sort_by(|a, b| b["mtime"].as_u64().cmp(&a["mtime"].as_u64()));
                Ok(serde_json::json!({"path": path, "entries": entries}))
            })
        }),
    }
}

fn file_search_tool(root: PathBuf) -> Tool {
    Tool {
        name: "SearchFiles".into(),
        description: "Regex search inside files of a directory.".into(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {"pattern": {"type": "string"}, "path": {"type": "string"}},
            "required": ["pattern", "path"]
        }),
        domain: "core".into(),
        handler: Arc::new(move |args| {
            let root = root.clone();
            Box::pin(async move {
                let pattern = args["pattern"].as_str().ok_or_else(|| EvAgentError::ToolExecution("missing pattern".into()))?;
                let path = args["path"].as_str().ok_or_else(|| EvAgentError::ToolExecution("missing path".into()))?;
                let abs = resolve_under_root(&root, path)?;
                let re = regex::Regex::new(pattern)
                    .map_err(|e| EvAgentError::ToolExecution(format!("regex: {}", e)))?;
                let mut hits = Vec::new();
                for entry in walkdir::WalkDir::new(&abs).into_iter().filter_map(|e| e.ok()) {
                    if !entry.file_type().is_file() { continue; }
                    let p = entry.path();
                    if let Ok(raw) = std::fs::read_to_string(p) {
                        for (i, line) in raw.lines().enumerate() {
                            if re.is_match(line) {
                                hits.push(serde_json::json!({
                                    "file": p.strip_prefix(&root).unwrap_or(p).to_string_lossy(),
                                    "line": i + 1,
                                    "text": line.chars().take(200).collect::<String>()
                                }));
                                if hits.len() >= 100 { break; }
                            }
                        }
                    }
                    if hits.len() >= 100 { break; }
                }
                Ok(serde_json::json!({"pattern": pattern, "matches": hits, "truncated": hits.len() >= 100}))
            })
        }),
    }
}

fn terminal_tool() -> Tool {
    Tool {
        name: "Terminal".into(),
        description: "Execute a shell command. Returns stdout + exit code. 180s default timeout.".into(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {"command": {"type": "string"}, "timeout_secs": {"type": "integer"}},
            "required": ["command"]
        }),
        domain: "core".into(),
        handler: Arc::new(|args| Box::pin(async move {
            let command = args["command"].as_str().ok_or_else(|| EvAgentError::ToolExecution("missing command".into()))?;
            let timeout = Duration::from_secs(args["timeout_secs"].as_u64().unwrap_or(180));
            let out = tokio::time::timeout(timeout, tokio::process::Command::new("sh").arg("-c").arg(command).output())
                .await.map_err(|_| EvAgentError::Timeout(timeout.as_secs()))?
                .map_err(|e| EvAgentError::ToolExecution(format!("spawn: {}", e)))?;
            Ok(serde_json::json!({
                "exit_code": out.status.code().unwrap_or(-1),
                "stdout": String::from_utf8_lossy(&out.stdout),
                "stderr": String::from_utf8_lossy(&out.stderr)
            }))
        })),
    }
}

fn web_fetch_tool() -> Tool {
    Tool {
        name: "WebFetch".into(),
        description: "HTTP GET request. Returns body text. For JSON/plain-text endpoints.".into(),
        parameters: serde_json::json!({"type": "object", "properties": {"url": {"type": "string"}}, "required": ["url"]}),
        domain: "core".into(),
        handler: Arc::new(|args| Box::pin(async move {
            let url = args["url"].as_str().ok_or_else(|| EvAgentError::ToolExecution("missing url".into()))?;
            let client = reqwest::Client::builder().timeout(Duration::from_secs(30)).build()
                .map_err(|e| EvAgentError::ToolExecution(format!("http client: {}", e)))?;
            let resp = client.get(url).send().await
                .map_err(|e| EvAgentError::ToolExecution(format!("http get: {}", e)))?;
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            Ok(serde_json::json!({"status": status.as_u16(), "body": body.chars().take(50_000).collect::<String>()}))
        })),
    }
}

fn llm_complete_tool(llm: Arc<dyn LlmClient>) -> Tool {
    Tool {
        name: "LLMComplete".into(),
        description: "Direct LLM call for sub-agent self-queries.".into(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {"prompt": {"type": "string"}, "max_tokens": {"type": "integer"}},
            "required": ["prompt"]
        }),
        domain: "core".into(),
        handler: Arc::new(move |args| {
            let llm = llm.clone();
            Box::pin(async move {
                let prompt = args["prompt"].as_str().ok_or_else(|| EvAgentError::ToolExecution("missing prompt".into()))?;
                let msgs = vec![LlmMessage { role: "user".into(), content: prompt.into() }];
                let resp = llm.complete(msgs).await?;
                Ok(serde_json::json!({
                    "content": resp.choices.first().map(|c| c.message.content.clone()).unwrap_or_default(),
                    "tokens": resp.usage.total_tokens
                }))
            })
        }),
    }
}

fn memory_read_tool(store: SessionStore) -> Tool {
    Tool {
        name: "MemoryRead".into(),
        description: "Read entries from persistent memory. Pass null key for all.".into(),
        parameters: serde_json::json!({"type": "object", "properties": {"key": {"type": "string"}}}),
        domain: "core".into(),
        handler: Arc::new(move |args| {
            let store = store.clone();
            Box::pin(async move {
                let key = args.get("key").and_then(|v| v.as_str());
                let rows = store.memory_read(key).await?;
                let arr: Vec<Value> = rows.into_iter()
                    .map(|(k, c, kind)| serde_json::json!({"key": k, "content": c, "kind": kind}))
                    .collect();
                Ok(serde_json::json!({"entries": arr}))
            })
        }),
    }
}

fn memory_write_tool(store: SessionStore) -> Tool {
    Tool {
        name: "MemoryWrite".into(),
        description: "Write an entry to persistent memory.".into(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {
                "key": {"type": "string"}, "content": {"type": "string"},
                "kind": {"type": "string", "enum": ["user", "agent"]}
            },
            "required": ["key", "content"]
        }),
        domain: "core".into(),
        handler: Arc::new(move |args| {
            let store = store.clone();
            Box::pin(async move {
                let key = args["key"].as_str().ok_or_else(|| EvAgentError::ToolExecution("missing key".into()))?;
                let content = args["content"].as_str().ok_or_else(|| EvAgentError::ToolExecution("missing content".into()))?;
                let kind = args["kind"].as_str().unwrap_or("agent");
                store.memory_write(key, content, kind).await?;
                Ok(serde_json::json!({"ok": true}))
            })
        }),
    }
}

fn session_search_tool(store: SessionStore) -> Tool {
    Tool {
        name: "SessionSearch".into(),
        description: "FTS5 search across all past sessions.".into(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {"query": {"type": "string"}, "limit": {"type": "integer"}},
            "required": ["query"]
        }),
        domain: "core".into(),
        handler: Arc::new(move |args| {
            let store = store.clone();
            Box::pin(async move {
                let query = args["query"].as_str().ok_or_else(|| EvAgentError::ToolExecution("missing query".into()))?;
                let limit = args["limit"].as_u64().unwrap_or(10) as u32;
                let rows = store.search_messages(query, limit).await?;
                let arr: Vec<Value> = rows.into_iter().map(|r| serde_json::json!({
                    "id": r.id, "domain": r.domain, "tokens": r.total_tokens,
                    "summary": r.summary, "created_at": r.created_at
                })).collect();
                Ok(serde_json::json!({"sessions": arr}))
            })
        }),
    }
}

/// Resolve `path` (which may be relative) under `root`, rejecting escapes.
fn resolve_under_root(root: &PathBuf, path: &str) -> Result<PathBuf> {
    let p = PathBuf::from(path);
    let abs = if p.is_absolute() { p } else { root.join(p) };
    let parent_ok = abs.parent().map(|p| p.starts_with(root) || p == root).unwrap_or(true);
    if !abs.starts_with(root) && !parent_ok {
        return Err(EvAgentError::ToolExecution(format!("path escapes root: {}", path)));
    }
    Ok(abs)
}
