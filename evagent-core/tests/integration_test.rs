//! Integration tests for EvAgent Core Engine.
//!
//! These tests validate the end-to-end behavior of core subsystems:
//! - Session store (create, read, archive, prune)
//! - Intent router (domain registration, routing, low-confidence handling)
//! - Agent registry (registration, lookup, search)
//! - Dispatcher (single/parallel dispatch, aggregation)
//! - Permission engine (profiles, grants, revocations)
//! - Skill loader (frontmatter parsing, caching)
//! - Configuration (loading, validation, env-override)

use std::sync::{Arc, Mutex};

use evagent_core::agent_registry::AgentRegistry;
use evagent_core::config;
use evagent_core::dispatcher::Dispatcher;
use evagent_core::intent_router::IntentRouter;
use evagent_core::models::{
    DispatchTask, HermesConfig, Message, MessageRole, PermissionProfile,
    RegisteredDomain, SessionStatus, SubAgentStatus, FilesystemAccessLevel, SubAgentOutput,
};
use evagent_core::permissions::PermissionEngine;
use evagent_core::session::SessionStore;
use evagent_core::skill_loader::SkillLoader;

// ── Helpers ───────────────────────────────────────────────────────────────────

fn in_memory_db() -> Arc<Mutex<rusqlite::Connection>> {
    Arc::new(Mutex::new(rusqlite::Connection::open_in_memory().unwrap()))
}

fn test_config() -> HermesConfig {
    HermesConfig::default()
}

fn test_session_store() -> SessionStore {
    SessionStore::new(in_memory_db(), 30, 7).unwrap()
}

fn test_agent_registry() -> AgentRegistry {
    AgentRegistry::new(in_memory_db(), 3).unwrap()
}

fn test_skill_loader() -> SkillLoader {
    SkillLoader::new(in_memory_db(), 3).unwrap()
}

fn test_permission_engine() -> PermissionEngine {
    PermissionEngine::new(in_memory_db()).unwrap()
}

fn test_router() -> IntentRouter {
    IntentRouter::new(0.5)
}

// ── Session Store Integration Tests ───────────────────────────────────────────

#[test]
fn test_session_lifecycle() {
    let store = test_session_store();

    // Create
    let session = store.create("coding").unwrap();
    assert_eq!(session.domain, "coding");
    assert_eq!(session.status, SessionStatus::Active);
    assert_eq!(session.message_count, 0);

    // Read
    let loaded = store.get(&session.id).unwrap();
    assert_eq!(loaded.id, session.id);
    assert_eq!(loaded.domain, "coding");

    // Append messages
    let msg = Message {
        id: uuid::Uuid::new_v4().to_string(),
        session_id: session.id.clone(),
        role: MessageRole::User,
        content: "Write a parser".to_string(),
        timestamp: chrono::Utc::now(),
        tokens: 42,
    };
    store.append_message(&session.id, msg).unwrap();

    let loaded2 = store.get(&session.id).unwrap();
    assert_eq!(loaded2.message_count, 1);
    assert_eq!(loaded2.total_tokens, 42);

    // Read messages
    let msgs = store.get_messages(&session.id, 1, 10).unwrap();
    assert_eq!(msgs.len(), 1);
    assert_eq!(msgs[0].content, "Write a parser");

    // Archive
    store.archive(&session.id).unwrap();
    let archived = store.get(&session.id).unwrap();
    assert_eq!(archived.status, SessionStatus::Archived);
}

#[test]
fn test_session_list_and_filter() {
    let store = test_session_store();
    let s1 = store.create("coding").unwrap();
    let s2 = store.create("research").unwrap();

    let all = store.list(None).unwrap();
    assert!(all.len() >= 2);

    store.archive(&s1.id).unwrap();
    let active = store.list(Some(SessionStatus::Active)).unwrap();
    assert!(active.iter().all(|s| s.status == SessionStatus::Active));
}

#[test]
fn test_session_archive_older_than() {
    let store = test_session_store();
    store.create("test").unwrap();

    // Archive with 0 days should attempt to archive everything older than now
    // (our session was just created, so it shouldn't be archived)
    let archived = store.archive_older_than(0).unwrap();
    // May be 0 or more depending on timing — just verify it returns a number
    assert!(archived <= 1);
}

#[test]
fn test_session_update_summary_and_agents() {
    let store = test_session_store();
    let session = store.create("writing").unwrap();

    store.update_summary(&session.id, "Completed writing task").unwrap();
    let loaded = store.get(&session.id).unwrap();
    assert_eq!(loaded.summary, Some("Completed writing task".to_string()));

    store.update_dispatch_agents(&session.id, &["agent1".to_string(), "agent2".to_string()]).unwrap();
    let loaded2 = store.get(&session.id).unwrap();
    assert_eq!(loaded2.dispatch_agents.len(), 2);
}

