mod auth_handlers;
pub mod files;

use std::time::Instant;

use axum::extract::State;
use axum::routing::{get, post};
use axum::Json;
use axum::Router;

use crate::auth::middleware::AuthUser;
use crate::error::AppError;
use crate::state::AppState;

pub fn auth_router() -> Router<AppState> {
    Router::new()
        .route("/auth/login", post(auth_handlers::login))
        .route("/auth/logout", post(auth_handlers::logout))
        .route("/auth/webauthn/challenge", post(auth_handlers::webauthn_challenge))
        .route("/auth/webauthn/verify", post(auth_handlers::webauthn_verify))
        .route("/auth/webauthn/register/start", post(auth_handlers::webauthn_register_start))
        .route("/auth/webauthn/register/finish", post(auth_handlers::webauthn_register_finish))
        .route("/auth/otp/verify", post(auth_handlers::otp_verify))
}

pub fn protected_router() -> Router<AppState> {
    Router::new()
        .route("/files", get(files::list_directory))
        .route("/ws/ticket", post(create_ws_ticket))
}

/// Creates a single-use, short-lived ticket for WebSocket authentication.
/// The ticket replaces passing JWT tokens in WebSocket URL query parameters,
/// which would leak the token in server logs, browser history, and Referer headers.
async fn create_ws_ticket(
    user: AuthUser,
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, AppError> {
    let ticket_id = uuid::Uuid::new_v4().to_string();
    state.ws_tickets.insert(
        ticket_id.clone(),
        crate::state::WsTicket {
            username: user.sub,
            created_at: Instant::now(),
        },
    );
    Ok(Json(serde_json::json!({ "ticket": ticket_id })))
}
