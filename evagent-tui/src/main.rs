use std::io;

use clap::Parser;
use color_eyre::eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::ExecutableCommand;
use ratatui::{backend::CrosstermBackend, Terminal};
use tokio::sync::mpsc;
use tracing_subscriber;

mod app;
mod types;
mod ui;
mod websocket;

use app::App;
use types::WsServerMessage;
use websocket::WsClient;

/// EvAgent Terminal TUI — Multi-domain orchestration system client.
#[derive(Parser, Debug)]
#[command(name = "evagent-tui", version = "0.1.0", about = "EvAgent terminal TUI client")]
struct Cli {
    /// WebSocket host to connect to
    #[arg(long, default_value = "127.0.0.1")]
    host: String,

    /// WebSocket port to connect to
    #[arg(long, default_value = "9753")]
    port: u16,

    /// Full WebSocket URL (overrides host/port)
    #[arg(long)]
    connect: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize color-eyre for better error reporting
    color_eyre::install()?;

    // Initialize tracing (disabled by default, enable with RUST_LOG=info)
    tracing_subscriber::fmt::init();

    // Parse CLI arguments
    let cli = Cli::parse();

    // Build the WebSocket URL
    let ws_url = cli.connect.unwrap_or_else(|| {
        format!("ws://{}:{}", cli.host, cli.port)
    });

    // Create the WebSocket client
    let ws_client = WsClient::new(ws_url);
    let ws_send = ws_client.write_tx.clone();
    let mut ws_recv = ws_client.read_rx;

    // Create the application state
    let mut app = App::new();

    // Set up terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    stdout.execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Ensure cleanup on panic
    let res = run_app(&mut terminal, &mut app, &ws_send, &mut ws_recv).await;

    // Cleanup
    disable_raw_mode()?;
    terminal.backend_mut().execute(LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    // Report any errors
    if let Err(e) = res {
        eprintln!("Error: {}", e);
    }

    Ok(())
}

/// Main application event loop.
async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    ws_send: &mpsc::UnboundedSender<String>,
    ws_recv: &mut mpsc::UnboundedReceiver<WsServerMessage>,
) -> Result<()> {
    // Initial data request will happen on first connect ping
    let mut last_tick = std::time::Instant::now();
    let tick_rate = std::time::Duration::from_millis(250);

    loop {
        // Draw the UI
        terminal.draw(|f| ui::draw(f, app))?;

        // Check for WebSocket messages (non-blocking)
        loop {
            match ws_recv.try_recv() {
                Ok(msg) => {
                    app.update_from_ws(msg);

                    // If we just got connected, request initial data
                    if app.connection_status == types::ConnectionState::Connected && !app.initialized {
                        app.request_initial_data(ws_send);
                    }
                }
                Err(mpsc::error::TryRecvError::Empty) => break,
                Err(mpsc::error::TryRecvError::Disconnected) => {
                    // Channel closed — server loop died
                    app.connection_status = types::ConnectionState::Disconnected;
                    break;
                }
            }
        }

        // Periodic tick
        let now = std::time::Instant::now();
        if now - last_tick >= tick_rate {
            app.tick();
            last_tick = now;
        }

        // Handle keyboard events
        if event::poll(std::time::Duration::from_millis(50))? {
            let evt = event::read()?;

            match evt {
                Event::Key(key) => {
                    // Only process press events (not releases)
                    if key.kind != KeyEventKind::Press {
                        continue;
                    }

                    match key.code {
                        // Quit: Ctrl+C or Esc
                        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            break;
                        }
                        KeyCode::Esc => {
                            break;
                        }

                        // Enter: dispatch the prompt
                        KeyCode::Enter => {
                            if app.connection_status == types::ConnectionState::Connected {
                                app.dispatch_prompt(ws_send);
                            }
                        }

                        // Tab: cycle domains
                        KeyCode::Tab => {
                            let domains = [
                                "general", "research", "coding", "writing",
                                "trading", "study", "communication", "media",
                            ];
                            if let Some(pos) = domains.iter().position(|d| *d == app.domain) {
                                let next = (pos + 1) % domains.len();
                                app.set_domain(domains[next]);
                            }
                        }

                        // Backspace: delete character before cursor
                        KeyCode::Backspace => {
                            if !app.input.is_empty() && app.input_cursor > 0 {
                                let idx = app.input_cursor;
                                app.input.remove(idx - 1);
                                app.input_cursor -= 1;
                            }
                        }

                        // Delete: delete character at cursor
                        KeyCode::Delete => {
                            if app.input_cursor < app.input.len() {
                                app.input.remove(app.input_cursor);
                            }
                        }

                        // Left arrow: move cursor left
                        KeyCode::Left => {
                            if app.input_cursor > 0 {
                                app.input_cursor -= 1;
                            }
                        }

                        // Right arrow: move cursor right
                        KeyCode::Right => {
                            if app.input_cursor < app.input.len() {
                                app.input_cursor += 1;
                            }
                        }

                        // Home: go to start
                        KeyCode::Home => {
                            app.input_cursor = 0;
                        }

                        // End: go to end
                        KeyCode::End => {
                            app.input_cursor = app.input.len();
                        }

                        // Char: insert at cursor position
                        KeyCode::Char(ch) => {
                            if app.connection_status == types::ConnectionState::Connected {
                                app.input.insert(app.input_cursor, ch);
                                app.input_cursor += 1;
                            }
                        }

                        _ => {}
                    }
                }
                Event::Resize(_, _) => {
                    // Terminal will be redrawn on next draw()
                }
                _ => {}
            }
        }
    }

    Ok(())
}
