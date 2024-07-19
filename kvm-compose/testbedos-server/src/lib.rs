use std::sync::Arc;
use tokio::sync::RwLock;
use axum::response::{IntoResponse, Response};
use axum::http::StatusCode;
use sysinfo::System;
use tera::Tera;
use service_clients::docker::DockerUnixClient;
use crate::config::provider::TestbedConfigProvider;
use crate::deployments::providers::DeploymentProvider;

pub mod config;
pub mod deployments;
pub mod cluster;
pub mod logging;
pub mod resource_monitoring;
pub mod orchestration;
pub mod gui;

/// Store a version of the testbed server when compiled - useful for versioning javascript
pub const PROJECT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Hold a connection to the services used by the testbed. This needs to be thread safe as the
/// connection will be shared.
pub struct ServiceClients {
    pub docker_conn: RwLock<DockerUnixClient>,
}

impl ServiceClients {
    pub async fn new(

    ) -> Self {
        Self {
            docker_conn: RwLock::new(DockerUnixClient::new("/var/run/docker.sock")
                .await
                .expect("could not connect to docker client")),
        }
    }
}

/// Store some state for the handlers.
#[derive(Clone)]
pub struct AppState {
    pub deployment_config_db: Arc<RwLock<Box<dyn DeploymentProvider + Sync + Send>>>,
    pub config_db: Arc<RwLock<Box<dyn TestbedConfigProvider + Sync + Send>>>,
    pub server_url: String,
    pub system_monitor: Arc<RwLock<System>>,
    pub template_env: Arc<RwLock<Tera>>,
    pub service_clients: Arc<ServiceClients>,
}

/// Store some state for the handlers in client mode.
#[derive(Clone)]
pub struct ClientAppState {
    pub config_db: Arc<RwLock<Box<dyn TestbedConfigProvider + Sync + Send>>>,
    pub main_server_url: String,
    pub system_monitor: Arc<RwLock<System>>,
    pub service_clients: Arc<ServiceClients>,
}

// TODO - use proper errors for the different http error codes where appropriate
/// This is a generic error handling struct to be used by the handlers.
pub struct AppError(anyhow::Error);

impl IntoResponse for AppError {
    // TODO - return a JSON style error response with code
    // TODO - throw actual status codes rather than 500
    fn into_response(self) -> Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Something went wrong: {}", self.0),
        )
            .into_response()
    }
}

impl<E> From<E> for AppError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        // TODO - match different anyhow errors to different Status codes?
        //  or in the code make sure we return anyhow::Result<StatusCode>?
        //  or in the handlers or code make sure to not use ? when we know the status code to return
        //   i.e. Ok(StatusCode::NOT_FOUND)
        Self(err.into())
    }
}

pub fn ip_string_to_slice(ip: &String) -> anyhow::Result<[u8; 4]> {
    let split: Vec<_> = ip.split(".").collect();
    Ok([split[0].parse::<u8>()?,split[1].parse::<u8>()?,split[2].parse::<u8>()?,split[3].parse::<u8>()?])
}


pub async fn debug_reload_templates(
    db_config: &Arc<AppState>,
) -> anyhow::Result<()> {
    if cfg!(debug_assertions) {
        db_config.template_env.write().await.full_reload()?;
    }
    Ok(())
}
