//! WebSocket server, dispatch orchestration, and engine initialization.
//! Handles bi-directional communication with the TUI and manages
//! the full dispatch lifecycle: route -> spawn subagents -> aggregate.

use std::net::SocketAddr;
use std::sync::Arc;

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
    routing::get,
    Router,
};
use futures_util::{SinkExt, StreamExt};
use tokio::sync::broadcast;
use tower_http::cors::CorsLayer;
use tracing::{debug, error, info, warn};

use crate::agent_registry::AgentRegistry;
use crate::dispatcher::{build_task, Dispatcher};
use crate::errors::{HermesError, HermesResult};
use crate::intent_router::IntentRouter;
use crate::models::{
    HermesConfig, RegisteredDomain, RouterOutput, WsClientMessage, WsServerMessage,
};
use crate::permissions::PermissionEngine;
use crate::session::SessionStore;
use crate::skill_loader::SkillLoader;

/// Shared application state accessible from all WebSocket handlers.
pub struct AppState {
    pub config: HermesConfig,
    pub agent_registry: AgentRegistry,
    pub skill_loader: SkillLoader,
    pub intent_router: IntentRouter,
    pub dispatcher: Dispatcher,
    pub session_store: SessionStore,
    pub permission_engine: PermissionEngine,
    pub tx: broadcast::Sender<String>,
}

/// Run the WebSocket server.
pub async fn run_server(state: Arc<AppState>) -> HermesResult<()> {
    let addr: SocketAddr = format!("{}:{}", state.config.server.host, state.config.server.port)
        .parse()
        .map_err(|e| {
            HermesError::config(format!(
                "Invalid server address: {}:{}",
                state.config.server.host, state.config.server.port
            ))
        })?;

    let app = Router::new()
        .route("/ws", get(ws_upgrade))
        .with_state(state.clone())
        .layer(CorsLayer::permissive());

    info!("WebSocket server starting on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| HermesError::websocket_with("Bind failed", e))?;
    axum::serve(listener, app)
        .await
        .map_err(|e| HermesError::websocket_with("Server error", e))?;
    Ok(())
}

/// WebSocket upgrade handler.
async fn ws_upgrade(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

/// Handle an individual WebSocket connection.
async fn handle_socket(socket: WebSocket, state: Arc<AppState>) {
    let mut rx = state.tx.subscribe();
    let (mut sender, mut receiver) = socket.split();

    let send_task = tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(msg) => {
                    if sender.send(Message::Text(msg.into())).await.is_err() {
                        break;
                    }
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    warn!("WebSocket receiver lagged by {} messages", n);
                }
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
    });

    let recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                Message::Text(text) => {
                    debug!("Received WebSocket message: {}", text);
                    if let Err(e) = handle_message(&state, &text).await {
                        error!("Message handler error: {}", e);
                        let err_msg = serde_json::to_string(&WsServerMessage::Error {
                            message: e.to_string(),
                        })
                        .unwrap_or_default();
                        let _ = state.tx.send(err_msg);
                    }
                }
                Message::Close(_) => break,
                _ => {}
            }
        }
    });

    tokio::select! {
        _ = send_task => {},
        _ = recv_task => {},
    };
}

/// Route an incoming WebSocket message to the appropriate handler.
async fn handle_message(state: &Arc<AppState>, raw: &str) -> HermesResult<()> {
    let client_msg: WsClientMessage =
        serde_json::from_str(raw).map_err(|e| HermesError::websocket_with("Invalid JSON", e))?;

    match client_msg {
        WsClientMessage::Ping => {
            let response = serde_json::to_string(&WsServerMessage::Pong)
                .map_err(|e| HermesError::websocket_with("Serialize error", e))?;
            let _ = state.tx.send(response);
            Ok(())
        }
        WsClientMessage::AgentList => {
            let agents = state.agent_registry.list(None)?;
            let response = serde_json::to_string(&WsServerMessage::AgentList { agents })
                .map_err(|e| HermesError::websocket_with("Serialize error", e))?;
            let _ = state.tx.send(response);
            Ok(())
        }
        WsClientMessage::SkillList => {
            let skills = state.skill_loader.list(None)?;
            let response = serde_json::to_string(&WsServerMessage::SkillList { skills })
                .map_err(|e| HermesError::websocket_with("Serialize error", e))?;
            let _ = state.tx.send(response);
            Ok(())
        }
        WsClientMessage::ConfigUpdate { key, value } => {
            info!("Config update requested: {} = {}", key, value);
            // Config updates happen at the TUI level; for now, just acknowledge
            Ok(())
        }
        WsClientMessage::SessionList => {
            let sessions = state.session_store.list(None)?;
            let response = serde_json::to_string(&WsServerMessage::SessionList { sessions })
                .map_err(|e| HermesError::websocket_with("Serialize error", e))?;
            let _ = state.tx.send(response);
            Ok(())
        }
        WsClientMessage::DispatchTask {
            goal,
            context,
            domain,
        } => handle_dispatch(state, &goal, context, domain).await,
        WsClientMessage::SessionDetail { session_id: _ } => {
            // Not yet implemented
            Ok(())
        }
    }
}

