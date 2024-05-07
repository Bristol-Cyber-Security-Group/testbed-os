use std::sync::Arc;
use axum::Router;
use axum::routing::*;
use crate::AppState;
use crate::orchestration::handlers::*;

pub mod handlers;
pub mod websocket;

pub fn add_orchestration_handlers() -> Router<Arc<AppState>> {
    Router::new()
        // .route("/setup", post(preflight_setup))
        .route("/ws", get(orchestration_websocket_handler))
        .route("/gui", get(gui_orchestration_websocket_handler))
}
