use std::sync::Arc;
use std::time::Instant;

use dashmap::DashMap;

use crate::auth::session::SessionStore;
use crate::auth::webauthn_manager::WebAuthnManager;
use crate::config::ServerConfig;

/// A single-use, short-lived ticket for WebSocket authentication.
/// Replaces passing JWT tokens in URL query parameters.
pub struct WsTicket {
    pub username: String,
    pub created_at: Instant,
}

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<ServerConfig>,
    pub session_store: Arc<SessionStore>,
    pub webauthn: Option<Arc<WebAuthnManager>>,
    pub ws_tickets: Arc<DashMap<String, WsTicket>>,
}
