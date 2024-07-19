use std::collections::HashMap;
use crate::deployments::models::*;
use crate::{AppError, AppState};
use axum::extract::{Path, Query, State};
use axum::{Json, response::IntoResponse};
use std::sync::Arc;
use axum_extra::response::ErasedJson;
use tokio::fs::File;
use tokio::io::AsyncReadExt;
use kvm_compose_schemas::handlers::PrettyQueryParams;
use crate::deployments::deployments::{ProjectAndPath, validate_project_name, validate_yaml};
use kvm_compose_lib::state::State as KvmComposeState;

/// List all deployments the database contains.
/// Requires a read lock on the database.
pub async fn list_deployments(
    State(db_config): State<Arc<AppState>>,
) -> Result<impl IntoResponse, AppError> {
    let db = &db_config.deployment_config_db;
    let list = db.read().await.list_deployments().await?;
    Ok(Json(list))
}

/// List all active deployments the database contains.
pub async fn list_active_deployments(
    State(db_config): State<Arc<AppState>>,
) -> Result<impl IntoResponse, AppError> {
    // TODO - what is a certain way to test if a deployment is up? is DeploymentState reliable?
    //  for now use ::Up
    let db = &db_config.deployment_config_db;
    let list = db.read().await.list_deployments().await?;
    let mut active_deployments = HashMap::new();
    for (name, deployment) in list.deployments {
        match deployment.state {
            DeploymentState::Up => {
                active_deployments.insert(name, deployment);
            }
            _ => {}
        }
    }
    Ok(Json(active_deployments))
}

/// Get a specific deployment in the database.
/// Requires a read lock on the database.
pub async fn get_deployment(
    State(db_config): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let db = &db_config.deployment_config_db;
    let deployment = db.read().await.get_deployment(name).await?;
    Ok(Json(deployment))
}

/// Create a deployment in the database.
/// Requires a write lock on the database.
pub async fn create_deployment(
    State(db_config): State<Arc<AppState>>,
    Json(deployment): Json<NewDeployment>,
) -> Result<impl IntoResponse, AppError> {
    let db = &db_config.deployment_config_db;
    db.write().await.create_deployment(deployment).await?;
    Ok(())
}

/// Delete a deployment in the database.
/// Requires a write lock on the database.
pub async fn delete_deployment(
    State(db_config): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let db = &db_config.deployment_config_db;
    db.write().await.delete_deployment(name).await?;
    Ok(())
}

/// Update an existing deployment in the database.
/// Requires a write lock on the database.
pub async fn update_deployment(
    State(db_config): State<Arc<AppState>>,
    Path(name): Path<String>,
    Json(deployment): Json<Deployment>,
) -> Result<impl IntoResponse, AppError> {
    // TODO - accept partially updated deployment data?
    let db = &db_config.deployment_config_db;
    db.write().await.update_deployment(name, deployment).await?;
    Ok(())
}

/// Get the state for the deployment, if it exists and artefacts has been generated
/// Requires a read lock on the database.
pub async fn get_state(
    State(db_config): State<Arc<AppState>>,
    Path(name): Path<String>,
    Query(params): Query<PrettyQueryParams>,
) -> Result<ErasedJson, AppError> {
    let state_json = db_config.deployment_config_db
        .read()
        .await
        .get_state(name)
        .await?;

    // format json with pretty formatting if query parameter present and true
    if let Some(pretty) = params.pretty {
        if pretty {
            return Ok(ErasedJson::pretty(state_json));
        }
    }
    Ok(ErasedJson::new(state_json))
}

/// Set the state for a specific deployment, in its project folder
pub async fn set_state(
    State(db_config): State<Arc<AppState>>,
    Path(name): Path<String>,
    Json(state): Json<KvmComposeState>, // this State has been aliased, see imports
) -> Result<impl IntoResponse, AppError> {
    db_config.deployment_config_db
        .write()
        .await
        .set_state(name, state)
        .await?;
    Ok(())
}

pub async fn get_deployment_yaml(
    State(db_config): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    // check project exists
    let db = &db_config.deployment_config_db;
    let deployment = db.read().await.get_deployment(name).await?;
    // get project location and yaml
    let project_location = deployment.project_location;
    let yaml_location = format!("{project_location}/kvm-compose.yaml");
    let mut yaml = File::open(yaml_location).await?;
    let mut yaml_string = String::new();
    yaml.read_to_string(&mut yaml_string).await?;
    // return yaml
    Ok(yaml_string)
}

pub async fn validate_yaml_endpoint(
    body: String,
) -> Result<impl IntoResponse, AppError> {
    // check if the given yaml over a POST request is valid in the testbed schema

    let result = validate_yaml(body);

    Ok(result)
}

pub async fn validate_project_name_handler(
    State(db_config): State<Arc<AppState>>,
    Json(body): Json<ProjectAndPath>,
) -> Result<impl IntoResponse, AppError> {
    Ok(validate_project_name(&db_config, body.project_name).await)
}
