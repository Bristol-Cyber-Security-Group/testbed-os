use crate::state_evaluation::models::*;
use crate::{AppError, AppState};
use axum::extract::{Path, State};
use axum::response::Response;
use axum::{
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use kvm_compose_lib::state::state_evaluation::StateComponentStatus;
use serde_json::json;
use std::sync::Arc;

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

pub async fn deployment_state(
    State(db_config): State<Arc<AppState>>,
    Path(project): Path<String>,
) -> Result<impl IntoResponse, AppError> {

    let state = total_deployment_state(db_config, project).await?;
    Ok((StatusCode::OK, state).into_response())

}

pub async fn guest_state_handler(
    State(db_config): State<Arc<AppState>>,
    Path((project, component)): Path<(String, String)>,
) -> Result<impl IntoResponse, AppError> {

    let guest_exists = guest_state(db_config, project, component).await?;
    Ok(get_status_response(guest_exists))

}


pub async fn logical_switch_state_handler(
    State(db_config): State<Arc<AppState>>,
    Path((project, component)): Path<(String, String)>,
) -> Result<impl IntoResponse, AppError> {

    let component_exists = logical_switch_state(db_config, project, component).await?;
    Ok(get_status_response(component_exists))

}

pub async fn logical_switch_port_state_handler(
    State(db_config): State<Arc<AppState>>,
    Path((project, component)): Path<(String, String)>,
) -> Result<impl IntoResponse, AppError> {

    let component_exists = logical_switch_port_state(db_config, project, component).await?;
    Ok(get_status_response(component_exists))

}

pub async fn logical_router_state_handler(
    State(db_config): State<Arc<AppState>>,
    Path((project, component)): Path<(String, String)>,
) -> Result<impl IntoResponse, AppError> {

    let component_exists = logical_router_state(db_config, project, component).await?;
    Ok(get_status_response(component_exists))

}

pub async fn logical_router_port_state_handler(
    State(db_config): State<Arc<AppState>>,
    Path((project, component)): Path<(String, String)>,
) -> Result<impl IntoResponse, AppError> {

    let component_exists = logical_router_port_state(db_config, project, component).await?;
    Ok(get_status_response(component_exists))

}

pub async fn logical_acl_state_handler(
    State(db_config): State<Arc<AppState>>,
    Path((project, component)): Path<(String, String)>,
) -> Result<impl IntoResponse, AppError> {

    let component_exists = logical_acl_state(db_config, project, component).await?;
    Ok(get_status_response(component_exists))

}

pub async fn logical_dhcp_state_handler(
    State(db_config): State<Arc<AppState>>,
    Path((project, component)): Path<(String, String)>,
) -> Result<impl IntoResponse, AppError> {

    let component_exists = logical_dhcp_state(db_config, project, component).await?;
    Ok(get_status_response(component_exists))

}

pub async fn ovs_port_state_handler(
    State(db_config): State<Arc<AppState>>,
    Path((project, component)): Path<(String, String)>,
) -> Result<impl IntoResponse, AppError> {

    let component_exists = ovs_port_state(db_config, project, component).await?;
    Ok(get_status_response(component_exists))

}
