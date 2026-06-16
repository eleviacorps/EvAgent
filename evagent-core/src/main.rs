use std::path::PathBuf;

use clap::{Parser, Subcommand};
use tracing::{debug, info};
use tracing_subscriber::EnvFilter;

mod agent_registry;
mod config;
mod dispatcher;
mod errors;
mod intent_router;
mod models;
mod permissions;
mod server;
mod session;
mod skill_loader;

use errors::HermesResult;

/// EvAgent Core Engine — Multi-Domain AI Agent Orchestration Backend
#[derive(Parser)]
#[command(name = "evagent-core")]
#[command(about = "EvAgent Multi-Domain Agent Orchestration Engine", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Path to config file (YAML). Overrides default paths.
    #[arg(short = 'c', long = "config", global = true)]
    config: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the EvAgent daemon (WebSocket server, agent dispatcher)
    Start {
        /// Port to listen on (overrides config)
        #[arg(short = 'p', long = "port")]
        port: Option<u16>,
    },

    /// List registered agents
    Agent {
        #[command(subcommand)]
        action: AgentAction,
    },

    /// List registered skills
    Skill {
        #[command(subcommand)]
        action: SkillAction,
    },

    /// Validate configuration
    Validate,
}

#[derive(clap::Subcommand)]
enum AgentAction {
    /// List all registered agents
    List {
        /// Filter by domain
        #[arg(short = 'd', long = "domain")]
        domain: Option<String>,
    },
}

#[derive(clap::Subcommand)]
enum SkillAction {
    /// List all registered skills
    List {
        /// Filter by domain
        #[arg(short = 'd', long = "domain")]
        domain: Option<String>,
    },
}

#[tokio::main]
async fn main() -> HermesResult<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info,evagent_core=debug")),
        )
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Start { port } => {
            cmd_start(cli.config, port).await?;
        }
        Commands::Agent { action } => {
            cmd_agent_list(cli.config, action).await?;
        }
        Commands::Skill { action } => {
            cmd_skill_list(cli.config, action).await?;
        }
        Commands::Validate => {
            cmd_validate(cli.config).await?;
        }
    }

    Ok(())
}

/// `start` subcommand: initialize stores, start WebSocket server.
async fn cmd_start(config_path: Option<String>, port_override: Option<u16>) -> HermesResult<()> {
    // Load .env file if present
    let _ = dotenvy::from_path_override("../.env");
    let _ = dotenvy::from_path_override(".env");

    let mut config = config::load_config(config_path.as_deref())?;

    // Override port if provided on CLI
    if let Some(p) = port_override {
        config.server.port = p;
    }

    info!(
        "Starting EvAgent Core Engine on {}:{}, {} max concurrent agents",
        config.server.host, config.server.port, config.core.max_concurrent_agents
    );
    debug!("Configuration: {:?}", config);

    // Initialize all engines and stores
    let state = server::initialize_engine(&config)?;

    // Run periodic maintenance in background
    let maintenance_state = state.clone();
    let _maintenance_handle = tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(3600));
        loop {
            interval.tick().await;
            debug!("Running periodic maintenance...");
            if let Err(e) = maintenance_state.session_store.archive_older_than(
                maintenance_state.config.store.archive_after_days,
            ) {
                tracing::warn!("Session archival error: {}", e);
            }
            if let Err(e) = maintenance_state.session_store.prune(
                maintenance_state.config.store.session_ttl_days,
            ) {
                tracing::warn!("Session pruning error: {}", e);
            }
            maintenance_state.agent_registry.evict_cache();
            maintenance_state.skill_loader.evict_cache();
        }
    });

    // Run the WebSocket server (this blocks until shutdown)
    server::run_server(state).await?;

    Ok(())
}

/// `agent list` subcommand: display registered agents.
pub async fn cmd_agent_list(config_path: Option<String>, action: AgentAction) -> HermesResult<()> {
    let config = config::load_config(config_path.as_deref())?;
    let conn = Arc::new(std::sync::Mutex::new(
        rusqlite::Connection::open(&config.store.db_path)
            .map_err(|e| errors::HermesError::store_with("Cannot open database", e))?,
    ));

    let registry = agent_registry::AgentRegistry::new(conn, config.routing.max_fs_walk_depth)?;

    let domain_filter = match action {
        AgentAction::List { domain } => domain,
    };

    let agents = registry.list(domain_filter.as_deref())?;

    if agents.is_empty() {
        println!("No agents registered.");
        println!();
        println!("To register agents, place .yaml files in:");
        println!("  - domains/*/agents/");
        return Ok(());
    }

    println!("Registered Agents:");
    println!("{:<25} {:<20} {:<40}", "Name", "Domain", "Description");
    println!("{:-<25} {:-<20} {:-<40}", "", "", "");
    for agent in &agents {
        let desc = if agent.description.len() > 37 {
            format!("{}...", &agent.description[..37])
        } else {
            agent.description.clone()
        };
        println!("{:<25} {:<20} {:<40}", agent.name, agent.domain, desc);
    }
    println!();
    println!("Total: {} agent(s)", agents.len());

    Ok(())
}

