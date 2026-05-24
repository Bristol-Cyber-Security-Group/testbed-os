use crate::{AppError, AppState};
use anyhow::{anyhow, Context};
use axum::extract::{Path, State};
use axum::{
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use kvm_compose_lib::state::state_evaluation::{EvaluateState, StateComponentStatus};
use serde_json::json;
use std::sync::Arc;


#[axum::debug_handler]
pub async fn guest_state(
    State(db_config): State<Arc<AppState>>,
    Path((project, component)): Path<(String, String)>,
) -> Result<impl IntoResponse, AppError> {

    // first check if project exists
    db_config.deployment_config_db
        .read()
        .await
        .get_deployment(project.clone())
        .await
        .context("Deployment does not exist")?;

    // if project exists, then lets get the state json
    let state_json = db_config.deployment_config_db
        .read()
        .await
        .get_state(project.clone())
        .await
        .context("Deployment exists but state json does not")?;

    let guest = state_json.testbed_guests.0
        .get(&component)
        .ok_or(anyhow!("Guest {} does not exist", component))?;

    // get the name of the host that holds this guest
    let guest_host = guest.testbed_host.clone().ok_or(anyhow!("Guest {} not assigned to testbed host", component))?;
    // get the cluster config with all host information
    let cluster_config = db_config.config_db
        .read()
        .await
        .get_cluster_config()
        .await?;
    // get the host's config
    let host_config = cluster_config.testbed_host_ssh_config
        .get(&guest_host)
        .ok_or(anyhow!("Guest's host {} does not exist", guest_host))?;
    // if this host config is "main" then we can poll from this as this is the "main" server as well
    // but if it is not main, then we need to relay the state request to the correct testbed server
    let guest_exists = if host_config.is_main_host.ok_or(anyhow!("Host {} has not been given an if main", guest_host))? {
        // on main
        guest.check_exists(project).await?
    } else {
        unimplemented!()
    };
    match guest_exists {
        StateComponentStatus::Up => {
            let body = json!({
                    "status": "up"
                });
            Ok((StatusCode::OK, Json(body)).into_response())
        }
        StateComponentStatus::Down(down_state) => {
            let body = json!({
                    "status": "down",
                    "state_info": down_state,
                });
            Ok((StatusCode::OK, Json(body)).into_response())
        }
        StateComponentStatus::DoesNotExist => {
            let body = json!({
                    "status": "does_not_exist"
                });
            Ok((StatusCode::OK, Json(body)).into_response())
        }
    }
}
