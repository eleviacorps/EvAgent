//! Agent registry — scans `domains/*/agents/*.yaml` and exposes lookups.

use crate::errors::Result;
use crate::models::AgentDef;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use walkdir::WalkDir;

#[derive(Clone, Default)]
pub struct AgentRegistry {
    agents: Arc<RwLock<HashMap<String, AgentDef>>>,
}

impl AgentRegistry {
    pub fn new() -> Self {
        Self {
            agents: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn scan(&self, root: &Path) -> Result<usize> {
        let base = root.join("domains");
        let mut loaded = 0usize;
        if !base.exists() {
            tracing::warn!("agents dir not found: {}", base.display());
            return Ok(0);
        }
        for entry in WalkDir::new(&base)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.file_type().is_file()
                    && (e.file_name().to_string_lossy().ends_with(".yaml")
                        || e.file_name().to_string_lossy().ends_with(".yml"))
            })
            .filter(|e| {
                e.path()
                    .parent()
                    .map(|p| p.file_name().map(|n| n == "agents").unwrap_or(false))
                    .unwrap_or(false)
            })
        {
            match std::fs::read_to_string(entry.path()) {
                Ok(raw) => match serde_yaml::from_str::<AgentDef>(&raw) {
                    Ok(a) => {
                        let key = format!("{}/{}", a.domain, a.name);
                        tracing::debug!("loaded agent: {}", key);
                        self.agents.write().insert(key, a);
                        loaded += 1;
                    }
                    Err(e) => {
                        tracing::warn!("skip agent {}: {}", entry.path().display(), e);
                    }
                },
                Err(e) => {
                    tracing::warn!("read agent {}: {}", entry.path().display(), e);
                }
            }
        }
        tracing::info!("loaded {} agents from {}", loaded, base.display());
        Ok(loaded)
    }

    pub fn get(&self, domain: &str, name: &str) -> Option<AgentDef> {
        self.agents
            .read()
            .get(&format!("{}/{}", domain, name))
            .cloned()
    }

    pub fn for_domain(&self, domain: &str) -> Vec<AgentDef> {
        self.agents
            .read()
            .values()
            .filter(|a| a.domain == domain)
            .cloned()
            .collect()
    }

    pub fn all(&self) -> Vec<AgentDef> {
        self.agents.read().values().cloned().collect()
    }

    pub fn count(&self) -> usize {
        self.agents.read().len()
    }
}
