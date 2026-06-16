#![allow(dead_code)]
use futures_util::{SinkExt, StreamExt};
use std::time::Duration;
use tokio::sync::mpsc;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;
use tracing::{error, info, warn};

use crate::types::WsServerMessage;

/// WebSocket client manager with auto-reconnect (exponential backoff).
pub struct WsClient {
    /// Send raw JSON strings to the WebSocket (buffered in channel if disconnected).
    pub write_tx: mpsc::UnboundedSender<String>,
    /// Receive parsed server messages.
    pub read_rx: mpsc::UnboundedReceiver<WsServerMessage>,
}

impl WsClient {
    /// Create a new WebSocket client and start a background connect/reconnect loop.
    ///
    /// The background task owns the single outgoing channel receiver. Messages
    /// buffered in the channel are sent on reconnection. Incoming messages are
    /// parsed and forwarded to the returned receiver.
    pub fn new(url: String) -> Self {
        let (outgoing_tx, outgoing_rx) = mpsc::unbounded_channel::<String>();
        let (incoming_tx, incoming_rx) = mpsc::unbounded_channel::<WsServerMessage>();

        tokio::spawn(run_ws_loop(url, outgoing_rx, incoming_tx.clone()));

        WsClient {
            write_tx: outgoing_tx,
            read_rx: incoming_rx,
        }
    }

    /// Send a JSON string to the WebSocket server (queued if disconnected).
    pub fn send(&self, msg: &str) {
        let _ = self.write_tx.send(msg.to_string());
    }

    /// Receive the next parsed message asynchronously.
    pub async fn recv(&mut self) -> Option<WsServerMessage> {
        self.read_rx.recv().await
    }
}

/// Background loop: connect (with backoff), then read/write until connection drops.
async fn run_ws_loop(
    url: String,
    mut outgoing_rx: mpsc::UnboundedReceiver<String>,
    incoming_tx: mpsc::UnboundedSender<WsServerMessage>,
) {
    let mut backoff = Duration::from_secs(1);
    let max_backoff = Duration::from_secs(30);

    loop {
        info!("Connecting to {}...", url);
        match connect_async(&url).await {
            Ok((ws_stream, _)) => {
                info!("Connected to {}", url);
                backoff = Duration::from_secs(1);

                let (mut write, mut read) = ws_stream.split();

                // Notify connected
                let _ = incoming_tx.send(WsServerMessage::Pong);

                // Read/write loop for this connection
                loop {
                    tokio::select! {
                        // Outgoing message from the application
                        outgoing = outgoing_rx.recv() => {
                            match outgoing {
                                Some(text) => {
                                    if let Err(e) = write.send(Message::Text(text)).await {
                                        warn!("WebSocket write error: {}", e);
                                        break;
                                    }
                                }
                                None => {
                                    info!("Outgoing channel closed, shutting down WS loop");
                                    return; // channel closed = app shutdown
                                }
                            }
                        }
                        // Incoming message from the server
                        incoming = read.next() => {
                            match incoming {
                                Some(Ok(Message::Text(text))) => {
                                    match serde_json::from_str::<WsServerMessage>(&text) {
                                        Ok(msg) => {
                                            if incoming_tx.send(msg).is_err() {
                                                // App dropped the receiver; shutdown
                                                return;
                                            }
                                        }
                                        Err(e) => {
                                            warn!("Failed to parse server message: {} — {}", e, text);
                                        }
                                    }
                                }
                                Some(Ok(Message::Close(_))) => {
                                    info!("WebSocket closed by server");
                                    break;
                                }
                                Some(Ok(_)) => { /* ignore binary/ping/pong */ }
                                Some(Err(e)) => {
                                    warn!("WebSocket read error: {}", e);
                                    break;
                                }
                                None => {
                                    info!("WebSocket stream ended");
                                    break;
                                }
                            }
                        }
                    }
                }

                // Notify disconnection — only if we were previously connected
                let _ = incoming_tx.send(WsServerMessage::Error {
                    message: "reconnect".into(),
                });
            }
            Err(e) => {
                error!("Connection failed: {}; retrying in {:?}", e, backoff);
                // Send one-time notification on first failure only
                if backoff == Duration::from_secs(1) {
                    let _ = incoming_tx.send(WsServerMessage::Error {
                        message: "connect".to_string(),
                    });
                }
            }
        }

        // Exponential backoff before reconnecting
        tokio::time::sleep(backoff).await;
        backoff = std::cmp::min(backoff * 2, max_backoff);
    }
}