/// `skill list` subcommand: display registered skills.
async fn cmd_skill_list(config_path: Option<String>, action: SkillAction) -> HermesResult<()> {
    let config = config::load_config(config_path.as_deref())?;
    let conn = Arc::new(std::sync::Mutex::new(
        rusqlite::Connection::open(&config.store.db_path)
            .map_err(|e| errors::HermesError::store_with("Cannot open database", e))?,
    ));

    let loader = skill_loader::SkillLoader::new(conn, config.routing.max_fs_walk_depth)?;

    let domain_filter = match action {
        SkillAction::List { domain } => domain,
    };

    let skills = loader.list(domain_filter.as_deref())?;

    if skills.is_empty() {
        println!("No skills registered.");
        println!();
        println!("To register skills, place SKILL.md files in:");
        println!("  - evagent/skills/");
        println!("  - evagent/domains/*/");
        return Ok(());
    }

    println!("Registered Skills:");
    println!("{:<30} {:<20} {:<8} {:<30}", "Name", "Domain", "Version", "Patterns");
    println!("{:-<30} {:-<20} {:-<8} {:-<30}", "", "", "", "");
    for skill in &skills {
        let pattern_count = skill.trigger_patterns.len();
        let first_pattern = skill.trigger_patterns.first().cloned().unwrap_or_default();
        let patterns_display = if first_pattern.len() > 27 {
            format!("{}...", &first_pattern[..27])
        } else {
            first_pattern
        };
        println!(
            "{:<30} {:<20} {:<8} {:<30}",
            skill.name,
            skill.domain,
            skill.version,
            if pattern_count > 1 {
                format!("{} (+{})", patterns_display, pattern_count - 1)
            } else {
                patterns_display
            }
        );
    }
    println!();
    println!("Total: {} skill(s)", skills.len());

    Ok(())
}

/// `validate` subcommand: validate configuration and report issues.
async fn cmd_validate(config_path: Option<String>) -> HermesResult<()> {
    println!("Validating EvAgent configuration...");
    println!();

    // Load and validate config
    match config::load_config(config_path.as_deref()) {
        Ok(config) => {
            println!("✅ Configuration is valid.");
            println!();
            println!("Configuration Summary:");
            println!("  Core:");
            println!("    Max concurrent agents: {}", config.core.max_concurrent_agents);
            println!("    Default timeout: {}s", config.core.default_timeout_secs);
            println!("  Store:");
            println!("    Database: {}", config.store.db_path);
            println!("    Session TTL: {} days", config.store.session_ttl_days);
            println!("    Archive after: {} days", config.store.archive_after_days);
            println!("  Dispatch:");
            println!("    Max concurrent: {}", config.dispatch.max_concurrent);
            println!("    Default token budget: {}", config.dispatch.default_token_budget);
            println!("  Security:");
            println!("    Default network access: {}", config.security.default_network_access);
            println!("    Allowed domains: {:?}", config.security.allowed_domains);
            println!("  Server:");
            println!("    Host: {}", config.server.host);
            println!("    Port: {}", config.server.port);
            println!("  Routing:");
            println!("    Min confidence: {}", config.routing.min_confidence);
            println!("    Max FS walk depth: {}", config.routing.max_fs_walk_depth);
        }
        Err(e) => {
            println!("❌ Configuration validation FAILED:");
            println!("   {}", e);
            println!();
            println!("Please fix the configuration and run again.");
            return Err(e);
        }
    }

    // Check database connectivity
    println!();
    let config = config::load_config(config_path.as_deref()).unwrap_or_default();
    match rusqlite::Connection::open(&config.store.db_path) {
        Ok(conn) => {
            println!("✅ Database '{}' is accessible.", config.store.db_path);
            let _ = conn.close();
        }
        Err(e) => {
            println!("⚠️  Cannot open database '{}': {}", config.store.db_path, e);
        }
    }

    // Check agent paths
    let agent_paths = [
        PathBuf::from("evagent/agents"),
        PathBuf::from("evagent/domains"),
    ];
    for path in &agent_paths {
        if path.exists() {
            println!("✅ Agent path '{}' exists.", path.display());
        } else {
            println!("\nℹ️  Path '{}' does not exist yet.", path.display());
        }
    }

    // Check skill paths
    let skill_paths = [
        PathBuf::from("evagent/skills"),
        PathBuf::from("evagent/domains"),
    ];
    for path in &skill_paths {
        if path.exists() {
            println!("✅ Skill path '{}' exists.", path.display());
        } else {
            println!("ℹ️  Skill path '{}' does not exist yet. Create it to register skills.", path.display());
        }
    }

    println!();
    println!("✅ Validation complete.");

    Ok(())
}

use std::sync::Arc;
