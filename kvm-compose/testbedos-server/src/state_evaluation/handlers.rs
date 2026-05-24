use crate::{AppError, AppState};
use anyhow::{anyhow, bail, Context};
use axum::extract::{Path, State};
use axum::{
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use kvm_compose_lib::state::state_evaluation::{EvaluateState, StateComponentStatus};
use serde_json::json;
use std::sync::Arc;
use axum::response::Response;
use kvm_compose_lib::state::schema::StateNetwork;

fn get_status_response(guest_exists: StateComponentStatus) -> Response {
    match guest_exists {
        StateComponentStatus::Up => {
            let body = json!({
                    "status": "up"
                });
            (StatusCode::OK, Json(body)).into_response()
        }
        StateComponentStatus::Down(down_state) => {
            let body = json!({
                    "status": "down",
                    "state_info": down_state,
                });
            (StatusCode::OK, Json(body)).into_response()
        }
        StateComponentStatus::DoesNotExist => {
            let body = json!({
                    "status": "does_not_exist"
                });
            (StatusCode::NOT_FOUND, Json(body)).into_response()
        }
    }
}

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
    Ok(get_status_response(guest_exists))
}


pub async fn logical_switch_state(
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

    // network components saved as their full name with project prepended
    let name = format!("{}-{}", project, component);

    match state_json.network {
        StateNetwork::Ovn(ovn) => {
            let ls = ovn.switches
                .get(&name)
                .ok_or(anyhow!("Logical Switch {} does not exist", name))?
                .check_exists(project).await?;
            Ok(get_status_response(ls))
        }
        StateNetwork::Ovs(_) => {
            Ok((StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"status": "OVS network type not supported for logical switch"}))).into_response())
        },
    }
}

pub async fn logical_switch_port_state(
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

    // network components saved as their full name with project prepended
    let name = format!("{}-{}", project, component);

    match state_json.network {
        StateNetwork::Ovn(ovn) => {
            let lsp = ovn.switch_ports
                .get(&name)
                .ok_or(anyhow!("Logical Switch Port {} does not exist", name))?
                .check_exists(project).await?;
            Ok(get_status_response(lsp))
        }
        StateNetwork::Ovs(_) => {
            Ok((StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"status": "OVS network type not supported for logical switch port"}))).into_response())
        },
    }
}

pub async fn logical_router_state(
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

    // network components saved as their full name with project prepended
    let name = format!("{}-{}", project, component);

    match state_json.network {
        StateNetwork::Ovn(ovn) => {
            let lsp = ovn.routers
                .get(&name)
                .ok_or(anyhow!("Logical Router {} does not exist", name))?
                .check_exists(project).await?;
            Ok(get_status_response(lsp))
        }
        StateNetwork::Ovs(_) => {
            Ok((StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"status": "OVS network type not supported for logical router"}))).into_response())
        },
    }
}

pub async fn logical_router_port_state(
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

    // network components saved as their full name with project prepended
    let name = format!("{}-{}", project, component);

    match state_json.network {
        StateNetwork::Ovn(ovn) => {
            let lsp = ovn.router_ports
                .get(&name)
                .ok_or(anyhow!("Logical Router Port {} does not exist", name))?
                .check_exists(project).await?;
            Ok(get_status_response(lsp))
        }
        StateNetwork::Ovs(_) => {
            Ok((StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"status": "OVS network type not supported for logical router port"}))).into_response())
        },
    }
}

pub async fn logical_acl_state(
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

    // network components saved as their full name with project prepended
    let name = format!("{}-{}", project, component);

    match state_json.network {
        StateNetwork::Ovn(ovn) => {
            let lsp = ovn.acl
                .get(&name)
                .ok_or(anyhow!("ACL {} does not exist", name))?
                .check_exists(project).await?;
            Ok(get_status_response(lsp))
        }
        StateNetwork::Ovs(_) => {
            Ok((StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"status": "OVS network type not supported for ACL"}))).into_response())
        },
    }
}

pub async fn logical_dhcp_state(
    State(db_config): State<Arc<AppState>>,
    Path((project, component)): Path<(String, String)>,
) -> Result<impl IntoResponse, AppError> {
    Ok(())
    // // first check if project exists
    // db_config.deployment_config_db
    //     .read()
    //     .await
    //     .get_deployment(project.clone())
    //     .await
    //     .context("Deployment does not exist")?;
    //
    // // if project exists, then lets get the state json
    // let state_json = db_config.deployment_config_db
    //     .read()
    //     .await
    //     .get_state(project.clone())
    //     .await
    //     .context("Deployment exists but state json does not")?;
    //
    // // network components saved as their full name with project prepended
    // let name = format!("{}-{}", project, component);
    //
    // match state_json.network {
    //     StateNetwork::Ovn(ovn) => {
    //         let lsp = ovn.dhcp_options
    //             .get(&component)
    //             .ok_or(anyhow!("Logical Switch Port {} does not exist", component))?
    //             .check_exists(project).await?;
    //         Ok(get_status_response(lsp))
    //     }
    //     StateNetwork::Ovs(_) => {
    //         Ok((StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"status": "OVS network type not supported for logical switch port"}))).into_response())
    //     },
    // }
}

pub async fn ovs_port_state(
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

    // network components saved as their full name with project prepended
    let name = format!("{}-{}", project, component);

    match state_json.network {
        StateNetwork::Ovn(ovn) => {
            let lsp = ovn.ovs_ports
                .get(&name)
                .ok_or(anyhow!("OVS Port {} does not exist", name))?
                .check_exists(project).await?;
            Ok(get_status_response(lsp))
        }
        StateNetwork::Ovs(_) => {
            Ok((StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"status": "OVS network type not supported for ovs port"}))).into_response())
        },
    }
}
