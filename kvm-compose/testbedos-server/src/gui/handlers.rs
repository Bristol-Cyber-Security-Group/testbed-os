use std::sync::Arc;
use anyhow::{anyhow};
use axum::extract::{Path, State};
use axum::Json;
use axum::response::{Html, IntoResponse};
use http::StatusCode;
use tera::Context;
use tokio::fs;
use kvm_compose_schemas::deployment_models::NewDeployment;
use kvm_compose_schemas::gui_models::GUICreateDeploymentJson;
use kvm_compose_schemas::kvm_compose_yaml::Config;
use crate::{AppError, AppState, debug_reload_templates, PROJECT_VERSION};
use crate::deployments::deployments::{validate_project_name};
use crate::gui::gui::create_deployment_files;
use crate::gui::testbed_user_folder;
use crate::resource_monitoring::collector::{get_active_guests_for_deployment, get_active_hosts_for_deployment};

/// GUI index page
pub async fn gui_home(
    State(db_config): State<Arc<AppState>>,
) -> Result<impl IntoResponse, AppError> {
    // reload if running in debug mode to allow quick changing of templates during development
    debug_reload_templates(&db_config).await?;

    let tera = db_config.template_env.read().await;
    let mut tera_context = Context::new();

    add_base_tera_context(&mut tera_context, "Home", &db_config).await;

    let render = tera.render("gui/index.html", &tera_context)?;

    Ok(Html(render))
}

/// GUI deployments page
pub async fn gui_deployments_list(
    State(db_config): State<Arc<AppState>>,
) -> Result<impl IntoResponse, AppError> {
    // re-use the deployment page, but add HTML specific to list

    // reload if running in debug mode to allow quick changing of templates during development
    debug_reload_templates(&db_config).await?;

    let tera = db_config.template_env.read().await;
    let mut tera_context = Context::new();

    add_base_tera_context(&mut tera_context, "List Deployments", &db_config).await;

    // get all deployments to display on page
    let deployments = db_config.deployment_config_db
        .read()
        .await
        .list_deployments()
        .await?
        .deployments;

    tera_context.insert("deployments", &deployments);

    let render = tera.render("gui/deployments.html", &tera_context)?;

    Ok(Html(render))
}

pub async fn gui_deployments_create_view(
    State(db_config): State<Arc<AppState>>,
) -> Result<impl IntoResponse, AppError> {
    // re-use the deployment page, but add HTML specific to create

    // reload if running in debug mode to allow quick changing of templates during development
    debug_reload_templates(&db_config).await?;

    let tera = db_config.template_env.read().await;
    let mut tera_context = Context::new();

    add_base_tera_context(&mut tera_context, "Create Deployment", &db_config).await;

    let tesbed_user = &db_config.config_db
        .read()
        .await
        .get_host_config()
        .await?
        .user;
    tera_context.insert("testbed_user", tesbed_user);

    let render = tera.render("gui/deployment_create.html", &tera_context)?;

    Ok(Html(render))
}

pub async fn gui_deployments_create(
    State(db_config): State<Arc<AppState>>,
    Json(config): Json<GUICreateDeploymentJson>,
) -> Result<impl IntoResponse, AppError> {
    // this is not a page, but the endpoint that will create the folder and also call the
    // deployment create command

    // check if project name already exists, look at the returned status code if not OK then fail
    let validation_status = validate_project_name(&db_config, config.project_name.clone()).await;
    if !validation_status.0.is_success() {
        return Ok(validation_status);
    }

    // make sure that the ~/.local/share/testbedos/deployments folder exists
    let mut testbed_user_folder = testbed_user_folder(&db_config).await?;
    // then see if there is not already a project folder with the same name
    testbed_user_folder.push(config.project_name.clone());
    if testbed_user_folder.exists() {
        return Ok((StatusCode::BAD_REQUEST, "A folder with this deployment name already exists in the testbed user home dir ~/.local/share/testbedos/".to_string()));
    }
    let testbed_user_folder_string = testbed_user_folder.clone().to_string_lossy().to_string();

    //
    let result = create_deployment_files(
        &db_config,
        testbed_user_folder_string.clone(),
        &config,
    ).await;
    match result {
        Ok(_) => {}
        Err(err) => {
            // undo creating folder and adding files, ignore errors
            let _ = fs::remove_dir_all(testbed_user_folder_string).await;
            return Ok((StatusCode::BAD_REQUEST, err.to_string()))
        }
    }

    // create deployment model
    let new_deployment = NewDeployment {
        name: config.project_name.clone(),
        project_location: testbed_user_folder_string,
    };

    // call create deployment to finalise the creation in that folder
    db_config.deployment_config_db
        .write()
        .await
        .create_deployment(new_deployment)
        .await?;

    Ok((StatusCode::OK, "Deployment Created".to_string()))
}

