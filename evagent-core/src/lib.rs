//! EvAgent Core Engine — Multi-Domain AI Agent Orchestration Backend
//!
//! This library crate provides all core modules used by the EvAgent engine:
//! - Agent Registry: scans, registers, indexes agent YAML definitions
//! - Intent Router: routes user prompts to domain experts via regex + similarity
//! - Dispatcher: spawns and coordinates sub-agent processes
//! - Session Store: SQLite-backed session lifecycle management
//! - Permission Engine: grants, revokes, and checks agent permissions
//! - Skill Loader: parses SKILL.md frontmatter and indexes skill definitions
//! - Configuration: loads, validates, applies env overrides to HermesConfig

pub mod agent_registry;
pub mod config;
pub mod dispatcher;
pub mod errors;
pub mod intent_router;
pub mod models;
pub mod permissions;
pub mod server;
pub mod session;
pub mod skill_loader;
