use std::sync::Arc;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use axum::response::IntoResponse;
use kvm_compose_schemas::settings::SshConfig;
use crate::{AppError, AppState};
use crate::cluster::manage::{manage_cluster};
use crate::cluster::ClusterOperation;

/// Client testbed will send it's own host.json config to the master, the master will then add it
/// to it's testbed cluster config
pub async fn join_cluster(
    State(db_config): State<Arc<AppState>>,
    Json(config): Json<SshConfig>,
) -> Result<impl IntoResponse, AppError> {
    tracing::info!("client testbed has requested to join cluster");
    // process_join_request(config).await?;
    manage_cluster(&ClusterOperation::Join(config), &db_config.config_db).await?;
    // return the ip of the OVN SB DB for the client testbed
    let local_ip = &db_config.config_db
        .read()
        .await
        .get_host_config()
        .await?
        .ip;
    let sb_remote = format!("tcp:{local_ip}:6642");

    tracing::info!("adding client to cluster complete");
    Ok(sb_remote)
}

/// Client testbeds will call this endpoint to check if they are part of the cluster periodically.
/// This will respond if they are part of the cluster or not, so that the client can then make a
/// request to join the cluster.
pub async fn check_membership(
    State(db_config): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let cluster_config = db_config.config_db
        .read()
        .await
        .get_cluster_config()
        .await?;
    let client_membership = cluster_config.testbed_host_ssh_config.get(&name);
    if client_membership.is_some() {
        Ok(StatusCode::OK)
    } else {
        Ok(StatusCode::NOT_FOUND)
    }
}
