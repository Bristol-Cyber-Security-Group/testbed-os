use std::sync::Arc;
use axum::Router;
use axum::routing::get;
use crate::AppState;
use crate::state_evaluation::handlers::*;

mod handlers;

pub fn add_state_evaluation_handlers() -> Router<Arc<AppState>> {
    let router = Router::new()
        .route("/api/state/{project}/guest/{guest}", get(guest_state))
        .route("/api/state/{project}/ls/{component}", get(logical_switch_state))
        .route("/api/state/{project}/lsp/{component}", get(logical_switch_port_state))
        .route("/api/state/{project}/lr/{component}", get(logical_router_state))
        .route("/api/state/{project}/lrp/{component}", get(logical_router_port_state))
        .route("/api/state/{project}/acl/{component}", get(logical_acl_state))
        .route("/api/state/{project}/dhcp/{component}", get(logical_dhcp_state))
        .route("/api/state/{project}/ovsport/{component}", get(ovs_port_state));

    router
}
