use std::fmt::Display;
use std::path::Path;
use std::sync::{Arc, Mutex};
use tokio::sync::RwLock;
use tera::Tera;
use sysinfo::System;
use kvm_compose_schemas::deployment_models::NewDeployment;
use testbedos_lib::{AppState, ServiceClients};
use testbedos_lib::config::provider::{TestbedConfigDatabaseProvider, TestbedConfigProvider};
use testbedos_lib::deployments::providers::{DeploymentDatabaseProvider, DeploymentProvider, FileBasedProvider};

/// setup the harness for this test, we will use a temporary directory for each test
pub async fn setup_file_based<D: Display>(
    temp_dir: D,
) -> Arc<AppState> {

    let deployment_db = DeploymentDatabaseProvider::FileDB(
        FileBasedProvider::new(format!("{temp_dir}/deployments/"))
    ).get_provider();
    let config_db = TestbedConfigDatabaseProvider::FileDB.get_provider();

    let deployment_config_db: Arc<RwLock<Box<dyn DeploymentProvider + Sync + Send>>> =
        Arc::new(RwLock::new(deployment_db));
    let config_db: Arc<RwLock<Box<dyn TestbedConfigProvider + Sync + Send>>> =
        Arc::new(RwLock::new(config_db));
    let server_url = "http://0.0.0.0:3355".to_string();
    let app_state = Arc::new(AppState {
        deployment_config_db,
        config_db,
        server_url,
        system_monitor: Arc::new(RwLock::new(System::new_all())),
        template_env: get_tera_env(),
        service_clients: Arc::new(ServiceClients::new().await),
        active_deployments: Arc::new(Mutex::new(Default::default())),
    });

    // create a deployments folder
    tokio::fs::create_dir(Path::new(&format!("{temp_dir}/deployments/"))).await.expect("create temp dir");

    app_state
}

fn get_tera_env() -> Arc<RwLock<Tera>> {
    let env = Arc::new(RwLock::new(Tera::new("assets/templates/**/*.html")
        .expect("could not load Tera templates")));
    env
}

pub async fn create_dummy_project(
    db: &Arc<AppState>,
    path: &Path,
    project_name: &str,
) -> anyhow::Result<()> {
    let temp_project_path = path.to_string_lossy().to_string();

    // place a kvm-compose inside the project before making they deployment
    tokio::fs::write(Path::new(&format!("{temp_project_path}/kvm-compose.yaml")), "").await?;

    // create new deployment
    let db = &db.deployment_config_db;
    db.read().await.create_deployment(NewDeployment {
        name: project_name.to_string(),
        project_location: temp_project_path,
    }).await?;

    Ok(())
}
