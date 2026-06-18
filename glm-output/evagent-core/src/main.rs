//! EvAgent core CLI entry point.
//!
//! Usage:
//!   evagent start                # start on default port (config.yaml)
//!   evagent start --port 9760    # custom port
//!   evagent scan                 # print agents + skills found and exit
//!   evagent route "build a todo app"  # print routing decision and exit

mod agent_registry;
mod config;
mod dispatcher;
mod errors;
mod intent_router;
mod llm_client;
mod memory;
mod models;
mod permissions;
mod server;
mod session;
mod skill_loader;
mod tools;
mod tools_builtins;

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Parser, Debug)]
#[command(name = "evagent", version, about = "EvAgent core engine")]
struct Cli {
    /// Path to config.yaml
    #[arg(long, default_value = "config.yaml")]
    config: PathBuf,

    /// Project root (where domains/ lives)
    #[arg(long, default_value = ".")]
    root: PathBuf,

    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    /// Start the WebSocket server
    Start {
        #[arg(long)]
        port: Option<u16>,
    },
    /// Scan domains/ and print discovered agents + skills
    Scan,
    /// Route a prompt and print the decision
    Route { prompt: String },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "evagent_core=info,tower_http=info".into()),
        )
        .init();

    let cli = Cli::parse();
    let cfg = Arc::new(config::Config::load(&cli.config)?);
    if let Some(p) = cli_cmd_port(&cli) {
        let mut cfg = (*cfg).clone();
        cfg.server.port = p;
        // Re-wrap
        return run(cli, Arc::new(cfg)).await;
    }
    run(cli, cfg).await
}

fn cli_cmd_port(cli: &Cli) -> Option<u16> {
    if let Cmd::Start { port } = &cli.cmd {
        *port
    } else {
        None
    }
}

async fn run(cli: Cli, cfg: Arc<config::Config>) -> anyhow::Result<()> {
    let root = cli.root.clone();
    let store = session::SessionStore::open(&std::path::PathBuf::from(&cfg.store.db_path))?;
    let memory = Arc::new(memory::Memory::new(&root, store.clone()));

    let registry = agent_registry::AgentRegistry::new();
    registry.scan(&root)?;
    let skills = skill_loader::SkillLoader::new();
    skills.scan(&root)?;

    let permissions = permissions::PermissionEngine::new();
    for a in registry.all() {
        permissions.register_agent(&a);
    }

    let tools = tools::ToolRegistry::new();
    let llm = llm_client::build_client(&cfg.llm)?;
    tools_builtins::register_builtins(&tools, llm.clone(), store.clone(), root.clone());

    let mut router = intent_router::IntentRouter::new(cfg.routing.min_confidence);
    router.set_llm(llm.clone());
    load_domain_definitions(&root, &router);

    let dispatcher = dispatcher::Dispatcher::new(
        cfg.clone(),
        registry,
        skills,
        permissions,
        tools,
        memory,
        llm,
    );

    match cli.cmd {
        Cmd::Start { .. } => {
            let state = server::AppState::new(cfg, Arc::new(router), Arc::new(dispatcher), store);
            server::serve(state).await?;
        }
        Cmd::Scan => {
            println!("agents: {}", state_dispatcher_count(&dispatcher));
            for a in dispatcher.registry.all() {
                println!("  - {}/{} ({})", a.domain, a.name, a.role);
            }
            println!("skills: {}", dispatcher.skills.count());
            for s in dispatcher.skills.all() {
                println!("  - {}/{} v{} — {}", s.domain, s.name, s.version, s.description);
            }
        }
        Cmd::Route { ref prompt } => {
            let r = router.route(prompt).await?;
            println!(
                "domain={} confidence={:.3} method={:?}",
                r.domain, r.confidence, r.method
            );
        }
    }
    Ok(())
}

fn state_dispatcher_count(d: &dispatcher::Dispatcher) -> usize {
    d.registry.count()
}

/// Load `<root>/domains/<name>/domain.yaml` files (if present) into the router.
/// If a domain has no `domain.yaml`, synthesize a stub from its name + patterns
/// derived from the directory name.
fn load_domain_definitions(root: &std::path::Path, router: &intent_router::IntentRouter) {
    let base = root.join("domains");
    if !base.exists() {
        return;
    }
    if let Ok(entries) = std::fs::read_dir(&base) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let name = match path.file_name().and_then(|n| n.to_str()) {
                Some(n) => n.to_string(),
                None => continue,
            };
            let domain_yaml = path.join("domain.yaml");
            if let Ok(raw) = std::fs::read_to_string(&domain_yaml) {
                match serde_yaml::from_str::<models::Domain>(&raw) {
                    Ok(d) => {
                        router.register(d);
                        continue;
                    }
                    Err(e) => {
                        tracing::warn!("bad domain.yaml {}: {}", domain_yaml.display(), e);
                    }
                }
            }
            // Synthesize minimal domain with keyword patterns derived from name.
            let patterns = synthesize_patterns(&name);
            let d = models::Domain {
                name: name.clone(),
                patterns,
                agents: Vec::new(),
                skills: Vec::new(),
                priority: 1,
            };
            router.register(d);
        }
    }
}

fn synthesize_patterns(name: &str) -> Vec<String> {
    let keywords: Vec<String> = match name {
        "coding" => vec!["\\bcode\\b", "\\bbug\\b", "\\bfunction\\b", "\\bapi\\b", "\\brefactor\\b"],
        "research" => vec!["\\bresearch\\b", "\\bpaper\\b", "\\bcite\\b", "\\bstudy\\b"],
        "writing" => vec!["\\bwrite\\b", "\\bessay\\b", "\\bblog\\b", "\\barticle\\b", "\\bcopy\\b"],
        "quant-trading" => vec!["\\btrade\\b", "\\bstock\\b", "\\bportfolio\\b", "\\bbtc\\b", "\\bprice\\b"],
        "media" => vec!["\\bvideo\\b", "\\baudio\\b", "\\bimage\\b", "\\bedit\\b"],
        "communication" => vec!["\\bemail\\b", "\\bmessage\\b", "\\bslack\\b", "\\btweet\\b"],
        "study-notes" => vec!["\\bstudy\\b", "\\bnotes\\b", "\\bexam\\b", "\\blearn\\b"],
        _ => vec![&format!("\\b{}\\b", name)],
    };
    keywords.into_iter().map(String::from).collect()
}
