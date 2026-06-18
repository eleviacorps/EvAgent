//! WebSocket server (axum) + HTTP routes.
//!
//! - `GET /ws` — WebSocket endpoint speaking the EvAgent protocol.
//! - `GET /` and `GET /health` — simple JSON status probe.
//! - `GET /domains` — list registered domains.
//! - `GET /agents` — list registered agents.
//! - `GET /skills` — list registered skills.

use crate::config::Config;
use crate::dispatcher::Dispatcher;
use crate::errors::Result;
use crate::intent_router::IntentRouter;
use crate::models::{ClientMessage, DispatchTask, ServerMessage};
use crate::session::SessionStore;
use futures_util::{SinkExt, StreamExt};
use parking_lot::Mutex;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::sync::mpsc;
use uuid::Uuid;

#[derive(Clone)]
pub struct AppState {
    pub cfg: Arc<Config>,
    pub router: Arc<IntentRouter>,
    pub dispatcher: Arc<Dispatcher>,
    pub store: SessionStore,
    pub subscribers: Arc<Mutex<Vec<broadcast::Sender<ServerMessage>>>>,
}

impl AppState {
    pub fn new(
        cfg: Arc<Config>,
        router: Arc<IntentRouter>,
        dispatcher: Arc<Dispatcher>,
        store: SessionStore,
    ) -> Self {
        Self {
            cfg,
            router,
            dispatcher,
            store,
            subscribers: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<ServerMessage> {
        let (tx, rx) = broadcast::channel(256);
        self.subscribers.lock().push(tx);
        rx
    }
}

pub async fn serve(state: AppState) -> Result<()> {
    let addr = format!("{}:{}", state.cfg.server.host, state.cfg.server.port);
    let app = build_router(state.clone());
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .map_err(|e| crate::errors::EvAgentError::Internal(format!("bind {}: {}", addr, e)))?;
    tracing::info!("EvAgent core listening on http://{}", addr);
    axum::serve(listener, app)
        .await
        .map_err(|e| crate::errors::EvAgentError::Internal(format!("serve: {}", e)))?;
    Ok(())
}

fn build_router(state: AppState) -> axum::Router {
    use axum::routing::{get, post};
    use tower_http::cors::CorsLayer;

    axum::Router::new()
        .route("/ws", get(ws_handler))
        .route("/health", get(health))
        .route("/", get(health))
        .route("/domains", get(list_domains))
        .route("/agents", get(list_agents))
        .route("/skills", get(list_skills))
        .route("/dispatch", post(http_dispatch))
        .layer(CorsLayer::permissive())
        .with_state(state)
}

async fn health() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({
        "service": "evagent-core",
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION")
    }))
}

async fn list_domains(
    axum::State(state): axum::State<AppState>,
) -> axum::Json<serde_json::Value> {
    let domains: Vec<_> = state.router.domains().into_iter().collect();
    axum::Json(serde_json::json!({"domains": domains}))
}

async fn list_agents(
    axum::State(state): axum::State<AppState>,
) -> axum::Json<serde_json::Value> {
    let agents = state.dispatcher.registry.all();
    axum::Json(serde_json::json!({"agents": agents, "count": agents.len()}))
}

async fn list_skills(
    axum::State(state): axum::State<AppState>,
) -> axum::Json<serde_json::Value> {
    let skills = state.dispatcher.skills.all();
    axum::Json(serde_json::json!({"skills": skills, "count": skills.len()}))
}

#[derive(serde::Deserialize)]
struct HttpDispatchBody {
    goal: String,
    #[serde(default)]
    context: Option<serde_json::Value>,
    #[serde(default = "default_domain_str")]
    domain: String,
}
fn default_domain_str() -> String {
    "general".into()
}

async fn http_dispatch(
    axum::State(state): axum::State<AppState>,
    axum::Json(body): axum::Json<HttpDispatchBody>,
) -> axum::Json<serde_json::Value> {
    let session_id = Uuid::new_v4();
    let _ = state.store.create_session(&session_id.to_string(), &body.domain).await;
    let task = DispatchTask {
        goal: body.goal.clone(),
        context: body.context,
        domain: body.domain,
        session_id,
    };
    let (tx, _rx) = broadcast::channel(256);
    let outputs = match state.dispatcher.dispatch(task, tx).await {
        Ok(o) => o,
        Err(e) => {
            return axum::Json(serde_json::json!({"error": e.to_string()}));
        }
    };
    let aggregated = outputs
        .iter()
        .map(|o| format!("## {}\n\n{}", o.agent_name, o.output))
        .collect::<Vec<_>>()
        .join("\n\n---\n\n");
    axum::Json(serde_json::json!({
        "session_id": session_id.to_string(),
        "outputs": outputs,
        "aggregated": aggregated
    }))
}