/// Core dispatch orchestration: route -> select agents -> fork subagents -> aggregate.
async fn handle_dispatch(
    state: &Arc<AppState>,
    prompt: &str,
    context: Option<String>,
    domain_override: Option<String>,
) -> HermesResult<()> {
    // Phase 1: Route the intent
    let router_output = if let Some(ref domain) = domain_override {
        let agents = state.agent_registry.list(Some(domain))?;
        RouterOutput {
            domain: domain.clone(),
            agent_candidates: agents.iter().map(|a| a.name.clone()).collect(),
            confidence: 1.0,
            matched_pattern: Some("explicit".to_string()),
            llm_fallback_used: false,
        }
    } else {
        state.intent_router.route(prompt)?
    };

    if router_output.confidence < state.config.routing.min_confidence {
        warn!(
            "Low confidence routing: domain={}, confidence={:.2}, min={:.2}",
            router_output.domain,
            router_output.confidence,
            state.config.routing.min_confidence
        );
    }

    // Phase 2: Look up agents for the resolved domain
    let agents = state.agent_registry.list(Some(&router_output.domain))?;
    if agents.is_empty() {
        return Err(HermesError::router(format!(
            "No agents found for domain '{}'",
            router_output.domain
        )));
    }

    // Phase 3: Create a session
    let mut session = state.session_store.create(&router_output.domain)?;

    let msg = crate::models::Message {
        id: uuid::Uuid::new_v4().to_string(),
        session_id: session.id.clone(),
        role: crate::models::MessageRole::User,
        content: prompt.to_string(),
        timestamp: chrono::Utc::now(),
        tokens: 0,
    };
    state.session_store.append_message(&session.id, msg)?;

    // Send SubAgentUpdate for each agent (agent_started equivalent)
    for agent in &agents {
        let start_msg = serde_json::to_string(&WsServerMessage::SubAgentUpdate {
            task_id: format!("{}-{}", session.id, agent.name),
            agent_name: agent.name.clone(),
            status: crate::models::SubAgentStatus::Completed,
            progress: Some("starting".to_string()),
            tokens_used: 0,
            wall_clock_ms: 0,
        })
        .map_err(|e| HermesError::websocket_with("Serialize error", e))?;
        let _ = state.tx.send(start_msg);
        tokio::task::yield_now().await;
    }

    // Phase 4: Build dispatch tasks for each agent
    let mut tasks = Vec::new();
    for agent in &agents {
        let agent_skills = state
            .skill_loader
            .list(Some(&router_output.domain))
            .unwrap_or_default();
        let skill_names: Vec<String> = agent_skills.iter().map(|s| s.name.clone()).collect();

        tasks.push(build_task(
            format!("{}-{}", session.id, agent.name),
            prompt.to_string(),
            context.clone().unwrap_or_default(),
            router_output.domain.clone(),
            skill_names,
            agent.permission_profile.clone(),
            state.config.core.default_timeout_secs,
            state.config.dispatch.default_token_budget,
        ));
    }

    // Phase 5: Dispatch in parallel
    let outputs = state.dispatcher.dispatch_parallel(tasks).await?;

    // Phase 6: Aggregate results
    let aggregated = state.dispatcher.aggregate(outputs.clone())?;

    // Phase 7: Update session
    let agent_names: Vec<String> = agents.iter().map(|a| a.name.clone()).collect();
    let _ = state
        .session_store
        .update_dispatch_agents(&session.id, &agent_names);

    let total_tokens: u64 = outputs.iter().map(|o| o.tokens_used).sum();
    let total_time: u64 = outputs.iter().map(|o| o.wall_clock_ms).sum();

    {
        let db = state
            .session_store
            .db
            .lock()
            .map_err(|e| HermesError::store(e.to_string()))?;
        let _ = db.execute(
            "UPDATE sessions SET total_tokens = total_tokens + ?1, wall_clock_ms = wall_clock_ms + ?2 WHERE id = ?3",
            rusqlite::params![total_tokens, total_time, session.id],
        );
    }

    // Phase 8: Send result back to TUI
    let response = serde_json::to_string(&WsServerMessage::DispatchResult {
        session_id: session.id.clone(),
        outputs,
        aggregated: Some(aggregated),
    })
    .map_err(|e| HermesError::websocket_with("Serialize error", e))?;
    let _ = state.tx.send(response);

    info!(
        "Dispatch complete for session {} ({} agents, {} tokens)",
        session.id,
        agents.len(),
        total_tokens
    );

    Ok(())
}

