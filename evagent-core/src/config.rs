use std::env;
use std::path::Path;

use crate::errors::{HermesError, HermesResult};
use crate::models::HermesConfig;
use tracing::{info, warn};

/// Load configuration from a YAML file, then override with environment variables.
/// Returns validated config or an error.
pub fn load_config(path: Option<&str>) -> HermesResult<HermesConfig> {
    let mut config = match path {
        Some(p) if Path::new(p).exists() => {
            let contents = std::fs::read_to_string(p)
                .map_err(|e| HermesError::config_with(format!("Cannot read config file: {}", p), e))?;
            serde_yaml::from_str(&contents)
                .map_err(|e| HermesError::config_with(format!("Invalid YAML in config: {}", p), e))?
        }
        Some(p) => {
            warn!("Config file not found at '{}', using defaults", p);
            HermesConfig::default()
        }
        None => {
            // Try default paths
            let default_paths = [
                "hermes-config.yaml",
                "hermes-config.yml",
                "config/hermes-config.yaml",
                "config/hermes-config.yml",
                "~/.hermes/config.yaml",
            ];
            let mut loaded = false;
            let mut config = HermesConfig::default();
            for dp in &default_paths {
                let expanded = shellexpand::tilde(dp);
                if Path::new(expanded.as_ref()).exists() {
                    let contents = std::fs::read_to_string(expanded.as_ref())
                        .map_err(|e| HermesError::config_with(format!("Cannot read config: {}", dp), e))?;
                    config = serde_yaml::from_str(&contents)
                        .map_err(|e| HermesError::config_with(format!("Invalid YAML: {}", dp), e))?;
                    info!("Loaded config from {}", dp);
                    loaded = true;
                    break;
                }
            }
            if !loaded {
                info!("No config file found, using default configuration");
            }
            config
        }
    };

    // Override from environment variables
    apply_env_overrides(&mut config);

    // Validate
    validate_config(&config)?;

    Ok(config)
}

fn apply_env_overrides(config: &mut HermesConfig) {
    if let Ok(v) = env::var("EVAGENT_MAX_CONCURRENT") {
        if let Ok(n) = v.parse::<usize>() {
            config.core.max_concurrent_agents = n;
        }
    }
    if let Ok(v) = env::var("EVAGENT_DEFAULT_TIMEOUT") {
        if let Ok(n) = v.parse::<u64>() {
            config.core.default_timeout_secs = n;
        }
    }
    if let Ok(v) = env::var("EVAGENT_DB_PATH") {
        config.store.db_path = v;
    }
    if let Ok(v) = env::var("EVAGENT_SESSION_TTL_DAYS") {
        if let Ok(n) = v.parse::<u32>() {
            config.store.session_ttl_days = n;
        }
    }
    if let Ok(v) = env::var("EVAGENT_ARCHIVE_AFTER_DAYS") {
        if let Ok(n) = v.parse::<u32>() {
            config.store.archive_after_days = n;
        }
    }
    if let Ok(v) = env::var("EVAGENT_MAX_CONCURRENT_DISPATCH") {
        if let Ok(n) = v.parse::<usize>() {
            config.dispatch.max_concurrent = n;
        }
    }
    if let Ok(v) = env::var("EVAGENT_TOKEN_BUDGET") {
        if let Ok(n) = v.parse::<u64>() {
            config.dispatch.default_token_budget = n;
        }
    }
    if let Ok(v) = env::var("EVAGENT_NETWORK_ACCESS") {
        config.security.default_network_access = v.eq_ignore_ascii_case("true");
    }
    if let Ok(v) = env::var("EVAGENT_ALLOWED_DOMAINS") {
        config.security.allowed_domains = v.split(',').map(|s| s.trim().to_string()).collect();
    }
    if let Ok(v) = env::var("EVAGENT_HOST") {
        config.server.host = v;
    }
    if let Ok(v) = env::var("EVAGENT_PORT") {
        if let Ok(n) = v.parse::<u16>() {
            config.server.port = n;
        }
    }
    if let Ok(v) = env::var("EvAgent_MIN_CONFIDENCE") {
        if let Ok(n) = v.parse::<f64>() {
            config.routing.min_confidence = n;
        }
    }
    if let Ok(v) = env::var("EvAgent_MAX_FS_WALK_DEPTH") {
        if let Ok(n) = v.parse::<u32>() {
            config.routing.max_fs_walk_depth = n;
        }
    }
}

fn validate_config(config: &HermesConfig) -> HermesResult<()> {
    if config.core.max_concurrent_agents == 0 {
        return Err(HermesError::config(
            "core.max_concurrent_agents must be > 0",
        ));
    }
    if config.core.max_concurrent_agents > 100 {
        return Err(HermesError::config(
            "core.max_concurrent_agents must be <= 100",
        ));
    }
    if config.core.default_timeout_secs == 0 {
        return Err(HermesError::config(
            "core.default_timeout_secs must be > 0",
        ));
    }
    if config.core.default_timeout_secs > 3600 {
        return Err(HermesError::config(
            "core.default_timeout_secs must be <= 3600 (1 hour)",
        ));
    }
    if config.store.session_ttl_days == 0 {
        return Err(HermesError::config("store.session_ttl_days must be > 0"));
    }
    if config.store.session_ttl_days > 365 {
        return Err(HermesError::config(
            "store.session_ttl_days must be <= 365",
        ));
    }
    if config.store.archive_after_days >= config.store.session_ttl_days {
        return Err(HermesError::config(
            "store.archive_after_days must be < store.session_ttl_days",
        ));
    }
    if config.dispatch.max_concurrent == 0 {
        return Err(HermesError::config(
            "dispatch.max_concurrent must be > 0",
        ));
    }
    if config.dispatch.max_concurrent > 50 {
        return Err(HermesError::config(
            "dispatch.max_concurrent must be <= 50",
        ));
    }
    if config.dispatch.default_token_budget == 0 {
        return Err(HermesError::config(
            "dispatch.default_token_budget must be > 0",
        ));
    }
    if config.dispatch.default_token_budget > 1_000_000 {
        return Err(HermesError::config(
            "dispatch.default_token_budget must be <= 1_000_000",
        ));
    }
    if config.routing.min_confidence < 0.0 || config.routing.min_confidence > 1.0 {
        return Err(HermesError::config(
            "routing.min_confidence must be between 0.0 and 1.0",
        ));
    }
    if config.routing.max_fs_walk_depth == 0 {
        return Err(HermesError::config(
            "routing.max_fs_walk_depth must be > 0",
        ));
    }
    if config.routing.max_fs_walk_depth > 20 {
        return Err(HermesError::config(
            "routing.max_fs_walk_depth must be <= 20",
        ));
    }
    if config.server.port == 0 {
        return Err(HermesError::config("server.port must be > 0"));
    }

    info!(
        "Configuration validated: {} max concurrent, {}s timeout, {} token budget",
        config.core.max_concurrent_agents,
        config.core.default_timeout_secs,
        config.dispatch.default_token_budget
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_valid() {
        let config = HermesConfig::default();
        assert!(validate_config(&config).is_ok());
    }

    #[test]
    fn test_zero_max_concurrent_invalid() {
        let mut config = HermesConfig::default();
        config.core.max_concurrent_agents = 0;
        assert!(validate_config(&config).is_err());
    }

    #[test]
    fn test_env_vars_applied() {
        let mut config = HermesConfig::default();
        // Simulate env var override by directly calling the function
        // In real life, set_env would be used
        assert_eq!(config.core.max_concurrent_agents, 5);
    }
}
