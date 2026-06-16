use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use tracing::{debug, info, warn};

use crate::errors::{HermesError, HermesResult};
use crate::models::AgentIndexEntry;

/// Cache entry with TTL eviction.
struct CacheEntry {
    agent: AgentIndexEntry,
    loaded_at: Instant,
}

const CACHE_TTL: Duration = Duration::from_secs(300);

/// Agent registry backed by SQLite with an in-memory cache.
pub struct AgentRegistry {
    db: Arc<Mutex<rusqlite::Connection>>,
    cache: Arc<Mutex<HashMap<String, CacheEntry>>>,
    agent_paths: Arc<Mutex<Vec<PathBuf>>>,
    last_scan_mtime: Arc<Mutex<Option<DateTime<Utc>>>>,
    max_walk_depth: u32,
}

impl AgentRegistry {
    pub fn new(conn: Arc<Mutex<rusqlite::Connection>>, max_walk_depth: u32) -> HermesResult<Self> {
        let registry = Self {
            db: conn.clone(),
            cache: Arc::new(Mutex::new(HashMap::new())),
            agent_paths: Arc::new(Mutex::new(Vec::new())),
            last_scan_mtime: Arc::new(Mutex::new(None)),
            max_walk_depth,
        };
        registry.init_schema()?;
        Ok(registry)
    }

    fn init_schema(&self) -> HermesResult<()> {
        let db = self.db.lock().map_err(|e| HermesError::store(e.to_string()))?;
        db.execute_batch(
            "CREATE TABLE IF NOT EXISTS agent_registry (
                name TEXT PRIMARY KEY,
                domain TEXT NOT NULL,
                description TEXT NOT NULL DEFAULT '',
                tool_scope TEXT NOT NULL DEFAULT '[]',
                permission_profile TEXT NOT NULL DEFAULT 'default',
                source_path TEXT NOT NULL,
                last_modified TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_agent_domain ON agent_registry(domain);",
        )
        .map_err(|e| HermesError::store_with("Failed to init agent registry schema", e))?;
        Ok(())
    }

    pub fn register_scan_paths(&self, paths: &[PathBuf]) {
        if let Ok(mut agent_paths) = self.agent_paths.lock() {
            for p in paths {
                if !agent_paths.contains(p) {
                    agent_paths.push(p.clone());
                }
            }
        }
        debug!("Registered {} agent scan paths", paths.len());
    }

    pub fn scan_and_register(&self) -> HermesResult<usize> {
        let paths = self.agent_paths.lock()
            .map_err(|e| HermesError::store(e.to_string()))?
            .clone();
        let mut total = 0;

        for base_path in &paths {
            if !base_path.exists() {
                debug!("Agent path does not exist, skipping: {:?}", base_path);
                continue;
            }
            total += self.scan_directory(base_path, 0)?;
        }

        let mut last_mtime = self.last_scan_mtime.lock()
            .map_err(|e| HermesError::store(e.to_string()))?;
        *last_mtime = Some(Utc::now());

        info!("Agent scan complete: {} agents registered", total);
        Ok(total)
    }

    fn scan_directory(&self, dir: &Path, depth: u32) -> HermesResult<usize> {
        if depth > self.max_walk_depth {
            return Ok(0);
        }

        let mut count = 0;
        let entries = std::fs::read_dir(dir)
            .map_err(|e| HermesError::io_with(format!("Cannot read directory: {:?}", dir), e))?;

        for entry in entries {
            let entry = entry.map_err(|e| HermesError::io_with("Directory read error", e))?;
            let path = entry.path();

            if path.is_dir() {
                count += self.scan_directory(&path, depth + 1)?;
            } else if path.extension().map_or(false, |ext| ext == "yaml" || ext == "yml") {
                match self.register_path(&path) {
                    Ok(true) => count += 1,
                    Ok(false) => {}
                    Err(e) => warn!("Failed to register agent from {:?}: {}", path, e),
                }
            }
        }

        Ok(count)
    }

