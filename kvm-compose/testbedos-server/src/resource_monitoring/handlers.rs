use std::sync::Arc;
use axum::extract::{Path, State};
use axum::{Json};
use axum::response::{Html, IntoResponse};
use crate::{AppError, AppState, ClientAppState, debug_reload_templates};
use crate::resource_monitoring::collector::{collect_from_guest, collect_from_host, collect_metrics_for_guests, collect_metrics_for_hosts, get_active_guests_for_deployment, get_active_hosts_for_deployment};

// these handlers are for prometheus to scrape

/// This endpoint is used by prometheus to scrape resource monitoring data for the active testbed
/// hosts
pub async fn prometheus_scrape_endpoint_for_hosts(
    State(db_config): State<Arc<AppState>>,
) -> Result<impl IntoResponse, AppError> {
    let cluster_config = db_config.config_db.read()
        .await
        .get_cluster_config()
        .await?;
    let metrics = collect_metrics_for_hosts(&cluster_config).await?;
    Ok(metrics)
}

/// This endpoint is used by prometheus to scrape resource monitoring data for the active testbed
/// libvirt guests
pub async fn prometheus_scrape_endpoint_for_libvirt(
    State(db_config): State<Arc<AppState>>,
) -> Result<impl IntoResponse, AppError> {
    let cluster_config = db_config.config_db.read()
        .await
        .get_cluster_config()
        .await?;
    let deployments = db_config.deployment_config_db.read()
        .await
        .list_deployments()
        .await?;
    let metrics = collect_metrics_for_guests(&cluster_config, &deployments, "libvirt").await?;
    Ok(metrics)
}

/// This endpoint is used by prometheus to scrape resource monitoring data for the active testbed
/// docker guests
pub async fn prometheus_scrape_endpoint_for_docker(
    State(db_config): State<Arc<AppState>>,
) -> Result<impl IntoResponse, AppError> {
    let cluster_config = db_config.config_db.read()
        .await
        .get_cluster_config()
        .await?;
    let deployments = db_config.deployment_config_db.read()
        .await
        .list_deployments()
        .await?;
    let metrics = collect_metrics_for_guests(&cluster_config, &deployments, "docker").await?;
    Ok(metrics)
}

/// This endpoint is used by prometheus to scrape resource monitoring data for the active testbed
/// android guests
pub async fn prometheus_scrape_endpoint_for_android(
    State(db_config): State<Arc<AppState>>,
) -> Result<impl IntoResponse, AppError> {
    let cluster_config = db_config.config_db.read()
        .await
        .get_cluster_config()
        .await?;
    let deployments = db_config.deployment_config_db.read()
        .await
        .list_deployments()
        .await?;
    let metrics = collect_metrics_for_guests(&cluster_config, &deployments, "android").await?;
    Ok(metrics)
}

// these handlers are the for master testbed to poll each testbed for raw metrics

/// This endpoint returns the master testbed host resource data
pub async fn get_master_testbed_host_resource(
    State(db_config): State<Arc<AppState>>,
) -> Result<impl IntoResponse, AppError> {
    let host_name = db_config.config_db.read().await
        .get_host_config()
        .await?
        .ovn
        .chassis_name;
    let system = db_config.system_monitor.write().await;
    let host_data = collect_from_host(host_name, system).await?;
    Ok(Json(host_data))
}

/// This endpoint returns the master testbed host resource data
pub async fn get_client_testbed_host_resource(
    State(db_config): State<Arc<ClientAppState>>,
) -> Result<impl IntoResponse, AppError> {
    let host_name = db_config.config_db.read().await
        .get_host_config()
        .await?
        .ovn
        .chassis_name;
    let system = db_config.system_monitor.write().await;
    let host_data = collect_from_host(host_name, system).await?;
    Ok(Json(host_data))
}

/// This endpoint returns the specified guest resource data on the master testbed
pub async fn get_master_testbed_guest_resource(
    State(db_config): State<Arc<AppState>>,
    Path((project, name)): Path<(String, String)>,
) -> Result<impl IntoResponse, AppError> {
    let guest_data = collect_from_guest(
        name,
        project,
        &"localhost".to_string(),
        db_config.service_clients.clone(),
    ).await?;
    Ok(Json(guest_data))
}

/// This endpoint returns the specified guest resource data on the client testbed
pub async fn get_client_testbed_guest_resource(
    State(db_config): State<Arc<ClientAppState>>,
    Path((project, name)): Path<(String, String)>,
) -> Result<impl IntoResponse, AppError> {
    let guest_data = collect_from_guest(
        name,
        project,
        &db_config.master_server_url,
        db_config.service_clients.clone(),
    ).await?;
    Ok(Json(guest_data))
}

// these endpoints are for visualisation of the metrics from the server

/// This endpoint provides the HTML page for the resource monitoring dashboard
pub async fn resource_monitoring_dashboard(
    State(db_config): State<Arc<AppState>>,
    Path(project): Path<String>,
) -> Result<impl IntoResponse, AppError> {

    // reload if running in debug mode to allow quick changing of templates during development
    debug_reload_templates(&db_config).await?;

    let tera = db_config.template_env.read().await;
    let mut tera_context = tera::Context::new();

    let deployment = db_config.deployment_config_db
        .read()
        .await
        .get_deployment(project.to_string())
        .await?;

    // fill template
    tera_context.insert("project_name", &project);
    // do hosts
    let guest_list = get_active_guests_for_deployment(&deployment).await?;
    tera_context.insert("guest_list", &guest_list);
    let host_list = get_active_hosts_for_deployment(&deployment).await?;
    tera_context.insert("host_list", &host_list);

    let render = tera.render("dashboard.html", &tera_context)?;

    Ok(Html(render))
}