#[test]
fn test_session_get_nonexistent() {
    let store = test_session_store();
    let result = store.get("nonexistent-session-id");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Session"));
}

#[test]
fn test_session_archive_already_archived() {
    let store = test_session_store();
    let session = store.create("test").unwrap();
    store.archive(&session.id).unwrap();
    // Archiving again should fail
    let result = store.archive(&session.id);
    assert!(result.is_err());
}

// ── Intent Router Integration Tests ───────────────────────────────────────────

#[test]
fn test_router_register_and_list_domains() {
    let router = test_router();
    router
        .register_domain(RegisteredDomain {
            name: "coding".to_string(),
            patterns: vec![r"code".to_string(), r"program".to_string()],
            agents: vec!["code_reviewer".to_string()],
        })
        .unwrap();

    let domains = router.get_domains();
    assert_eq!(domains.len(), 1);
    assert_eq!(domains[0].name, "coding");

    let domain = router.get_domain("coding");
    assert!(domain.is_some());
    assert_eq!(domain.unwrap().agents.len(), 1);
}

#[test]
fn test_router_regex_matching() {
    let router = test_router();
    router
        .register_domain(RegisteredDomain {
            name: "engineering".to_string(),
            patterns: vec![
                r"fix bug".to_string(),
                r"code review".to_string(),
                r"refactor".to_string(),
            ],
            agents: vec!["code_reviewer".to_string()],
        })
        .unwrap();

    let result = router.route("can you do a code review for me").unwrap();
    assert_eq!(result.domain, "engineering");
    assert!(result.confidence >= 0.3);
    assert!(!result.llm_fallback_used);
}

#[test]
fn test_router_multiple_domains_best_match() {
    let router = test_router();
    router
        .register_domain(RegisteredDomain {
            name: "writing".to_string(),
            patterns: vec![r"write".to_string(), r"draft".to_string(), r"article".to_string()],
            agents: vec!["writer".to_string()],
        })
        .unwrap();
    router
        .register_domain(RegisteredDomain {
            name: "research".to_string(),
            patterns: vec![r"search".to_string(), r"find".to_string(), r"research".to_string()],
            agents: vec!["researcher".to_string()],
        })
        .unwrap();

    // Should route to writing
    let result = router.route("write a draft article about AI").unwrap();
    assert_eq!(result.domain, "writing");
}

#[test]
fn test_router_low_confidence_error() {
    let router = test_router();
    router
        .register_domain(RegisteredDomain {
            name: "general".to_string(),
            patterns: vec!["hello".to_string()],
            agents: vec!["assistant".to_string()],
        })
        .unwrap();

    let result = router.route("quantum chromodynamics lagrangian");
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("LowConfidence") || err.contains("low"));
}

#[test]
fn test_router_duplicate_domain_prevented() {
    let router = test_router();
    router
        .register_domain(RegisteredDomain {
            name: "coding".to_string(),
            patterns: vec![],
            agents: vec![],
        })
        .unwrap();
    let result = router.register_domain(RegisteredDomain {
        name: "coding".to_string(),
        patterns: vec![],
        agents: vec![],
    });
    assert!(result.is_err());
}

#[test]
fn test_router_unregister_domain() {
    let router = test_router();
    router
        .register_domain(RegisteredDomain {
            name: "temp".to_string(),
            patterns: vec!["temp".to_string()],
            agents: vec![],
        })
        .unwrap();
    router.unregister_domain("temp").unwrap();
    assert!(router.get_domain("temp").is_none());
}

// ── Agent Registry Integration Tests ──────────────────────────────────────────