    pub fn register_path(&self, path: &Path) -> HermesResult<bool> {
        let contents = std::fs::read_to_string(path)
            .map_err(|e| HermesError::io_with(format!("Cannot read agent file: {:?}", path), e))?;

        let agent: crate::models::AgentDefinition = serde_yaml::from_str(&contents)
            .map_err(|e| HermesError::config_with(format!("Invalid agent YAML: {:?}", path), e))?;

        let metadata = std::fs::metadata(path)
            .map_err(|e| HermesError::io_with("Cannot read metadata", e))?;

        let last_modified: DateTime<Utc> = metadata
            .modified()
            .map(|t| {
                let duration = t.duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
                DateTime::from_timestamp(duration.as_secs() as i64, 0).unwrap_or(Utc::now())
            })
            .unwrap_or(Utc::now());

        let path_str = path.to_string_lossy().to_string();

        let db = self.db.lock().map_err(|e| HermesError::store(e.to_string()))?;
        db.execute(
            "INSERT OR REPLACE INTO agent_registry (name, domain, description, tool_scope, permission_profile, source_path, last_modified)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                agent.name,
                agent.domain,
                agent.description,
                serde_json::to_string(&agent.tool_scope).map_err(|e| HermesError::store_with("Serialize error", e))?,
                agent.permission_profile,
                path_str,
                last_modified.to_rfc3339(),
            ],
        )
        .map_err(|e| HermesError::store_with("Failed to register agent", e))?;

        let mut cache = self.cache.lock().map_err(|e| HermesError::store(e.to_string()))?;
        cache.remove(&agent.name);

