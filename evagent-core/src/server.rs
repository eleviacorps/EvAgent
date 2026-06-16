//! WebSocket server, dispatch orchestration, and engine initialization.

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

async fn ws_upgrade(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

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
        WsClientMessage::SessionDetail { session_id: _ } => Ok(()),
    }
}

async fn handle_dispatch(
    state: &Arc<AppState>,
    prompt: &str,
    context: Option<String>,
    domain_override: Option<String>,
) -> HermesResult<()> {
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
            "Low confidence routing: domain={}, confidence={:.2}",
            router_output.domain, router_output.confidence
        );
    }

    let agents = state.agent_registry.list(Some(&router_output.domain))?;
    if agents.is_empty() {
        info!("[dispatch] No agents for domain '{}', falling back to LLM chat", router_output.domain);
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

        let api_key = std::env::var("EVAGENT_API_KEY").unwrap_or_default();
        let key_usable = api_key.len() > 20 && !api_key.contains("...");
        info!("[dispatch] EVAGENT_API_KEY set={}, usable={}, len={}", !api_key.is_empty(), key_usable, api_key.len());

        let result = if key_usable {
            let base_url = std::env::var("EVAGENT_BASE_URL")
                .unwrap_or_else(|_| "https://opencode.ai/zen/v1".to_string());
            let model = std::env::var("EVAGENT_MODEL")
                .unwrap_or_else(|_| "deepseek-v4-flash-free".to_string());
            info!("[dispatch] Calling LLM: base_url={}, model={}", base_url, model);
            let llm_result = call_llm(prompt, &api_key, &base_url, &model);
            match &llm_result {
                Ok(text) => info!("[dispatch] LLM OK: {} chars", text.len()),
                Err(e) => info!("[dispatch] LLM error: {}", e),
            }
            llm_result.unwrap_or_else(|e| format!("LLM error: {}", e))
        } else {
            info!("[dispatch] No usable API key, echoing message");
            format!("[{}] Received: \"{}\"", chrono::Utc::now().format("%H:%M:%S"), prompt)
        };

        let resp = serde_json::to_string(&WsServerMessage::DispatchResult {
            session_id: session.id,
            outputs: vec![],
            aggregated: Some(result),
        }).map_err(|e| HermesError::websocket_with("Serialize error", e))?;
        let _ = state.tx.send(resp);
        return Ok(());
    }

    let session = state.session_store.create(&router_output.domain)?;
    let msg = crate::models::Message {
        id: uuid::Uuid::new_v4().to_string(),
        session_id: session.id.clone(),
        role: crate::models::MessageRole::User,
        content: prompt.to_string(),
        timestamp: chrono::Utc::now(),
        tokens: 0,
    };
    state.session_store.append_message(&session.id, msg)?;

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

    let outputs = state.dispatcher.dispatch_parallel(tasks).await?;
    let aggregated = state.dispatcher.aggregate(outputs.clone())?;

    let agent_names: Vec<String> = agents.iter().map(|a| a.name.clone()).collect();
    let _ = state.session_store.update_dispatch_agents(&session.id, &agent_names);

    let total_tokens: u64 = outputs.iter().map(|o| o.tokens_used).sum();
    let total_time: u64 = outputs.iter().map(|o| o.wall_clock_ms).sum();

    {
        let db = state.session_store.db.lock()
            .map_err(|e| HermesError::store(e.to_string()))?;
        let _ = db.execute(
            "UPDATE sessions SET total_tokens = total_tokens + ?1, wall_clock_ms = wall_clock_ms + ?2 WHERE id = ?3",
            rusqlite::params![total_tokens, total_time, session.id],
        );
    }

    let response = serde_json::to_string(&WsServerMessage::DispatchResult {
        session_id: session.id.clone(),
        outputs,
        aggregated: Some(aggregated),
    })
    .map_err(|e| HermesError::websocket_with("Serialize error", e))?;
    let _ = state.tx.send(response);

    info!("Dispatch complete for session {} ({} agents, {} tokens)",
        session.id, agents.len(), total_tokens);
    Ok(())
}

