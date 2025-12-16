use std::fmt::Display;
use std::path::Path;
use std::sync::Arc;
use sysinfo::System;
use tera::Tera;
use tempfile::tempdir;
use tokio::sync::RwLock;
use kvm_compose_schemas::deployment_models::{DeploymentState, NewDeployment};
use testbedos_lib::{AppState, ServiceClients};
use testbedos_lib::config::provider::{TestbedConfigDatabaseProvider, TestbedConfigProvider};
use testbedos_lib::deployments::providers::{DeploymentDatabaseProvider, DeploymentProvider, FileBasedProvider};

/// setup the harness for this test, we will use a temporary directory for each test
async fn setup_file_based<D: Display>(
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

async fn create_dummy_project(
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

#[tokio::test]
async fn test_create_then_read_deployment_filesystem_race_condition() -> anyhow::Result<()> {

    // occasionally, we get an:
    // Error: EOF while parsing a value at line 1 column 0
    // which comes from reading the deployment after creating it, or similar quick succession
    // operations because we did not have a flush/sync to filesystem ... now that exists, we keep
    // this test to show it hasn't come back
    for _ in 0..100 {
        let deployment_db_dir = tempdir()?;
        let config_db = setup_file_based(deployment_db_dir.path().to_string_lossy()).await;

        let test_project_dir = tempdir()?;
        create_dummy_project(&config_db, test_project_dir.path(), "test").await?;

        let db = &config_db.deployment_config_db;
        db.read().await.get_deployment("test".to_string()).await?;

    }

    Ok(())
}

#[tokio::test]
async fn test_create_get_update_get_delete_get() -> anyhow::Result<()> {
    // this test makes sure we don't have any race conditions on the filesystem like the test
    // test_create_then_read_deployment_filesystem_race_condition
    // but this time around it tests a full set of operations in the lifecycle of a deployment

    for _ in 0..100 {
        let deployment_db_dir = tempdir()?;
        let config_db = setup_file_based(deployment_db_dir.path().to_string_lossy()).await;

        let test_project_dir = tempdir()?;
        create_dummy_project(&config_db, test_project_dir.path(), "test").await?;

        let db = &config_db.deployment_config_db;
        let mut dep = db.read().await.get_deployment("test".to_string()).await?;
        // deployments always start in down state
        assert_eq!(dep.state, DeploymentState::Down);

        dep.state = DeploymentState::Running;
        db.write().await.update_deployment("test".to_string(), dep).await?;

        // make sure the update worked
        let mut dep = db.read().await.get_deployment("test".to_string()).await?;
        assert_eq!(dep.state, DeploymentState::Running);

        // try to delete, wont work since running state should not allow deletion
        let cant_delete = db.write().await.delete_deployment("test".to_string()).await;
        assert!(cant_delete.is_err());

        // set to down so we can delete
        dep.state = DeploymentState::Down;
        db.write().await.update_deployment("test".to_string(), dep).await?;
        db.write().await.delete_deployment("test".to_string()).await?;

        // check its gone
        let should_fail = db.read().await.get_deployment("test".to_string()).await;
        assert!(should_fail.is_err());

    }

    Ok(())
}

#[tokio::test]
async fn test_list_deployments() -> anyhow::Result<()> {
    let deployment_db_dir = tempdir()?;
    let config_db = setup_file_based(deployment_db_dir.path().to_string_lossy()).await;
    let db = &config_db.deployment_config_db;

    let test_project_dir_1 = tempdir()?;
    create_dummy_project(&config_db, test_project_dir_1.path(), "test1").await?;

    let test_project_dir_2 = tempdir()?;
    create_dummy_project(&config_db, test_project_dir_2.path(), "test2").await?;

    let list_deployments = db.read().await.list_deployments().await?;
    assert_eq!(list_deployments.deployments.len(), 2);

    let test_project_dir_3 = tempdir()?;
    create_dummy_project(&config_db, test_project_dir_3.path(), "test3").await?;

    let list_deployments = db.read().await.list_deployments().await?;
    assert_eq!(list_deployments.deployments.len(), 3);

    db.write().await.delete_deployment("test2".to_string()).await?;

    let list_deployments = db.read().await.list_deployments().await?;
    assert_eq!(list_deployments.deployments.len(), 2);

    assert!(list_deployments.deployments.get("test1").is_some());
    assert!(list_deployments.deployments.get("test2").is_none());
    assert!(list_deployments.deployments.get("test3").is_some());

    Ok(())
}