pub async fn gui_deployments_view(
    State(db_config): State<Arc<AppState>>,
    Path(project): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    // re-use the deployment page, but add HTML specific to create

    // reload if running in debug mode to allow quick changing of templates during development
    debug_reload_templates(&db_config).await?;

    let tera = db_config.template_env.read().await;
    let mut tera_context = Context::new();

    add_base_tera_context(&mut tera_context, &project.to_string(), &db_config).await;
    tera_context.insert("project_name", &project);

    // get specific deployment
    let deployments = db_config.deployment_config_db
        .read()
        .await
        .list_deployments()
        .await?
        .deployments;
    let deployment_data = deployments
        .get(&project)
        .ok_or(anyhow!("Deployment does not exist"))?;

    // add guest data for resource monitoring grafana dashboards
    let guest_list = get_active_guests_for_deployment(deployment_data).await.unwrap_or_else(|_| Vec::new());
    tera_context.insert("guest_list", &guest_list);
    let host_list = get_active_hosts_for_deployment(deployment_data).await.unwrap_or_else(|_| Vec::new());
    tera_context.insert("host_list", &host_list);

    tera_context.insert("deployment", deployment_data);

    let render = tera.render("gui/deployments.html", &tera_context)?;

    Ok(Html(render))
}

pub async fn gui_update_yaml(
    State(db_config): State<Arc<AppState>>,
    Path(project): Path<String>,
    body: String,
) -> Result<impl IntoResponse, AppError> {

    // we re-validate the yaml just in case
    let yaml: Config = serde_yaml::from_str(&body)?;
    // get yaml location
    let mut project_location = db_config.deployment_config_db
        .read()
        .await
        .get_deployment(project)
        .await?
        .project_location;
    project_location.push_str("/kvm-compose.yaml");
    yaml.save_to(project_location).await?;

    Ok(())
}

pub async fn gui_deployment_delete(
    State(db_config): State<Arc<AppState>>,
    Path(project): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    // delete the deployment and redirect to deployment list page

    tracing::info!("deleting deployment {project}");

    db_config.deployment_config_db
        .write()
        .await
        .delete_deployment(project)
        .await?;

    // Redirect::to("/gui/deployments")
    Ok(())
}

pub async fn resource_monitoring_list(
    State(db_config): State<Arc<AppState>>,
) -> Result<impl IntoResponse, AppError> {
    // list all deployments to get their resource monitoring pages

    // reload if running in debug mode to allow quick changing of templates during development
    debug_reload_templates(&db_config).await?;

    let tera = db_config.template_env.read().await;
    let mut tera_context = Context::new();

    add_base_tera_context(&mut tera_context, "Resource Monitoring", &db_config).await;

    // get all deployments to display on page
    let deployments = db_config.deployment_config_db
        .read()
        .await
        .list_deployments()
        .await?
        .deployments;

    tera_context.insert("deployments", &deployments);

    let render = tera.render("gui/resource_monitoring_list.html", &tera_context)?;

    Ok(Html(render))

}

pub async fn gui_configuration(
    State(db_config): State<Arc<AppState>>,
) -> Result<impl IntoResponse, AppError> {
    // reload if running in debug mode to allow quick changing of templates during development
    debug_reload_templates(&db_config).await?;

    let tera = db_config.template_env.read().await;
    let mut tera_context = Context::new();

    add_base_tera_context(&mut tera_context, "Configuration", &db_config).await;

    let render = tera.render("gui/configuration/configuration.html", &tera_context)?;

    Ok(Html(render))
}

pub async fn gui_configuration_setup(
    State(db_config): State<Arc<AppState>>,
) -> Result<impl IntoResponse, AppError> {
    // reload if running in debug mode to allow quick changing of templates during development
    debug_reload_templates(&db_config).await?;

    let tera = db_config.template_env.read().await;
    let mut tera_context = Context::new();

    add_base_tera_context(&mut tera_context, "Configuration Setup", &db_config).await;

    let render = tera.render("gui/configuration/setup.html", &tera_context)?;

    Ok(Html(render))
}

pub async fn gui_documentation(
    State(db_config): State<Arc<AppState>>,
) -> Result<impl IntoResponse, AppError> {
    // reload if running in debug mode to allow quick changing of templates during development
    debug_reload_templates(&db_config).await?;

    let tera = db_config.template_env.read().await;
    let mut tera_context = Context::new();

    add_base_tera_context(&mut tera_context, "Documentation", &db_config).await;

    let render = tera.render("gui/documentation.html", &tera_context)?;

    Ok(Html(render))
}

/// Helper to add base context variables for the Tera templating
async fn add_base_tera_context(
    tera_context: &mut Context,
    page_name: &str,
    db_config: &Arc<AppState>,
) {
    tera_context.insert("testbed_project_version", PROJECT_VERSION);
    tera_context.insert("page_name", page_name);
    let mut server_url = db_config.server_url.clone();
    server_url = server_url.replace("0.0.0.0", "localhost");
    tera_context.insert("server_url", &server_url);
}