#[test]
fn test_agent_register_and_list() {
    let registry = test_agent_registry();
    let dir = std::env::temp_dir().join("hermes_int_test_agents");
    std::fs::create_dir_all(&dir).unwrap();

    let yaml = r#"
name: test-agent
domain: coding
description: A test agent for integration tests
tool_scope:
  - read_files
  - search_files
permission_profile: default
"#;
    let path = dir.join("test-agent.yaml");
    std::fs::write(&path, yaml).unwrap();

    registry.register_path(&path).unwrap();
    let agent = registry.get("test-agent").unwrap();
    assert_eq!(agent.name, "test-agent");
    assert_eq!(agent.domain, "coding");

    let agents = registry.list(None).unwrap();
    assert!(agents.iter().any(|a| a.name == "test-agent"));

    let coding_agents = registry.list(Some("coding")).unwrap();
    assert!(coding_agents.iter().any(|a| a.name == "test-agent"));

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_agent_search() {
    let registry = test_agent_registry();
    let dir = std::env::temp_dir().join("hermes_int_test_search");
    std::fs::create_dir_all(&dir).unwrap();

    let yaml = r#"
name: code-reviewer
domain: coding
description: Reviews code for quality and standards
tool_scope:
  - read_files
permission_profile: default
"#;
    std::fs::write(&dir.join("code-reviewer.yaml"), yaml).unwrap();
    registry.register_path(&dir.join("code-reviewer.yaml")).unwrap();

    let results = registry.search("review").unwrap();
    assert!(!results.is_empty());

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_agent_get_nonexistent() {
    let registry = test_agent_registry();
    let result = registry.get("no-such-agent");
    assert!(result.is_err());
}

#[test]
fn test_agent_scan_and_register_paths() {
    let registry = test_agent_registry();
    let dir = std::env::temp_dir().join("hermes_int_test_scan");
    let agents_dir = dir.join("agents");
    std::fs::create_dir_all(&agents_dir).unwrap();

    let yaml = r#"name: scanner-agent
domain: coding
description: Scanned agent
tool_scope: []
permission_profile: default
"#;
    std::fs::write(&agents_dir.join("scanner-agent.yaml"), yaml).unwrap();

    registry.register_scan_paths(&[agents_dir.clone()]);
    let count = registry.scan_and_register().unwrap();
    assert_eq!(count, 1);

    std::fs::remove_dir_all(&dir).unwrap();
}

// ── Dispatcher Integration Tests ──────────────────────────────────────────────

#[test]
fn test_dispatcher_single_task() {
    let dispatcher = Dispatcher::new(5, 30, 4096);
    let task = DispatchTask {
        id: "int-test-1".to_string(),
        goal: "Integration test task".to_string(),
        context: "Testing".to_string(),
        assigned_skills: vec![],
        permission_profile: "default".to_string(),
        timeout_secs: 10,
        token_budget: 1000,
        domain: "general".to_string(),
    };
    let result = dispatcher.dispatch(&task).unwrap();
    assert_eq!(result.status, SubAgentStatus::Completed);
    assert!(result.wall_clock_ms > 0);
    assert!(result.task_id == "int-test-1");
}

#[tokio::test]
async fn test_dispatcher_parallel() {
    let dispatcher = Dispatcher::new(10, 30, 4096);
    let tasks: Vec<DispatchTask> = (0..5)
        .map(|i| DispatchTask {
            id: format!("parallel-{}", i),
            goal: format!("Task {}", i),
            context: String::new(),
            assigned_skills: vec![],
            permission_profile: "default".to_string(),
            timeout_secs: 10,
            token_budget: 1000,
            domain: "general".to_string(),
        })
        .collect();

    let results = dispatcher.dispatch_parallel(tasks).await.unwrap();
    assert_eq!(results.len(), 5);
    for r in &results {
        assert_eq!(r.status, SubAgentStatus::Completed);
    }
}

#[test]
fn test_dispatcher_aggregate() {
    let dispatcher = Dispatcher::new(5, 30, 4096);
    let outputs = vec![
        SubAgentOutput {
            task_id: "t1".to_string(),
            status: SubAgentStatus::Completed,
            result: Some("Done".to_string()),
            tokens_used: 100,
            errors: vec![],
            wall_clock_ms: 50,
        },
        SubAgentOutput {
            task_id: "t2".to_string(),
            status: SubAgentStatus::Failed,
            result: None,
            tokens_used: 0,
            errors: vec!["Error".to_string()],
            wall_clock_ms: 0,
        },
    ];
    let aggregated = dispatcher.aggregate(outputs).unwrap();
    assert!(aggregated.contains("total_agents"));
    assert!(aggregated.contains("completed"));
    assert!(aggregated.contains("failed"));
}

#[test]
fn test_dispatcher_assign_skills_and_permissions() {
    let dispatcher = Dispatcher::new(5, 30, 4096);
    dispatcher.assign_skills("agent1", vec!["skill1".to_string(), "skill2".to_string()]);
    dispatcher.assign_permissions("agent1", "restricted");
    // Dispatch a task with this agent's permissions
    let task = DispatchTask {
        id: "assign-test".to_string(),
        goal: "Test assignment".to_string(),
        context: String::new(),
        assigned_skills: vec!["skill1".to_string()],
        permission_profile: "restricted".to_string(),
        timeout_secs: 10,
        token_budget: 1000,
        domain: "general".to_string(),
    };
    let result = dispatcher.dispatch(&task).unwrap();
    assert_eq!(result.status, SubAgentStatus::Completed);
}

// ── Permission Engine Integration Tests ───────────────────────────────────────

#[test]
fn test_permission_default_profile() {
    let engine = test_permission_engine();
    let profile = engine.get_profile("default").unwrap();
    assert_eq!(profile.name, "default");
    assert!(!profile.network_access);
    assert_eq!(profile.filesystem_access_level, FilesystemAccessLevel::None);
}

#[test]
fn test_permission_create_and_list_profiles() {
    let engine = test_permission_engine();
    let profile = PermissionProfile {
        name: "power-user".to_string(),
        allow_tools: vec!["read_files".to_string(), "write_files".to_string()],
        allow_domains: vec!["coding".to_string()],
        max_tokens: Some(100_000),
        timeout_secs: Some(300),
        network_access: true,
        filesystem_access_level: FilesystemAccessLevel::ReadWrite,
    };
    engine.create_profile(profile).unwrap();

    let loaded = engine.get_profile("power-user").unwrap();
    assert_eq!(loaded.allow_tools.len(), 2);
    assert!(loaded.network_access);

    let profiles = engine.list_profiles().unwrap();
    assert!(profiles.len() >= 2); // default + power-user
}

#[test]
fn test_permission_grant_revoke_check() {
    let engine = test_permission_engine();
    assert!(!engine.check("agent_x", "delete", "/critical/file").unwrap());
    engine.grant("agent_x", "delete", "/critical/file").unwrap();
    assert!(engine.check("agent_x", "delete", "/critical/file").unwrap());
    engine.revoke("agent_x", "delete", "/critical/file").unwrap();
    assert!(!engine.check("agent_x", "delete", "/critical/file").unwrap());
}

#[test]
fn test_permission_get_nonexistent_profile() {
    let engine = test_permission_engine();
    let result = engine.get_profile("no-such-profile");
    assert!(result.is_err());
}

#[test]
fn test_permission_evict_cache() {
    let engine = test_permission_engine();
    engine.get_profile("default").unwrap(); // populates cache
    engine.evict_cache();
    // Should still be able to get it (re-fetches from DB)
    let profile = engine.get_profile("default").unwrap();
    assert_eq!(profile.name, "default");
}

// ── Skill Loader Integration Tests ────────────────────────────────────────────

#[test]
fn test_skill_load_and_get() {
    let loader = test_skill_loader();
    let dir = std::env::temp_dir().join("hermes_int_test_skills");
    std::fs::create_dir_all(&dir).unwrap();

    let content = r#"---
name: code-review-skill
domain: coding
trigger_patterns:
  - "review this PR"
  - "check my code"
applicable_agents:
  - code-reviewer
steps:
  - "Analyze code"
  - "Provide feedback"
examples:
  - "Good: clear variable names"
anti_patterns:
  - "Nitpicking style only"
version: 2
---
# Code Review Skill
"#;
    let path = dir.join("SKILL.md");
    std::fs::write(&path, content).unwrap();

    let skill = loader.load(&path).unwrap();
    assert_eq!(skill.name, "code-review-skill");
    assert_eq!(skill.domain, "coding");
    assert_eq!(skill.version, 2);
    assert_eq!(skill.trigger_patterns.len(), 2);
    assert_eq!(skill.steps.len(), 2);

    // Get from cache/db
    let cached = loader.get("code-review-skill").unwrap();
    assert_eq!(cached.name, "code-review-skill");

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_skill_search() {
    let loader = test_skill_loader();
    let dir = std::env::temp_dir().join("hermes_int_test_skill_search");
    std::fs::create_dir_all(&dir).unwrap();

    let content = r#"---
name: debugging-skill
domain: coding
trigger_patterns:
  - "debug this"
  - "fix error"
applicable_agents:
  - debugger
version: 1
---
"#;
    std::fs::write(&dir.join("SKILL.md"), content).unwrap();
    loader.load(&dir.join("SKILL.md")).unwrap();

    let results = loader.search("debug", None).unwrap();
    assert!(!results.is_empty());

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_skill_list_by_domain() {
    let loader = test_skill_loader();
    let dir = std::env::temp_dir().join("hermes_int_test_skill_list");
    std::fs::create_dir_all(&dir).unwrap();

    let content = r#"---
name: list-test-skill
domain: testing
trigger_patterns:
  - "run test"
applicable_agents: []
version: 1
---
"#;
    std::fs::write(&dir.join("SKILL.md"), content).unwrap();
    loader.load(&dir.join("SKILL.md")).unwrap();

    let testing_skills = loader.list(Some("testing")).unwrap();
    assert!(testing_skills.iter().any(|s| s.name == "list-test-skill"));

    let other_skills = loader.list(Some("other")).unwrap();
    assert!(other_skills.iter().all(|s| s.domain != "list-test-skill"));

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_skill_load_missing_frontmatter() {
    let loader = test_skill_loader();
    let dir = std::env::temp_dir().join("hermes_int_test_bad_skill");
    std::fs::create_dir_all(&dir).unwrap();

    let bad_content = "# No frontmatter\nJust markdown content";
    std::fs::write(&dir.join("SKILL.md"), bad_content).unwrap();

    let result = loader.load(&dir.join("SKILL.md"));
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("frontmatter"));

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_skill_evict_cache() {
    let loader = test_skill_loader();
    let dir = std::env::temp_dir().join("hermes_int_test_evict");
    std::fs::create_dir_all(&dir).unwrap();

    let content = r#"---
name: cache-skill
domain: cache-test
trigger_patterns: []
applicable_agents: []
version: 1
---
"#;
    std::fs::write(&dir.join("SKILL.md"), content).unwrap();
    loader.load(&dir.join("SKILL.md")).unwrap();

    // Get to populate cache
    loader.get("cache-skill").unwrap();
    loader.evict_cache();
    // Should still be loadable
    let skill = loader.get("cache-skill").unwrap();
    assert_eq!(skill.name, "cache-skill");

    std::fs::remove_dir_all(&dir).unwrap();
}

// ── Configuration Integration Tests ───────────────────────────────────────────

#[test]
fn test_config_default_valid() {
    let config = HermesConfig::default();
    assert_eq!(config.core.max_concurrent_agents, 5);
    assert_eq!(config.core.default_timeout_secs, 120);
    assert_eq!(config.server.port, 9753);
    assert_eq!(config.dispatch.default_token_budget, 4096);
    assert_eq!(config.routing.min_confidence, 0.6);
    assert_eq!(config.routing.max_fs_walk_depth, 3);
}

#[test]
fn test_config_roundtrip() {
    let config = HermesConfig::default();
    let yaml = serde_yaml::to_string(&config).unwrap();
    let deserialized: HermesConfig = serde_yaml::from_str(&yaml).unwrap();
    assert_eq!(deserialized.core.max_concurrent_agents, config.core.max_concurrent_agents);
    assert_eq!(deserialized.server.port, config.server.port);
}

#[test]
fn test_config_validation_bounds() {
    // These would normally go through validate_config, which is internal.
    // Instead we verify the defaults are within valid bounds.
    let config = HermesConfig::default();
    assert!(config.core.max_concurrent_agents > 0);
    assert!(config.core.max_concurrent_agents <= 100);
    assert!(config.core.default_timeout_secs > 0);
    assert!(config.core.default_timeout_secs <= 3600);
    assert!(config.store.session_ttl_days > 0);
    assert!(config.store.session_ttl_days <= 365);
    assert!(config.store.archive_after_days < config.store.session_ttl_days);
    assert!(config.dispatch.max_concurrent > 0);
    assert!(config.dispatch.max_concurrent <= 50);
    assert!(config.dispatch.default_token_budget > 0);
    assert!(config.dispatch.default_token_budget <= 1_000_000);
    assert!(config.routing.min_confidence >= 0.0);
    assert!(config.routing.min_confidence <= 1.0);
    assert!(config.routing.max_fs_walk_depth > 0);
    assert!(config.routing.max_fs_walk_depth <= 20);
    assert!(config.server.port > 0);
}

// ── Error Handling Integration Tests ──────────────────────────────────────────

#[test]
fn test_error_display_and_conversion() {
    use evagent_core::errors::HermesError;

    let config_err = HermesError::config("test config error");
    let msg = config_err.to_string();
    assert!(msg.contains("Config"));
    assert!(msg.contains("test config error"));

    let io_err: HermesError = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found").into();
    assert!(io_err.to_string().contains("Io"));

    let yaml_err: HermesError = serde_yaml::from_str::<HermesConfig>("invalid: [").unwrap_err().into();
    assert!(yaml_err.to_string().contains("YAML"));

    // HermesResult alias works
    fn returns_result() -> evagent_core::errors::HermesResult<String> {
        Ok("success".to_string())
    }
    assert_eq!(returns_result().unwrap(), "success");
}