async fn ws_handler(
    ws: axum::extract::WebSocketUpgrade,
    axum::State(state): axum::State<AppState>,
) -> axum::response::Response {
    ws.on_upgrade(|socket| handle_ws(socket, state))
}

async fn handle_ws(socket: axum::extract::ws::WebSocket, state: AppState) {
    let (mut ws_tx, mut ws_rx) = socket.split();
    let (out_tx, mut out_rx) = mpsc::channel::<String>(256);
    let mut broadcast_rx = state.subscribe();

    // Outgoing pump: forward queued messages to the WebSocket.
    let send_task = tokio::spawn(async move {
        loop {
            let next: Option<String> = tokio::select! {
                m = out_rx.recv() => m,
                m = broadcast_rx.recv() => match m {
                    Ok(msg) => Some(serde_json::to_string(&msg).unwrap_or_default()),
                    Err(_) => None,
                },
            };
            match next {
                Some(s) => {
                    if ws_tx.send(axum::extract::ws::Message::Text(s)).await.is_err() {
                        break;
                    }
                }
                None => break,
            }
        }
    });

    // Incoming pump: parse ClientMessage, dispatch, push results to out_tx.
    while let Some(Ok(msg)) = ws_rx.next().await {
        let text = match msg {
            axum::extract::ws::Message::Text(t) => t,
            axum::extract::ws::Message::Binary(b) => String::from_utf8_lossy(&b).to_string(),
            axum::extract::ws::Message::Close(_) => break,
            _ => continue,
        };
        let parsed: ClientMessage = match serde_json::from_str(&text) {
            Ok(m) => m,
            Err(e) => {
                let _ = out_tx
                    .send(serde_json::to_string(&ServerMessage::Error {
                        message: format!("invalid message: {}", e),
                    })
                    .unwrap_or_default())
                    .await;
                continue;
            }
        };
        match parsed {
            ClientMessage::Ping => {
                let _ = out_tx
                    .send(serde_json::to_string(&ServerMessage::Pong).unwrap_or_default())
                    .await;
            }
            ClientMessage::DispatchTask {
                goal,
                context,
                domain,
            } => {
                let domain = if domain == "general" {
                    // Auto-route when the client didn't specify.
                    match state.router.route(&goal).await {
                        Ok(r) => r.domain,
                        Err(_) => "general".into(),
                    }
                } else {
                    domain
                };
                let session_id = Uuid::new_v4();
                let _ = state
                    .store
                    .create_session(&session_id.to_string(), &domain)
                    .await;
                let _ = state
                    .store
                    .append_message(&session_id.to_string(), "user", &goal)
                    .await;

                let task = DispatchTask {
                    goal: goal.clone(),
                    context,
                    domain: domain.clone(),
                    session_id,
                };
                let dispatcher = state.dispatcher.clone();
                let out_tx2 = out_tx.clone();
                let (tx, _rx) = broadcast::channel(256);
                tokio::spawn(async move {
                    let result = dispatcher.dispatch(task, tx).await;
                    match result {
                        Ok(outputs) => {
                            let aggregated = outputs
                                .iter()
                                .map(|o| format!("## {}\n\n{}", o.agent_name, o.output))
                                .collect::<Vec<_>>()
                                .join("\n\n---\n\n");
                            let msg = ServerMessage::DispatchResult {
                                session_id: session_id.to_string(),
                                outputs,
                                aggregated,
                            };
                            let _ = out_tx2
                                .send(serde_json::to_string(&msg).unwrap_or_default())
                                .await;
                        }
                        Err(e) => {
                            let msg = ServerMessage::Error {
                                message: e.to_string(),
                            };
                            let _ = out_tx2
                                .send(serde_json::to_string(&msg).unwrap_or_default())
                                .await;
                        }
                    }
                });
            }
        }
    }

    send_task.abort();
}
