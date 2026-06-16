use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tracing::{debug, info, warn};

use crate::errors::{HermesError, HermesResult};
use crate::models::SkillDefinition;

/// Cache entry with version tracking.
struct SkillCacheEntry {
    skill: SkillDefinition,
    loaded_at: Instant,
}

const SKILL_CACHE_TTL: Duration = Duration::from_secs(300);

/// Skill loader: loads/validates YAML frontmatter from SKILL.md files and caches them.
pub struct SkillLoader {
    db: Arc<Mutex<rusqlite::Connection>>,
    cache: Arc<Mutex<HashMap<String, SkillCacheEntry>>>,
    max_walk_depth: u32,
}

impl SkillLoader {
    pub fn new(conn: Arc<Mutex<rusqlite::Connection>>, max_walk_depth: u32) -> HermesResult<Self> {
        let loader = Self {
            db: conn,
            cache: Arc::new(Mutex::new(HashMap::new())),
            max_walk_depth,
        };
        loader.init_schema()?;
        Ok(loader)
    }

    fn init_schema(&self) -> HermesResult<()> {
        let db = self.db.lock().map_err(|e| HermesError::store(e.to_string()))?;
        db.execute_batch(
            "CREATE TABLE IF NOT EXISTS skill_index (
                name TEXT PRIMARY KEY,
                domain TEXT NOT NULL,
                trigger_patterns TEXT NOT NULL DEFAULT '[]',
                applicable_agents TEXT NOT NULL DEFAULT '[]',
                steps TEXT NOT NULL DEFAULT '[]',
                examples TEXT NOT NULL DEFAULT '[]',
                anti_patterns TEXT NOT NULL DEFAULT '[]',
                version INTEGER NOT NULL DEFAULT 1,
                source_path TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_skill_domain ON skill_index(domain);",
        )
        .map_err(|e| HermesError::store_with("Failed to init skill schema", e))?;
        Ok(())
    }

    pub fn load(&self, path: &Path) -> HermesResult<SkillDefinition> {
        let contents = std::fs::read_to_string(path)
            .map_err(|e| HermesError::io_with(format!("Cannot read skill file: {:?}", path), e))?;

        // Parse YAML frontmatter (between --- and ---)
        let trimmed = contents.trim();
        if !trimmed.starts_with("---") {
            return Err(HermesError::skill(format!(
                "Missing YAML frontmatter in SKILL.md: {:?}. Must start with '---'",
                path
            )));
        }

        let without_first = trimmed.trim_start_matches("---").trim();
        let end_marker = match without_first.find("\n---") {
            Some(pos) => &without_first[..pos],
            None => {
                return Err(HermesError::skill(format!(
                    "Unclosed YAML frontmatter in SKILL.md: {:?}",
                    path
                )));
            }
        };

        let yaml_str = end_marker.trim();
        let mut skill: SkillDefinition = serde_yaml::from_str(yaml_str)
            .map_err(|e| HermesError::skill_with(format!("Invalid YAML frontmatter in {:?}", path), e))?;

        if skill.name.is_empty() {
            if let Some(stem) = path.file_stem() {
                skill.name = stem.to_string_lossy().to_string();
            }
        }

        let path_str = path.to_string_lossy().to_string();
        let db = self.db.lock().map_err(|e| HermesError::store(e.to_string()))?;
        db.execute(
            "INSERT OR REPLACE INTO skill_index (name, domain, trigger_patterns, applicable_agents, steps, examples, anti_patterns, version, source_path)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            rusqlite::params![
                skill.name,
                skill.domain,
                serde_json::to_string(&skill.trigger_patterns).map_err(|e| HermesError::store_with("Serialize error", e))?,
                serde_json::to_string(&skill.applicable_agents).map_err(|e| HermesError::store_with("Serialize error", e))?,
                serde_json::to_string(&skill.steps).map_err(|e| HermesError::store_with("Serialize error", e))?,
                serde_json::to_string(&skill.examples).map_err(|e| HermesError::store_with("Serialize error", e))?,
                serde_json::to_string(&skill.anti_patterns).map_err(|e| HermesError::store_with("Serialize error", e))?,
                skill.version,
                path_str,
            ],
        )
        .map_err(|e| HermesError::store_with("Failed to index skill", e))?;

        let mut cache = self.cache.lock().map_err(|e| HermesError::store(e.to_string()))?;
        cache.insert(
            skill.name.clone(),
            SkillCacheEntry {
                skill: skill.clone(),
                loaded_at: Instant::now(),
            },
        );

