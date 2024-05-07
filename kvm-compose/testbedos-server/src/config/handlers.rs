use std::sync::Arc;
use crate::{AppError, AppState};
use axum::{response::IntoResponse, Json};
use axum::extract::{Query, State};
use axum_extra::response::ErasedJson;
use kvm_compose_schemas::handlers::PrettyQueryParams;
use kvm_compose_schemas::settings::{SshConfig, TestbedClusterConfig};
use crate::config::setup::{get_qemu_conf_user_and_group, get_resource_monitoring_state};

pub async fn get_testbed_cluster_config(
    State(db_config): State<Arc<AppState>>,
    Query(params): Query<PrettyQueryParams>,
) -> Result<impl IntoResponse, AppError> {
    // this is the dynamically managed cluster config
    // try to read the config in the expected location
    let tb_config = db_config.config_db
        .read()
        .await
        .get_cluster_config()
        .await?;

    // format json with pretty formatting if query parameter present and true
    if let Some(pretty) = params.pretty {
        if pretty {
            return Ok(ErasedJson::pretty(tb_config));
        }
    }
    Ok(ErasedJson::new(tb_config))
}

pub async fn set_testbed_cluster_config(
    State(db_config): State<Arc<AppState>>,
    Json(config): Json<TestbedClusterConfig>,
) -> Result<impl IntoResponse, AppError> {
    // create testbed config from json input, validated by serde
    // TODO - should this be editable externally? this should really only be managed by the server
    //  on testbed join/leave requests, otherwise undefined behaviour could happen
    db_config.config_db
        .write()
        .await
        .set_cluster_config(config)
        .await?;
    Ok(())
}

pub async fn get_testbed_host_config(
    State(db_config): State<Arc<AppState>>,
    Query(params): Query<PrettyQueryParams>,
) -> Result<impl IntoResponse, AppError> {
    // return the host.json config
    let host_config = db_config.config_db
        .read()
        .await
        .get_host_config()
        .await?;

    // format json with pretty formatting if query parameter present and true
    if let Some(pretty) = params.pretty {
        if pretty {
            return Ok(ErasedJson::pretty(host_config));
        }
    }
    Ok(ErasedJson::new(host_config))
}

pub async fn set_testbed_host_config(
    State(db_config): State<Arc<AppState>>,
    Json(config): Json<SshConfig>,
) -> Result<impl IntoResponse, AppError> {
    // set the host.json config
    // TODO - validation? this endpoint already attempts the deserialisation implicitly
    // TODO - trigger the cluster config to be re-generated? would this affect live deployments?
    //  maybe stop any updates if there are running deployments
    db_config.config_db
        .read()
        .await
        .set_host_config(config)
        .await?;
    Ok(())
}

pub async fn host_status(

) -> Result<impl IntoResponse, AppError> {
    // for now just return 200 to show this host is running
    // TODO - some useful data?
    Ok(())
}

/// Get the default host.json
pub async fn get_default_host_json(

) -> Result<impl IntoResponse, AppError> {
    let json = serde_json::to_string_pretty(&SshConfig::default())?;
    Ok(json)
}

/// Return a JSON response with the current qemu user and group for libvirt
pub async fn get_qemu_conf_user_group() -> Result<impl IntoResponse, AppError> {
    let (user, group) = get_qemu_conf_user_and_group().await?;

    let mut json = serde_json::Map::new();
    match user {
        None => {
            json.insert("user".to_string(), serde_json::Value::String("".to_string()));
        }
        Some(user) => {
            json.insert("user".to_string(), serde_json::Value::String(user));
        }
    }
    match group {
        None => {
            json.insert("group".to_string(), serde_json::Value::String("".to_string()));
        }
        Some(group) => {
            json.insert("group".to_string(), serde_json::Value::String(group));
        }
    }

    Ok(serde_json::to_string_pretty(&json)?)
}

/// Interrogate the docker daemon to see if the resource monitoring containers are running.
pub async fn get_metrics_state(
    // State(db_config): State<Arc<AppState>>,
) -> Result<impl IntoResponse, AppError> {
    let res = get_resource_monitoring_state().await?;
    Ok(serde_json::to_string_pretty(&res)?)
}