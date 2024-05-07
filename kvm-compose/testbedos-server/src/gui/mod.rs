mod handlers;
pub mod websocket;
mod gui;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use anyhow::bail;
use axum::Router;
use axum::routing::{get, post};
use tokio::fs::create_dir_all;
use tower_http::services::ServeDir;
use crate::AppState;
use crate::deployments::deployments::{get_home_folder_user_group, set_file_folder_permission};
use crate::gui::handlers::*;


pub fn add_gui_handlers() -> Router<Arc<AppState>> {
    let router = Router::new()
        .route("/gui", get(gui_home))
        .route("/gui/deployments", get(gui_deployments_list))
        .route("/gui/deployments/create",
               get(gui_deployments_create_view).post(gui_deployments_create))
        .route("/gui/deployments/:project", get(gui_deployments_view))
        .route("/gui/deployments/:project/delete", post(gui_deployment_delete))
        .route("/gui/deployments/:project/yaml", post(gui_update_yaml))
        .route("/gui/configuration", get(gui_configuration))
        .route("/gui/configuration/setup", get(gui_configuration_setup))
        .route("/gui/documentation", get(gui_documentation))
        .route("/gui/resource-monitoring", get(resource_monitoring_list));

    #[cfg(debug_assertions)]
    let router = router
        .nest_service("/assets/scripts", ServeDir::new("assets/scripts"));
    #[cfg(not(debug_assertions))]
    let router = router
        .nest_service("/assets/scripts", ServeDir::new("/var/lib/testbedos/assets/scripts"));

    #[cfg(debug_assertions)]
        let router = router
        .nest_service("/assets/icons", ServeDir::new("assets/icons"));
    #[cfg(not(debug_assertions))]
        let router = router
        .nest_service("/assets/icons", ServeDir::new("/var/lib/testbedos/assets/icons"));

    #[cfg(debug_assertions)]
        let router = router
        .nest_service("/assets/documentation", ServeDir::new("assets/documentation"));
    #[cfg(not(debug_assertions))]
        let router = router
        .nest_service("/assets/documentation", ServeDir::new("/var/lib/testbedos/assets/documentation"));

    router
}

/// Get the testbed folder that is in the testbed user's folder. This folder is specifically used
/// for the testbed when using the testbed with the GUI. This also makes sure that this folder will
/// exist when being used by callers. Fails if the folder cannot be made.
async fn testbed_user_folder(db_config: &Arc<AppState>) -> anyhow::Result<PathBuf> {
    let username = &db_config.config_db
        .read()
        .await
        .get_host_config()
        .await?
        .user;
    // first check if the home folder exists, just in case to prevent creating whole empty folder
    // structures in the home directory
    if !Path::new(&format!("/home/{username}/")).exists() {
        bail!("home folder for the testbed user doesn't exist, is this a real user of the system?");
    }
    // TODO - this username is coming from the host.json, do we need to consider if this username
    //  has been input maliciously to mess with this folder creation?
    let path = format!("/home/{username}/.local/share/testbedos/");
    let folder = PathBuf::from(&path);
    if !folder.exists() {
        // create the folder and any parent folders that don't exist
        create_dir_all(folder.clone()).await?;
    }
    // set folder to user permissions
    let (uid, gid) = get_home_folder_user_group(username.clone()).await?;
    set_file_folder_permission(path.into(), uid, gid).await?;
    Ok(folder)
}
