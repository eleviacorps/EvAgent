//! Skill loader — scans `domains/*/skills/*.md` at startup, parses YAML
//! frontmatter + markdown body, caches results in memory.

use crate::errors::Result;
use crate::models::Skill;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Clone, Default)]
pub struct SkillLoader {
    skills: Arc<RwLock<HashMap<String, Skill>>>,
}

use std::sync::Arc;

impl SkillLoader {
    pub fn new() -> Self {
        Self {
            skills: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Scan `<root>/domains/*/skills/*.md` and load every skill file.
    pub fn scan(&self, root: &Path) -> Result<usize> {
        let skills_dir = root.join("domains");
        let mut loaded = 0usize;
        if !skills_dir.exists() {
            tracing::warn!("skills dir not found: {}", skills_dir.display());
            return Ok(0);
        }
        for entry in WalkDir::new(&skills_dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file() && e.file_name().to_string_lossy().ends_with(".md"))
        {
            match parse_skill_file(entry.path()) {
                Ok(skill) => {
                    tracing::debug!("loaded skill: {}/{}", skill.domain, skill.name);
                    self.skills
                        .write()
                        .insert(format!("{}/{}", skill.domain, skill.name), skill);
                    loaded += 1;
                }
                Err(e) => {
                    tracing::warn!("skip skill {}: {}", entry.path().display(), e);
                }
            }
        }
        tracing::info!("loaded {} skills from {}", loaded, skills_dir.display());
        Ok(loaded)
    }

    /// Get all skills for a domain (always-injected per spec).
    pub fn for_domain(&self, domain: &str) -> Vec<Skill> {
        self.skills
            .read()
            .values()
            .filter(|s| s.domain == domain)
            .cloned()
            .collect()
    }

    /// Get skills whose `triggers` match any keyword in the prompt.
    pub fn matching_triggers(&self, prompt: &str) -> Vec<Skill> {
        let lower = prompt.to_lowercase();
        self.skills
            .read()
            .values()
            .filter(|s| {
                s.triggers
                    .iter()
                    .any(|t| lower.contains(&t.to_lowercase()))
            })
            .cloned()
            .collect()
    }

    pub fn get(&self, domain: &str, name: &str) -> Option<Skill> {
        self.skills
            .read()
            .get(&format!("{}/{}", domain, name))
            .cloned()
    }

    pub fn all(&self) -> Vec<Skill> {
        self.skills.read().values().cloned().collect()
    }

    pub fn count(&self) -> usize {
        self.skills.read().len()
    }
}

fn parse_skill_file(path: &Path) -> Result<Skill> {
    let raw = std::fs::read_to_string(path)?;
    let (frontmatter, body) = split_frontmatter(&raw);
    let mut skill: Skill = if frontmatter.is_empty() {
        // No frontmatter — synthesize minimal metadata from filename.
        let name = path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "unnamed".into());
        let domain = path
            .parent()
            .and_then(|p| p.parent())
            .and_then(|p| p.file_name())
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "general".into());
        Skill {
            name,
            version: 1,
            domain,
            description: String::new(),
            triggers: Vec::new(),
            body: body.clone(),
        }
    } else {
        let mut s: Skill = serde_yaml::from_str(frontmatter)?;
        s.body = body.clone();
        s
    };
    if skill.body.is_empty() {
        skill.body = body;
    }
    Ok(skill)
}

/// Split a markdown file into (yaml_frontmatter, body).
/// Frontmatter is delimited by leading `---\n ... \n---\n`.
fn split_frontmatter(raw: &str) -> (&str, String) {
    let trimmed = raw.trim_start();
    if !trimmed.starts_with("---") {
        return ("", raw.to_string());
    }
    let after_first = &trimmed[3..];
    let end = match after_first.find("\n---") {
        Some(i) => i,
        None => return ("", raw.to_string()),
    };
    let front = &after_first[..end];
    let body_start = end + 4; // skip "\n---"
    let body = after_first[body_start..].trim_start_matches('\n').to_string();
    (front, body)
}

#[allow(dead_code)]
pub fn skill_paths(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let base = root.join("domains");
    if !base.exists() {
        return out;
    }
    for entry in WalkDir::new(&base)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file() && e.file_name().to_string_lossy().ends_with(".md"))
    {
        out.push(entry.path().to_path_buf());
    }
    out
}
