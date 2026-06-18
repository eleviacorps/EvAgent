//! Tool registry and shared types. Handler implementations live in
//! `tools_builtins.rs`.

use crate::errors::{EvAgentError, Result};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

pub type ToolHandler =
    Arc<dyn Fn(Value) -> futures_util::future::BoxFuture<'static, Result<Value>> + Send + Sync>;

#[derive(Clone)]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub parameters: Value,
    pub handler: ToolHandler,
    pub domain: String,
}

#[derive(Clone, Default)]
pub struct ToolRegistry {
    tools: Arc<RwLock<HashMap<String, Tool>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub name: String,
    pub arguments: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub name: String,
    pub ok: bool,
    pub output: Value,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    pub fn register(&self, tool: Tool) {
        self.tools.write().insert(tool.name.clone(), tool);
    }
    pub fn get(&self, name: &str) -> Option<Tool> {
        self.tools.read().get(name).cloned()
    }
    pub fn list(&self) -> Vec<Tool> {
        self.tools.read().values().cloned().collect()
    }
    pub async fn invoke(&self, call: ToolCall) -> Result<ToolResult> {
        let tool = self
            .get(&call.name)
            .ok_or_else(|| EvAgentError::ToolExecution(format!("unknown tool: {}", call.name)))?;
        let fut = (tool.handler)(call.arguments);
        match fut.await {
            Ok(v) => Ok(ToolResult { name: call.name, ok: true, output: v }),
            Err(e) => Ok(ToolResult { name: call.name, ok: false, output: Value::String(e.to_string()) }),
        }
    }
}
