use std::sync::Arc;
use axum::Router;
use axum::routing::get;
use crate::AppState;
use crate::state_evaluation::handlers::{guest_state};

mod handlers;

pub fn add_state_evaluation_handlers() -> Router<Arc<AppState>> {
    let router = Router::new()
        .route("/api/state/{project}/guest/{guest}", get(guest_state));

    router
}