/// Initialize core stores and engines. Called during `start`.
/// LOG EVERY STEP so we can pinpoint hangs.
pub fn initialize_engine(config: &HermesConfig) -> HermesResult<Arc<AppState>> {
    info!("[init] step 1: opening SQLite database at {}", config.store.db_path);
    let conn = match rusqlite::Connection::open(&config.store.db_path) {
        Ok(c) => c,
        Err(e) => {
            error!("[init] FAILED to open database: {}", e);
            return Err(HermesError::store_with(format!("Cannot open database: {}", config.store.db_path), e));
        }
    };

    info!("[init] step 2: setting PRAGMAs");
    if let Err(e) = conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;") {
        error!("[init] PRAGMA failed: {}", e);
        return Err(HermesError::store_with("Failed to set PRAGMA", e));
    }

    let conn = Arc::new(std::sync::Mutex::new(conn));

    info!("[init] step 3: PermissionEngine");
    let permission_engine = PermissionEngine::new(conn.clone())?;

    info!("[init] step 4: SessionStore");
    let session_store = SessionStore::new(conn.clone(), config.store.session_ttl_days, config.store.archive_after_days)?;

    info!("[init] step 5: AgentRegistry");
    let agent_registry = AgentRegistry::new(conn.clone(), config.routing.max_fs_walk_depth)?;

    info!("[init] step 6: SkillLoader");
    let skill_loader = SkillLoader::new(conn.clone(), config.routing.max_fs_walk_depth)?;

    info!("[init] step 7: IntentRouter");
    let intent_router = IntentRouter::new(config.routing.min_confidence);
    if let Err(e) = register_default_domains(&intent_router) {
        error!("[init] register_default_domains failed: {}", e);
        return Err(e);
    }

    info!("[init] step 8: Dispatcher");
    let dispatcher = Dispatcher::new(
        config.dispatch.max_concurrent,
        config.core.default_timeout_secs,
        config.dispatch.default_token_budget,
    );

    info!("[init] step 9: broadcast channel");
    let (tx, _) = broadcast::channel(256);

    info!("[init] step 10: Arc<AppState>");
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

    info!("[init] step 11: spawning background agent/skill scan");
    let bg_state = state.clone();
    tokio::spawn(async move {
        info!("[bg-scan] starting agent/skill scan...");
        let agent_paths = vec![
            std::path::PathBuf::from("../domains"),
            std::path::PathBuf::from("domains"),
        ];
        bg_state.agent_registry.register_scan_paths(&agent_paths);
        match bg_state.agent_registry.scan_and_register() {
            Ok(n) => info!("[bg-scan] registered {} agents", n),
            Err(e) => warn!("[bg-scan] agent scan issue: {}", e),
        }

        let skill_paths = vec![
            std::path::PathBuf::from("../domains"),
            std::path::PathBuf::from("domains"),
        ];
        match bg_state.skill_loader.reload_index(&skill_paths) {
            Ok(n) => info!("[bg-scan] loaded {} skills", n),
            Err(e) => warn!("[bg-scan] skill load issue: {}", e),
        }
        info!("[bg-scan] scan complete");
    });

    info!("[init] DONE — returning Ok(state)");
    Ok(state)
}

