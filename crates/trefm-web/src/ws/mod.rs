mod terminal;

use axum::routing::get;
use axum::Router;

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new().route("/terminal", get(terminal::terminal_handler))
}
