//! Tool/action permission engine.
//!
//! Each tool has a permission level (`allow`, `restrict`, `deny`). Each agent
//! profile lists which restricted tools it has been granted. The permission
//! engine answers `can(agent, tool)` queries.

use crate::models::AgentDef;
use parking_lot::RwLock;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Permission {
    Allow,
    Restrict,
    Deny,
}

#[derive(Clone, Default)]
pub struct PermissionEngine {
    defaults: Arc<RwLock<HashMap<String, Permission>>>,
    grants: Arc<RwLock<HashMap<String, HashSet<String>>>>,
}

impl PermissionEngine {
    pub fn new() -> Self {
        let mut defaults = HashMap::new();
        // File tools — read is open, write is restricted, patch is restricted.
        defaults.insert("ReadFile".into(), Permission::Allow);
        defaults.insert("ListDirectory".into(), Permission::Allow);
        defaults.insert("SearchFiles".into(), Permission::Allow);
        defaults.insert("WriteFile".into(), Permission::Restrict);
        defaults.insert("PatchFile".into(), Permission::Restrict);
        // Execution tools — always restricted by default.
        defaults.insert("Terminal".into(), Permission::Restrict);
        defaults.insert("PythonCode".into(), Permission::Restrict);
        defaults.insert("BackgroundProcess".into(), Permission::Restrict);
        // Web tools — open by default (network access is controlled separately).
        defaults.insert("WebFetch".into(), Permission::Allow);
        defaults.insert("WebSearch".into(), Permission::Allow);
        defaults.insert("WebExtract".into(), Permission::Allow);
        // Knowledge tools — open.
        defaults.insert("SkillSearch".into(), Permission::Allow);
        defaults.insert("MemoryRead".into(), Permission::Allow);
        defaults.insert("MemoryWrite".into(), Permission::Restrict);
        defaults.insert("SessionSearch".into(), Permission::Allow);
        // LLM tools — open.
        defaults.insert("LLMComplete".into(), Permission::Allow);
        defaults.insert("LLMEmbed".into(), Permission::Allow);

        Self {
            defaults: Arc::new(RwLock::new(defaults)),
            grants: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn register_agent(&self, agent: &AgentDef) {
        let mut grants = self.grants.write();
        grants.insert(
            format!("{}/{}", agent.domain, agent.name),
            agent.tools.iter().cloned().collect(),
        );
    }

    pub fn can(&self, agent_key: &str, tool: &str) -> bool {
        let defaults = self.defaults.read();
        let perm = defaults.get(tool).copied().unwrap_or(Permission::Deny);
        match perm {
            Permission::Allow => true,
            Permission::Deny => false,
            Permission::Restrict => {
                let grants = self.grants.read();
                grants
                    .get(agent_key)
                    .map(|set| set.contains(tool))
                    .unwrap_or(false)
            }
        }
    }

    pub fn list_for(&self, agent_key: &str) -> Vec<String> {
        let defaults = self.defaults.read();
        let mut out: Vec<String> = defaults
            .iter()
            .filter(|(_, p)| **p == Permission::Allow)
            .map(|(k, _)| k.clone())
            .collect();
        if let Some(granted) = self.grants.read().get(agent_key) {
            for g in granted {
                if !out.contains(g) {
                    out.push(g.clone());
                }
            }
        }
        out.sort();
        out
    }
}