/// Call the LLM for a direct response (used when no agents match)
fn call_llm(prompt: &str, api_key: &str, base_url: &str, model: &str) -> Result<String, String> {
    let url = format!("{}/chat/completions", base_url.trim_end_matches('/'));

    let body = serde_json::json!({
        "model": model,
        "messages": [{"role": "user", "content": prompt}],
        "max_tokens": 2048,
        "temperature": 0.7,
    });

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let resp = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .map_err(|e| format!("HTTP request failed: {}", e))?;

    let status = resp.status();
    if !status.is_success() {
        let text = resp.text().unwrap_or_default();
        return Err(format!("HTTP {}: {}", status, text));
    }

    let json: serde_json::Value = resp
        .json()
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    let text = json["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("")
        .to_string();

    // deepseek-v4 puts reasoning in reasoning_content, fallback to that
    let text = if text.is_empty() {
        json["choices"][0]["message"]["reasoning_content"]
            .as_str()
            .unwrap_or("")
            .to_string()
    } else {
        text
    };

    if text.is_empty() {
        Err("Empty response from LLM".to_string())
    } else {
        Ok(text)
    }
}

fn register_default_domains(router: &IntentRouter) -> HermesResult<()> {
    router.register_domain(RegisteredDomain {
        name: "coding".to_string(), patterns: vec![
            r"(?i)\bcode\b".to_string(), r"(?i)\breview\b".to_string(), r"(?i)\bbug\b".to_string(),
            r"(?i)\brefactor\b".to_string(), r"(?i)\bdeploy\b".to_string(), r"(?i)\bcommit\b".to_string(),
            r"(?i)\bmerge\b".to_string(), r"(?i)\btest\b".to_string(), r"(?i)\bdebug\b".to_string(),
            r"(?i)\brepository\b".to_string(), r"(?i)\bgit\b".to_string(), r"(?i)\bapi\b".to_string(),
            r"(?i)\bendpoint\b".to_string(), r"(?i)\bperformance\b".to_string(), r"(?i)\boptimize\b".to_string(),
            r"(?i)\brewrite\b".to_string(), r"(?i)\blint\b".to_string(), r"(?i)\bbuild\b".to_string(),
            r"(?i)\bci\b".to_string(), r"(?i)\bcd\b".to_string(), r"(?i)\bdocker\b".to_string(),
            r"(?i)\bcontainer\b".to_string(), r"(?i)\bkubernetes\b".to_string(), r"(?i)\bk8s\b".to_string(),
            r"(?i)\bplan\b".to_string(), r"(?i)\bimplementation\b".to_string(), r"(?i)\barchitect\b".to_string(),
        ], agents: vec!["planner".to_string(), "architect".to_string(), "code-reviewer".to_string(),
            "build-error-resolver".to_string(), "e2e-runner".to_string()],
    })?;
    router.register_domain(RegisteredDomain {
        name: "research".to_string(), patterns: vec![
            r"(?i)\bsearch\b".to_string(), r"(?i)\bfind\b".to_string(), r"(?i)\blookup\b".to_string(),
            r"(?i)\bresearch\b".to_string(), r"(?i)\binvestigate\b".to_string(), r"(?i)\bwhat is\b".to_string(),
            r"(?i)\bexplain\b".to_string(), r"(?i)\bsummarize\b".to_string(), r"(?i)\banalyze\b".to_string(),
            r"(?i)\bcompare\b".to_string(), r"(?i)\bdifference\b".to_string(), r"(?i)\bdefine\b".to_string(),
        ], agents: vec!["deep-researcher".to_string(), "literature-reviewer".to_string(), "competitive-analyst".to_string()],
    })?;
    router.register_domain(RegisteredDomain {
        name: "writing".to_string(), patterns: vec![
            r"(?i)\bwrite\b".to_string(), r"(?i)\bcompose\b".to_string(), r"(?i)\bdraft\b".to_string(),
            r"(?i)\bcreate\b".to_string(), r"(?i)\bpoem\b".to_string(), r"(?i)\bstory\b".to_string(),
            r"(?i)\bessay\b".to_string(), r"(?i)\barticle\b".to_string(), r"(?i)\bcontent\b".to_string(),
            r"(?i)\bcreative\b".to_string(), r"(?i)\bgenerate\b".to_string(), r"(?i)\bidea\b".to_string(),
        ], agents: vec!["content-writer".to_string(), "brand-voice-specialist".to_string(), "technical-writer".to_string()],
    })?;
    router.register_domain(RegisteredDomain {
        name: "quant-trading".to_string(), patterns: vec![
            r"(?i)\btrade\b".to_string(), r"(?i)\bmarket\b".to_string(), r"(?i)\bstock\b".to_string(),
            r"(?i)\bportfolio\b".to_string(), r"(?i)\binvest\b".to_string(), r"(?i)\bbuy\b".to_string(),
            r"(?i)\bsell\b".to_string(), r"(?i)\bprice\b".to_string(), r"(?i)\bchart\b".to_string(),
            r"(?i)\btrading\b".to_string(), r"(?i)\bstrategy\b".to_string(), r"(?i)\brisk\b".to_string(),
        ], agents: vec!["strategy-designer".to_string(), "risk-manager".to_string(), "market-analyst".to_string()],
    })?;
    router.register_domain(RegisteredDomain {
        name: "data".to_string(), patterns: vec![
            r"(?i)\bdata\b".to_string(), r"(?i)\banalytics\b".to_string(), r"(?i)\bvisualize\b".to_string(),
            r"(?i)\bdashboard\b".to_string(),
            r"(?i)\bmetrics\b".to_string(), r"(?i)\bstatistics\b".to_string(), r"(?i)\breport\b".to_string(),
            r"(?i)\bquery\b".to_string(), r"(?i)\bsql\b".to_string(), r"(?i)\bdatabase\b".to_string(),
            r"(?i)\bcsv\b".to_string(), r"(?i)\bexcel\b".to_string(),
        ], agents: vec!["market-analyst".to_string(), "deep-researcher".to_string()],
    })?;
    router.register_domain(RegisteredDomain {
        name: "general".to_string(), patterns: vec![
            r"(?i)\bhello\b".to_string(), r"(?i)\bhi\b".to_string(), r"(?i)\bhey\b".to_string(),
            r"(?i)\bwhat can you do\b".to_string(), r"(?i)\bhelp\b".to_string(),
            r"(?i)\bwho are you\b".to_string(), r"(?i)\bintroduce yourself\b".to_string(),
            r"(?i)\bthanks\b".to_string(), r"(?i)\bthank you\b".to_string(), r"(?i)\bbye\b".to_string(),
        ], agents: vec!["content-writer".to_string(), "planner".to_string(), "deep-researcher".to_string()],
    })?;
    Ok(())
}