        debug!("Registered agent '{}' (domain: {})", agent.name, agent.domain);
        Ok(true)
    }

    pub fn get(&self, name: &str) -> HermesResult<AgentIndexEntry> {
        {
            let cache = self.cache.lock().map_err(|e| HermesError::store(e.to_string()))?;
            if let Some(entry) = cache.get(name) {
                if entry.loaded_at.elapsed() < CACHE_TTL {
                    return Ok(entry.agent.clone());
                }
            }
        }

        let db = self.db.lock().map_err(|e| HermesError::store(e.to_string()))?;
        let mut stmt = db
            .prepare(
                "SELECT name, domain, description, tool_scope, permission_profile, source_path, last_modified
                 FROM agent_registry WHERE name = ?1",
            )
            .map_err(|e| HermesError::store_with("Failed to prepare agent query", e))?;

        let result = stmt.query_row(rusqlite::params![name], |row| {
            let tool_scope_str: String = row.get(3)?;
            let lm_str: String = row.get(6)?;
            let tool_scope: Vec<String> = serde_json::from_str(&tool_scope_str).unwrap_or_default();
            let last_modified = DateTime::parse_from_rfc3339(&lm_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());

            Ok(AgentIndexEntry {
                name: row.get(0)?,
                domain: row.get(1)?,
                description: row.get(2)?,
                tool_scope,
                permission_profile: row.get(4)?,
                source_path: row.get(5)?,
                last_modified,
            })
        });

        match result {
            Ok(agent) => {
                let mut cache = self.cache.lock().map_err(|e| HermesError::store(e.to_string()))?;
                cache.insert(
                    name.to_string(),
                    CacheEntry {
                        agent: agent.clone(),
                        loaded_at: Instant::now(),
                    },
                );
                Ok(agent)
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                Err(HermesError::agent(format!("Agent '{}' not found", name)))
            }
            Err(e) => Err(HermesError::store_with("Failed to query agent", e)),
        }
    }

    pub fn list(&self, domain_filter: Option<&str>) -> HermesResult<Vec<AgentIndexEntry>> {
        let db = self.db.lock().map_err(|e| HermesError::store(e.to_string()))?;
        let (query, params): (String, Vec<Box<dyn rusqlite::types::ToSql>>) = match domain_filter {
            Some(domain) => (
                "SELECT name, domain, description, tool_scope, permission_profile, source_path, last_modified
                 FROM agent_registry WHERE domain = ?1 ORDER BY name".to_string(),
                vec![Box::new(domain.to_string()) as Box<dyn rusqlite::types::ToSql>],
            ),
            None => (
                "SELECT name, domain, description, tool_scope, permission_profile, source_path, last_modified
                 FROM agent_registry ORDER BY domain, name".to_string(),
                vec![],
            ),
        };

        let mut stmt = db
            .prepare(&query)
            .map_err(|e| HermesError::store_with("Failed to prepare list query", e))?;

        let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();

        let agents = stmt
            .query_map(param_refs.as_slice(), |row| {
                let tool_scope_str: String = row.get(3)?;
                let lm_str: String = row.get(6)?;
                let tool_scope: Vec<String> = serde_json::from_str(&tool_scope_str).unwrap_or_default();
                let last_modified = DateTime::parse_from_rfc3339(&lm_str)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now());

                Ok(AgentIndexEntry {
                    name: row.get(0)?,
                    domain: row.get(1)?,
                    description: row.get(2)?,
                    tool_scope,
                    permission_profile: row.get(4)?,
                    source_path: row.get(5)?,
                    last_modified,
                })
            })
            .map_err(|e| HermesError::store_with("Failed to query agents", e))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(agents)
    }

    pub fn search(&self, query: &str) -> HermesResult<Vec<AgentIndexEntry>> {
        let pattern = format!("%{}%", query);
        let db = self.db.lock().map_err(|e| HermesError::store(e.to_string()))?;
        let mut stmt = db
            .prepare(
                "SELECT name, domain, description, tool_scope, permission_profile, source_path, last_modified
                 FROM agent_registry
                 WHERE name LIKE ?1 OR description LIKE ?1
                 ORDER BY CASE WHEN name LIKE ?2 THEN 0 ELSE 1 END, name",
            )
            .map_err(|e| HermesError::store_with("Failed to prepare search query", e))?;

        let pattern_prefix = format!("{}%", query);

        let agents = stmt
            .query_map(rusqlite::params![pattern, pattern_prefix], |row| {
                let tool_scope_str: String = row.get(3)?;
                let lm_str: String = row.get(6)?;
                let tool_scope: Vec<String> = serde_json::from_str(&tool_scope_str).unwrap_or_default();
                let last_modified = DateTime::parse_from_rfc3339(&lm_str)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now());

                Ok(AgentIndexEntry {
                    name: row.get(0)?,
                    domain: row.get(1)?,
                    description: row.get(2)?,
                    tool_scope,
                    permission_profile: row.get(4)?,
                    source_path: row.get(5)?,
                    last_modified,
                })
            })
            .map_err(|e| HermesError::store_with("Failed to search agents", e))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(agents)
    }

    pub fn evict_cache(&self) {
        if let Ok(mut cache) = self.cache.lock() {
            let before = cache.len();
            cache.retain(|_, entry| entry.loaded_at.elapsed() < CACHE_TTL);
            let after = cache.len();
            if before != after {
                debug!("Evicted {} stale entries from agent cache", before - after);
            }
        }
    }

    pub fn check_and_rescan(&self) -> HermesResult<usize> {
        let paths = self.agent_paths.lock()
            .map_err(|e| HermesError::store(e.to_string()))?
            .clone();
        let mut needs_rescan = false;

        for base_path in &paths {
            if base_path.exists() {
                if let Ok(metadata) = std::fs::metadata(base_path) {
                    if let Ok(modified) = metadata.modified() {
                        let duration = modified
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default();
                        let mtime = DateTime::from_timestamp(duration.as_secs() as i64, 0)
                            .unwrap_or(Utc::now());

                        let last = *self.last_scan_mtime.lock()
                            .map_err(|e| HermesError::store(e.to_string()))?;
                        if let Some(last_mtime) = last {
                            if mtime > last_mtime {
                                needs_rescan = true;
                            }
                        } else {
                            needs_rescan = true;
                        }
                    }
                }
            }
        }

        if needs_rescan {
            self.scan_and_register()
        } else {
            Ok(0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_and_get() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        let registry = AgentRegistry::new(Arc::new(Mutex::new(conn)), 3).unwrap();

        let dir = std::env::temp_dir().join("hermes_test_agents");
        std::fs::create_dir_all(&dir).unwrap();
        let agent_path = dir.join("test_agent.yaml");
        let yaml = r#"
name: test_agent
domain: general
description: A test agent
tool_scope:
  - search
permission_profile: default
"#;
        std::fs::write(&agent_path, yaml).unwrap();

        registry.register_path(&agent_path).unwrap();
        let agent = registry.get("test_agent").unwrap();
        assert_eq!(agent.name, "test_agent");
        assert_eq!(agent.domain, "general");

        std::fs::remove_dir_all(&dir).unwrap();
    }
}