        debug!("Loaded skill '{}' (v{}, domain: {})", skill.name, skill.version, skill.domain);
        Ok(skill)
    }

    pub fn get(&self, name: &str) -> HermesResult<SkillDefinition> {
        {
            let cache = self.cache.lock().map_err(|e| HermesError::store(e.to_string()))?;
            if let Some(entry) = cache.get(name) {
                if entry.loaded_at.elapsed() < SKILL_CACHE_TTL {
                    return Ok(entry.skill.clone());
                }
            }
        }

        let db = self.db.lock().map_err(|e| HermesError::store(e.to_string()))?;
        let mut stmt = db
            .prepare(
                "SELECT name, domain, trigger_patterns, applicable_agents, steps, examples, anti_patterns, version, source_path
                 FROM skill_index WHERE name = ?1",
            )
            .map_err(|e| HermesError::store_with("Failed to prepare skill query", e))?;

        let result = stmt.query_row(rusqlite::params![name], |row| {
            let tp_str: String = row.get(2)?;
            let aa_str: String = row.get(3)?;
            let steps_str: String = row.get(4)?;
            let ex_str: String = row.get(5)?;
            let ap_str: String = row.get(6)?;

            Ok(SkillDefinition {
                name: row.get(0)?,
                domain: row.get(1)?,
                trigger_patterns: serde_json::from_str(&tp_str).unwrap_or_default(),
                applicable_agents: serde_json::from_str(&aa_str).unwrap_or_default(),
                steps: serde_json::from_str(&steps_str).unwrap_or_default(),
                examples: serde_json::from_str(&ex_str).unwrap_or_default(),
                anti_patterns: serde_json::from_str(&ap_str).unwrap_or_default(),
                version: row.get(7)?,
            })
        });

        match result {
            Ok(skill) => {
                let mut cache = self.cache.lock().map_err(|e| HermesError::store(e.to_string()))?;
                cache.insert(
                    name.to_string(),
                    SkillCacheEntry {
                        skill: skill.clone(),
                        loaded_at: Instant::now(),
                    },
                );
                Ok(skill)
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                Err(HermesError::skill(format!("Skill '{}' not found", name)))
            }
            Err(e) => Err(HermesError::store_with("Failed to query skill", e)),
        }
    }

    pub fn search(&self, query: &str, domain_filter: Option<&str>) -> HermesResult<Vec<SkillDefinition>> {
        let db = self.db.lock().map_err(|e| HermesError::store(e.to_string()))?;
        let pattern = format!("%{}%", query);

        let (sql, params): (String, Vec<Box<dyn rusqlite::types::ToSql>>) = match domain_filter {
            Some(domain) => (
                "SELECT name, domain, trigger_patterns, applicable_agents, steps, examples, anti_patterns, version
                 FROM skill_index
                 WHERE domain = ?1 AND (name LIKE ?2 OR trigger_patterns LIKE ?2 OR steps LIKE ?2)
                 ORDER BY name".to_string(),
                vec![
                    Box::new(domain.to_string()) as Box<dyn rusqlite::types::ToSql>,
                    Box::new(pattern) as Box<dyn rusqlite::types::ToSql>,
                ],
            ),
            None => (
                "SELECT name, domain, trigger_patterns, applicable_agents, steps, examples, anti_patterns, version
                 FROM skill_index
                 WHERE name LIKE ?1 OR trigger_patterns LIKE ?1 OR steps LIKE ?1
                 ORDER BY name".to_string(),
                vec![Box::new(pattern) as Box<dyn rusqlite::types::ToSql>],
            ),
        };

        let mut stmt = db
            .prepare(&sql)
            .map_err(|e| HermesError::store_with("Failed to prepare skill search", e))?;

        let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();

        let skills = stmt
            .query_map(param_refs.as_slice(), |row| {
                let tp_str: String = row.get(2)?;
                let aa_str: String = row.get(3)?;
                let steps_str: String = row.get(4)?;
                let ex_str: String = row.get(5)?;
                let ap_str: String = row.get(6)?;

                Ok(SkillDefinition {
                    name: row.get(0)?,
                    domain: row.get(1)?,
                    trigger_patterns: serde_json::from_str(&tp_str).unwrap_or_default(),
                    applicable_agents: serde_json::from_str(&aa_str).unwrap_or_default(),
                    steps: serde_json::from_str(&steps_str).unwrap_or_default(),
                    examples: serde_json::from_str(&ex_str).unwrap_or_default(),
                    anti_patterns: serde_json::from_str(&ap_str).unwrap_or_default(),
                    version: row.get(7)?,
                })
            })
            .map_err(|e| HermesError::store_with("Failed to search skills", e))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(skills)
    }

    pub fn list(&self, domain_filter: Option<&str>) -> HermesResult<Vec<SkillDefinition>> {
        let db = self.db.lock().map_err(|e| HermesError::store(e.to_string()))?;
        let (sql, params): (String, Vec<Box<dyn rusqlite::types::ToSql>>) = match domain_filter {
            Some(domain) => (
                "SELECT name, domain, trigger_patterns, applicable_agents, steps, examples, anti_patterns, version
                 FROM skill_index WHERE domain = ?1 ORDER BY name".to_string(),
                vec![Box::new(domain.to_string()) as Box<dyn rusqlite::types::ToSql>],
            ),
            None => (
                "SELECT name, domain, trigger_patterns, applicable_agents, steps, examples, anti_patterns, version
                 FROM skill_index ORDER BY domain, name".to_string(),
                vec![],
            ),
        };

        let mut stmt = db
            .prepare(&sql)
            .map_err(|e| HermesError::store_with("Failed to prepare skill list", e))?;

        let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();

        let skills = stmt
            .query_map(param_refs.as_slice(), |row| {
                let tp_str: String = row.get(2)?;
                let aa_str: String = row.get(3)?;
                let steps_str: String = row.get(4)?;
                let ex_str: String = row.get(5)?;
                let ap_str: String = row.get(6)?;

                Ok(SkillDefinition {
                    name: row.get(0)?,
                    domain: row.get(1)?,
                    trigger_patterns: serde_json::from_str(&tp_str).unwrap_or_default(),
                    applicable_agents: serde_json::from_str(&aa_str).unwrap_or_default(),
                    steps: serde_json::from_str(&steps_str).unwrap_or_default(),
                    examples: serde_json::from_str(&ex_str).unwrap_or_default(),
                    anti_patterns: serde_json::from_str(&ap_str).unwrap_or_default(),
                    version: row.get(7)?,
                })
            })
            .map_err(|e| HermesError::store_with("Failed to list skills", e))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(skills)
    }

    pub fn reload_index(&self, skill_paths: &[PathBuf]) -> HermesResult<usize> {
        let mut count = 0;
        for base_path in skill_paths {
            count += self.scan_for_skills(base_path, 0)?;
        }
        info!("Skill index reloaded: {} skills loaded", count);
        Ok(count)
    }

    fn scan_for_skills(&self, dir: &Path, depth: u32) -> HermesResult<usize> {
        if depth > self.max_walk_depth {
            return Ok(0);
        }

        let mut count = 0;
        if !dir.exists() {
            return Ok(0);
        }

        let entries = std::fs::read_dir(dir)
            .map_err(|e| HermesError::io_with(format!("Cannot read directory: {:?}", dir), e))?;

        for entry in entries {
            let entry = entry.map_err(|e| HermesError::io_with("Directory read error", e))?;
            let path = entry.path();

            if path.is_dir() {
                count += self.scan_for_skills(&path, depth + 1)?;
            } else if path.file_name().map_or(false, |f| f == "SKILL.md") {
                match self.load(&path) {
                    Ok(_) => count += 1,
                    Err(e) => warn!("Failed to load skill from {:?}: {}", path, e),
                }
            }
        }

        Ok(count)
    }

    pub fn evict_cache(&self) {
        if let Ok(mut cache) = self.cache.lock() {
            let before = cache.len();
            cache.retain(|_, entry| entry.loaded_at.elapsed() < SKILL_CACHE_TTL);
            let after = cache.len();
            if before != after {
                debug!("Evicted {} stale entries from skill cache", before - after);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_skill_from_yaml_frontmatter() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        let loader = SkillLoader::new(Arc::new(Mutex::new(conn)), 3).unwrap();

        let dir = std::env::temp_dir().join("hermes_test_skills");
        std::fs::create_dir_all(&dir).unwrap();
        let skill_path = dir.join("SKILL.md");

        let content = r#"---
name: code_review
domain: engineering
trigger_patterns:
  - "review this code"
  - "code review"
applicable_agents:
  - "code_reviewer"
steps:
  - "Analyze the code for issues"
  - "Provide improvement suggestions"
version: 2
---

# Code Review Skill

This skill handles code review requests.
"#;
        std::fs::write(&skill_path, content).unwrap();

        let skill = loader.load(&skill_path).unwrap();
        assert_eq!(skill.name, "code_review");
        assert_eq!(skill.domain, "engineering");
        assert_eq!(skill.version, 2);
        assert!(skill.trigger_patterns.contains(&"review this code".to_string()));

        let cached = loader.get("code_review").unwrap();
        assert_eq!(cached.name, "code_review");

        std::fs::remove_dir_all(&dir).unwrap();
    }
}