/// Initialize core stores and engines. Called during `start`.
pub fn initialize_engine(config: &HermesConfig) -> HermesResult<Arc<AppState>> {
    let conn = rusqlite::Connection::open(&config.store.db_path).map_err(|e| {
        HermesError::store_with(
            format!("Cannot open database: {}", config.store.db_path),
            e,
        )
    })?;

    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")
        .map_err(|e| HermesError::store_with("Failed to set PRAGMA", e))?;

    let conn = Arc::new(std::sync::Mutex::new(conn));

    let permission_engine = PermissionEngine::new(conn.clone())?;
    let session_store = SessionStore::new(
        conn.clone(),
        config.store.session_ttl_days,
        config.store.archive_after_days,
    )?;
    let agent_registry = AgentRegistry::new(conn.clone(), config.routing.max_fs_walk_depth)?;
    let skill_loader = SkillLoader::new(conn.clone(), config.routing.max_fs_walk_depth)?;

    let intent_router = IntentRouter::new(config.routing.min_confidence);
    register_default_domains(&intent_router)?;

    let dispatcher = Dispatcher::new(
        config.dispatch.max_concurrent,
        config.core.default_timeout_secs,
        config.dispatch.default_token_budget,
    );

    let (tx, _) = broadcast::channel(256);

    let state = Arc::new(AppState {
        config: config.clone(),
        agent_registry,
        skill_loader,
        intent_router,
        dispatcher,
        session_store,
        permission_engine,
        tx,
    });

    let agent_paths = vec![
        std::path::PathBuf::from("evagent/agents"),
        std::path::PathBuf::from("evagent/domains"),
    ];
    state.agent_registry.register_scan_paths(&agent_paths);
    if let Err(e) = state.agent_registry.scan_and_register() {
        warn!("Agent scan issue: {}", e);
    }

    let skill_paths = vec![
        std::path::PathBuf::from("evagent/skills"),
        std::path::PathBuf::from("evagent/domains"),
    ];
    if let Err(e) = state.skill_loader.reload_index(&skill_paths) {
        warn!("Skill load issue: {}", e);
    }

    info!(
        "Engine initialized: agents={}, skills={}, domains={}",
        state
            .agent_registry
            .list(None)
            .map(|a| a.len())
            .unwrap_or(0),
        state
            .skill_loader
            .list(None)
            .map(|s| s.len())
            .unwrap_or(0),
        state.intent_router.get_domains().len(),
    );

    Ok(state)
}

