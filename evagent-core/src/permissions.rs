use rusqlite::Connection;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::errors::{HermesError, HermesResult};
use crate::models::{FilesystemAccessLevel, PermissionProfile};
use tracing::{debug, info, warn};

/// Thread-safe permission engine backed by SQLite with in-memory cache.
pub struct PermissionEngine {
    db: Arc<Mutex<Connection>>,
    cache: Arc<Mutex<HashMap<String, PermissionProfile>>>,
}

impl PermissionEngine {
    pub fn new(conn: Arc<Mutex<Connection>>) -> HermesResult<Self> {
        let engine = Self {
            db: conn,
            cache: Arc::new(Mutex::new(HashMap::new())),
        };
        engine.init_schema()?;
        if engine.get_profile("default").is_err() {
            let default_profile = PermissionProfile {
                name: "default".to_string(),
                allow_tools: vec![],
                allow_domains: vec![],
                max_tokens: None,
                timeout_secs: None,
                network_access: false,
                filesystem_access_level: FilesystemAccessLevel::None,
            };
            engine.create_profile(default_profile)?;
            info!("Created default deny-all permission profile");
        }
        Ok(engine)
    }

    fn init_schema(&self) -> HermesResult<()> {
        let db = self.db.lock().map_err(|e| HermesError::store(e.to_string()))?;
        db.execute_batch(
            "CREATE TABLE IF NOT EXISTS permission_profiles (
                name TEXT PRIMARY KEY,
                allow_tools TEXT NOT NULL DEFAULT '[]',
                allow_domains TEXT NOT NULL DEFAULT '[]',
                max_tokens INTEGER,
                timeout_secs INTEGER,
                network_access INTEGER NOT NULL DEFAULT 0,
                filesystem_access_level TEXT NOT NULL DEFAULT 'None'
            );
            CREATE TABLE IF NOT EXISTS permission_grants (
                agent_name TEXT NOT NULL,
                action TEXT NOT NULL,
                resource TEXT NOT NULL,
                granted INTEGER NOT NULL DEFAULT 1,
                PRIMARY KEY (agent_name, action, resource)
            );",
        )
        .map_err(|e| HermesError::store_with("Failed to init permissions schema", e))?;
        Ok(())
    }

    pub fn create_profile(&self, profile: PermissionProfile) -> HermesResult<()> {
        let db = self.db.lock().map_err(|e| HermesError::store(e.to_string()))?;
        db.execute(
            "INSERT INTO permission_profiles (name, allow_tools, allow_domains, max_tokens, timeout_secs, network_access, filesystem_access_level)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                profile.name,
                serde_json::to_string(&profile.allow_tools).map_err(|e| HermesError::store_with("Serialize error", e))?,
                serde_json::to_string(&profile.allow_domains).map_err(|e| HermesError::store_with("Serialize error", e))?,
                profile.max_tokens,
                profile.timeout_secs,
                profile.network_access as i32,
                format!("{:?}", profile.filesystem_access_level),
            ],
        )
        .map_err(|e| HermesError::store_with("Failed to create permission profile", e))?;

        let mut cache = self.cache.lock().map_err(|e| HermesError::store(e.to_string()))?;
        cache.insert(profile.name.clone(), profile);
        debug!("Created permission profile, cache size: {}", cache.len());
        Ok(())
    }

    pub fn get_profile(&self, name: &str) -> HermesResult<PermissionProfile> {
        {
            let cache = self.cache.lock().map_err(|e| HermesError::store(e.to_string()))?;
            if let Some(profile) = cache.get(name) {
                return Ok(profile.clone());
            }
        }

        let db = self.db.lock().map_err(|e| HermesError::store(e.to_string()))?;
        let mut stmt = db
            .prepare("SELECT name, allow_tools, allow_domains, max_tokens, timeout_secs, network_access, filesystem_access_level FROM permission_profiles WHERE name = ?1")
            .map_err(|e| HermesError::store_with("Failed to prepare profile query", e))?;

        let result = stmt.query_row(rusqlite::params![name], |row| {
            let allow_tools_str: String = row.get(1)?;
            let allow_domains_str: String = row.get(2)?;
            let fs_level_str: String = row.get(6)?;

            let allow_tools: Vec<String> = serde_json::from_str(&allow_tools_str).unwrap_or_default();
            let allow_domains: Vec<String> = serde_json::from_str(&allow_domains_str).unwrap_or_default();
            let fs_level = match fs_level_str.as_str() {
                "ReadOnly" => FilesystemAccessLevel::ReadOnly,
                "ReadWrite" => FilesystemAccessLevel::ReadWrite,
                _ => FilesystemAccessLevel::None,
            };

            Ok(PermissionProfile {
                name: row.get(0)?,
                allow_tools,
                allow_domains,
                max_tokens: row.get(3)?,
                timeout_secs: row.get(4)?,
                network_access: row.get::<_, i32>(5)? != 0,
                filesystem_access_level: fs_level,
            })
        });

        match result {
            Ok(profile) => {
                let mut cache = self.cache.lock().map_err(|e| HermesError::store(e.to_string()))?;
                cache.insert(name.to_string(), profile.clone());
                Ok(profile)
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                Err(HermesError::permission(format!("Permission profile '{}' not found", name)))
            }
            Err(e) => Err(HermesError::store_with("Failed to query profile", e)),
        }
    }

    pub fn check(&self, agent_name: &str, action: &str, resource: &str) -> HermesResult<bool> {
        let db = self.db.lock().map_err(|e| HermesError::store(e.to_string()))?;
        let mut stmt = db
            .prepare("SELECT granted FROM permission_grants WHERE agent_name = ?1 AND action = ?2 AND resource = ?3")
            .map_err(|e| HermesError::store_with("Failed to prepare permission check", e))?;

        let result: Result<i32, _> = stmt.query_row(
            rusqlite::params![agent_name, action, resource],
            |row| row.get(0),
        );

        match result {
            Ok(granted) => Ok(granted != 0),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(false),
            Err(e) => Err(HermesError::store_with("Failed to check permission", e)),
        }
    }

    pub fn grant(&self, agent_name: &str, action: &str, resource: &str) -> HermesResult<()> {
        let db = self.db.lock().map_err(|e| HermesError::store(e.to_string()))?;
        db.execute(
            "INSERT OR REPLACE INTO permission_grants (agent_name, action, resource, granted) VALUES (?1, ?2, ?3, 1)",
            rusqlite::params![agent_name, action, resource],
        )
        .map_err(|e| HermesError::store_with("Failed to grant permission", e))?;
        debug!("Granted {} {} on {}", agent_name, action, resource);
        Ok(())
    }

    pub fn revoke(&self, agent_name: &str, action: &str, resource: &str) -> HermesResult<()> {
        let db = self.db.lock().map_err(|e| HermesError::store(e.to_string()))?;
        db.execute(
            "DELETE FROM permission_grants WHERE agent_name = ?1 AND action = ?2 AND resource = ?3",
            rusqlite::params![agent_name, action, resource],
        )
        .map_err(|e| HermesError::store_with("Failed to revoke permission", e))?;
        debug!("Revoked {} {} on {}", agent_name, action, resource);
        Ok(())
    }

    pub fn list_profiles(&self) -> HermesResult<Vec<PermissionProfile>> {
        let db = self.db.lock().map_err(|e| HermesError::store(e.to_string()))?;
        let mut stmt = db
            .prepare("SELECT name, allow_tools, allow_domains, max_tokens, timeout_secs, network_access, filesystem_access_level FROM permission_profiles")
            .map_err(|e| HermesError::store_with("Failed to prepare list profiles", e))?;

        let profiles: Vec<PermissionProfile> = stmt
            .query_map([], |row| {
                let allow_tools_str: String = row.get(1)?;
                let allow_domains_str: String = row.get(2)?;
                let fs_level_str: String = row.get(6)?;

                let allow_tools: Vec<String> = serde_json::from_str(&allow_tools_str).unwrap_or_default();
                let allow_domains: Vec<String> = serde_json::from_str(&allow_domains_str).unwrap_or_default();
                let fs_level = match fs_level_str.as_str() {
                    "ReadOnly" => FilesystemAccessLevel::ReadOnly,
                    "ReadWrite" => FilesystemAccessLevel::ReadWrite,
                    _ => FilesystemAccessLevel::None,
                };

                Ok(PermissionProfile {
                    name: row.get(0)?,
                    allow_tools,
                    allow_domains,
                    max_tokens: row.get(3)?,
                    timeout_secs: row.get(4)?,
                    network_access: row.get::<_, i32>(5)? != 0,
                    filesystem_access_level: fs_level,
                })
            })
            .map_err(|e| HermesError::store_with("Failed to query profiles", e))?
            .filter_map(|r| r.ok())
            .collect();

        {
            let mut cache = self.cache.lock().map_err(|e| HermesError::store(e.to_string()))?;
            for p in &profiles {
                cache.insert(p.name.clone(), p.clone());
            }
        }

        Ok(profiles)
    }

    pub fn evict_cache(&self) {
        if let Ok(mut cache) = self.cache.lock() {
            let count = cache.len();
            cache.clear();
            if count > 0 {
                debug!("Evicted {} entries from permission cache", count);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_engine() -> PermissionEngine {
        let conn = Connection::open_in_memory().unwrap();
        PermissionEngine::new(Arc::new(Mutex::new(conn))).unwrap()
    }

    #[test]
    fn test_default_profile_exists() {
        let engine = test_engine();
        let profile = engine.get_profile("default").unwrap();
        assert_eq!(profile.name, "default");
        assert!(!profile.network_access);
        assert_eq!(profile.filesystem_access_level, FilesystemAccessLevel::None);
    }

    #[test]
    fn test_create_and_get_profile() {
        let engine = test_engine();
        let profile = PermissionProfile {
            name: "test_profile".to_string(),
            allow_tools: vec!["search".to_string()],
            allow_domains: vec!["general".to_string()],
            max_tokens: Some(1000),
            timeout_secs: Some(30),
            network_access: true,
            filesystem_access_level: FilesystemAccessLevel::ReadOnly,
        };
        engine.create_profile(profile.clone()).unwrap();
        let loaded = engine.get_profile("test_profile").unwrap();
        assert_eq!(loaded.name, "test_profile");
        assert_eq!(loaded.allow_tools, vec!["search"]);
        assert!(loaded.network_access);
    }

    #[test]
    fn test_grant_and_check() {
        let engine = test_engine();
        assert!(!engine.check("agent1", "write", "/tmp/file").unwrap());
        engine.grant("agent1", "write", "/tmp/file").unwrap();
        assert!(engine.check("agent1", "write", "/tmp/file").unwrap());
        engine.revoke("agent1", "write", "/tmp/file").unwrap();
        assert!(!engine.check("agent1", "write", "/tmp/file").unwrap());
    }
}
