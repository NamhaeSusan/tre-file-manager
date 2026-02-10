use std::io::{Read, Write};
use std::sync::{Arc, Mutex};

use axum::extract::ws::{Message, WebSocket};
use axum::extract::{Query, State, WebSocketUpgrade};
use axum::response::IntoResponse;
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use futures::{SinkExt, StreamExt};
use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use serde::Deserialize;
use tokio::sync::mpsc;

use crate::state::AppState;

#[derive(Deserialize)]
pub struct TerminalQuery {
    /// Single-use ticket ID (replaces JWT token in URL to prevent token leakage in logs)
    ticket: Option<String>,
    cwd: Option<String>,
}

pub async fn terminal_handler(
    State(state): State<AppState>,
    Query(query): Query<TerminalQuery>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    let has_auth = state.config.has_auth();

    let username = if has_auth {
        // Validate single-use ticket (consume on use)
        let ticket_id = match &query.ticket {
            Some(t) => t,
            None => return axum::http::StatusCode::UNAUTHORIZED.into_response(),
        };
        let (_, ticket) = match state.ws_tickets.remove(ticket_id) {
            Some(t) => t,
            None => return axum::http::StatusCode::UNAUTHORIZED.into_response(),
        };
        // Tickets expire after 30 seconds
        if ticket.created_at.elapsed() > std::time::Duration::from_secs(30) {
            return axum::http::StatusCode::UNAUTHORIZED.into_response();
        }
        ticket.username
    } else {
        "anonymous".to_string()
    };

    let cwd = resolve_cwd(&state, query.cwd.as_deref(), &username);
    ws.on_upgrade(move |socket| handle_terminal(socket, cwd))
        .into_response()
}

fn resolve_cwd(state: &AppState, cwd: Option<&str>, username: &str) -> std::path::PathBuf {
    let root = state.config.resolve_root(username);

    if let Some(requested) = cwd {
        let path = std::path::PathBuf::from(requested);
        if let Ok(canonical) = path.canonicalize() {
            if canonical.starts_with(root) && canonical.is_dir() {
                return canonical;
            }
        }
    }

    root.clone()
}

enum PtyEvent {
    Output(Vec<u8>),
    Exit,
}

#[derive(Deserialize)]
#[serde(tag = "type")]
enum ClientMessage {
    #[serde(rename = "input")]
    Input { data: String },
    #[serde(rename = "resize")]
    Resize { cols: u16, rows: u16 },
}

async fn handle_terminal(socket: WebSocket, cwd: std::path::PathBuf) {
    let (mut ws_sender, mut ws_receiver) = socket.split();

    // Spawn PTY
    let pty_system = native_pty_system();
    let initial_size = PtySize {
        rows: 24,
        cols: 80,
        pixel_width: 0,
        pixel_height: 0,
    };

    let pair = match pty_system.openpty(initial_size) {
        Ok(p) => p,
        Err(e) => {
            let msg = serde_json::json!({
                "type": "error",
                "message": format!("Failed to open PTY: {e}")
            });
            let _ = ws_sender.send(Message::Text(msg.to_string().into())).await;
            return;
        }
    };

    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
    let mut cmd = CommandBuilder::new(&shell);
    cmd.cwd(&cwd);
    cmd.env("TERM", "xterm-256color");

    if let Err(e) = pair.slave.spawn_command(cmd) {
        let msg = serde_json::json!({
            "type": "error",
            "message": format!("Failed to spawn shell: {e}")
        });
        let _ = ws_sender.send(Message::Text(msg.to_string().into())).await;
        return;
    }

    // Drop slave to avoid holding the fd
    drop(pair.slave);

    // Wrap master in Arc<Mutex> so we can share it between resize and cleanup
    let master = Arc::new(Mutex::new(pair.master));

    let mut writer = master
        .lock()
        .unwrap()
        .take_writer()
        .expect("Failed to take PTY writer");

    let mut reader = master
        .lock()
        .unwrap()
        .try_clone_reader()
        .expect("Failed to clone PTY reader");

    // PTY reader thread -> mpsc channel
    let (pty_tx, mut pty_rx) = mpsc::unbounded_channel::<PtyEvent>();

    std::thread::spawn(move || {
        let mut buf = [0u8; 4096];
        loop {
            match reader.read(&mut buf) {
                Ok(0) => {
                    let _ = pty_tx.send(PtyEvent::Exit);
                    break;
                }
                Ok(n) => {
                    if pty_tx.send(PtyEvent::Output(buf[..n].to_vec())).is_err() {
                        break;
                    }
                }
                Err(_) => {
                    let _ = pty_tx.send(PtyEvent::Exit);
                    break;
                }
            }
        }
    });

    // Main relay loop
    let master_for_resize = Arc::clone(&master);

    loop {
        tokio::select! {
            pty_event = pty_rx.recv() => {
                match pty_event {
                    Some(PtyEvent::Output(data)) => {
                        let encoded = BASE64.encode(&data);
                        let msg = serde_json::json!({
                            "type": "output",
                            "data": encoded
                        });
                        if ws_sender.send(Message::Text(msg.to_string().into())).await.is_err() {
                            break;
                        }
                    }
                    Some(PtyEvent::Exit) | None => {
                        let msg = serde_json::json!({
                            "type": "exit",
                            "code": 0
                        });
                        let _ = ws_sender.send(Message::Text(msg.to_string().into())).await;
                        break;
                    }
                }
            }
            ws_msg = ws_receiver.next() => {
                match ws_msg {
                    Some(Ok(Message::Text(text))) => {
                        if let Ok(client_msg) = serde_json::from_str::<ClientMessage>(&text) {
                            match client_msg {
                                ClientMessage::Input { data } => {
                                    if let Ok(bytes) = BASE64.decode(&data) {
                                        let _ = writer.write_all(&bytes);
                                    }
                                }
                                ClientMessage::Resize { cols, rows } => {
                                    // Clamp dimensions to prevent resource exhaustion
                                    let cols = cols.clamp(1, 500);
                                    let rows = rows.clamp(1, 500);
                                    if let Ok(m) = master_for_resize.lock() {
                                        let _ = m.resize(PtySize {
                                            rows,
                                            cols,
                                            pixel_width: 0,
                                            pixel_height: 0,
                                        });
                                    }
                                }
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => {
                        break;
                    }
                    _ => {}
                }
            }
        }
    }

    // Cleanup: drop master which kills PTY
    drop(master);
}