fn register_default_domains(router: &IntentRouter) -> HermesResult<()> {
    router.register_domain(RegisteredDomain {
        name: "general".to_string(),
        patterns: vec![
            r"(?i)\bhello\b".to_string(),
            r"(?i)\bhi\b".to_string(),
            r"(?i)\bhey\b".to_string(),
            r"(?i)\bwhat can you do\b".to_string(),
            r"(?i)\bhelp\b".to_string(),
            r"(?i)\bwho are you\b".to_string(),
            r"(?i)\bintroduce yourself\b".to_string(),
            r"(?i)\bthanks\b".to_string(),
            r"(?i)\bthank you\b".to_string(),
            r"(?i)\bbye\b".to_string(),
        ],
        agents: vec!["general_assistant".to_string()],
    })?;

    router.register_domain(RegisteredDomain {
        name: "engineering".to_string(),
        patterns: vec![
            r"(?i)\bcode\b".to_string(),
            r"(?i)\breview\b".to_string(),
            r"(?i)\bbug\b".to_string(),
            r"(?i)\brefactor\b".to_string(),
            r"(?i)\bdeploy\b".to_string(),
            r"(?i)\bcommit\b".to_string(),
            r"(?i)\bmerge\b".to_string(),
            r"(?i)\btest\b".to_string(),
            r"(?i)\bdebug\b".to_string(),
            r"(?i)\brepository\b".to_string(),
            r"(?i)\bgit\b".to_string(),
            r"(?i)\bapi\b".to_string(),
            r"(?i)\bendpoint\b".to_string(),
            r"(?i)\bperformance\b".to_string(),
            r"(?i)\boptimize\b".to_string(),
            r"(?i)\brewrite\b".to_string(),
            r"(?i)\blint\b".to_string(),
            r"(?i)\bbuild\b".to_string(),
            r"(?i)\bci\b".to_string(),
            r"(?i)\bcd\b".to_string(),
            r"(?i)\bdocker\b".to_string(),
            r"(?i)\bcontainer\b".to_string(),
            r"(?i)\bkubernetes\b".to_string(),
            r"(?i)\bk8s\b".to_string(),
        ],
        agents: vec!["code_reviewer".to_string(), "devops_agent".to_string()],
    })?;

    router.register_domain(RegisteredDomain {
        name: "research".to_string(),
        patterns: vec![
            r"(?i)\bsearch\b".to_string(),
            r"(?i)\bfind\b".to_string(),
            r"(?i)\blookup\b".to_string(),
            r"(?i)\bresearch\b".to_string(),
            r"(?i)\binvestigate\b".to_string(),
            r"(?i)\bwhat is\b".to_string(),
            r"(?i)\bexplain\b".to_string(),
            r"(?i)\bsummarize\b".to_string(),
            r"(?i)\banalyze\b".to_string(),
            r"(?i)\bcompare\b".to_string(),
            r"(?i)\bdifference\b".to_string(),
            r"(?i)\bdefine\b".to_string(),
        ],
        agents: vec!["research_agent".to_string()],
    })?;

    router.register_domain(RegisteredDomain {
        name: "creative".to_string(),
        patterns: vec![
            r"(?i)\bwrite\b".to_string(),
            r"(?i)\bcompose\b".to_string(),
            r"(?i)\bdraft\b".to_string(),
            r"(?i)\bcreate\b".to_string(),
            r"(?i)\bpoem\b".to_string(),
            r"(?i)\bstory\b".to_string(),
            r"(?i)\bessay\b".to_string(),
            r"(?i)\barticle\b".to_string(),
            r"(?i)\bcontent\b".to_string(),
            r"(?i)\bcreative\b".to_string(),
            r"(?i)\bgenerate\b".to_string(),
            r"(?i)\bidea\b".to_string(),
        ],
        agents: vec!["writer_agent".to_string()],
    })?;

    router.register_domain(RegisteredDomain {
        name: "data".to_string(),
        patterns: vec![
            r"(?i)\bdata\b".to_string(),
            r"(?i)\banalytics\b".to_string(),
            r"(?i)\bvisualize\b".to_string(),
            r"(?i)\bchart\b".to_string(),
            r"(?i)\bgraph\b".to_string(),
            r"(?i)\bdashboard\b".to_string(),
            r"(?i)\bmetrics\b".to_string(),
            r"(?i)\bstatistics\b".to_string(),
            r"(?i)\breport\b".to_string(),
            r"(?i)\bquery\b".to_string(),
            r"(?i)\bsql\b".to_string(),
            r"(?i)\bdatabase\b".to_string(),
            r"(?i)\bcsv\b".to_string(),
            r"(?i)\bexcel\b".to_string(),
        ],
        agents: vec!["data_analyst".to_string()],
    })?;

    Ok(())
}
