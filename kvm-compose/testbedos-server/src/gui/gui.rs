use std::sync::Arc;
use anyhow::bail;
use tokio::fs::create_dir;
use kvm_compose_lib::components::helpers::serialisation::write_file_with_permissions;
use kvm_compose_schemas::gui_models::GUICreateDeploymentJson;
use crate::AppState;
use crate::deployments::deployments::{get_home_folder_user_group, set_file_folder_permission, validate_yaml};

/// This function does the filesystem work when a deployment is going to be created via the GUI.
/// If at any point this function fails, we need to roll back the changes on the filesystem, so
/// that there is no mess left and when the user fixes their input the consecutive attempt doesn't
/// fail due to leftover files/folders.
pub async fn create_deployment_files(
    db_config: &Arc<AppState>,
    testbed_user_folder: String,
    config: &GUICreateDeploymentJson,
) -> anyhow::Result<String> {
    // try to create folder in specified location
    create_dir(testbed_user_folder.clone()).await?;

    // get testbed user's id and group
    let testbed_user = &db_config.config_db
        .read()
        .await
        .get_host_config()
        .await?
        .user;
    let (uid, gid) = get_home_folder_user_group(testbed_user.clone()).await?;

    // validate yaml
    let valid_yaml = validate_yaml(config.yaml.clone());
    if !valid_yaml.0.is_success() {
        bail!("Yaml was not valid");
    }

    // add yaml to folder
    write_file_with_permissions(
        format!("{}/kvm-compose.yaml", testbed_user_folder.clone()),
        config.yaml.clone(),
        0o755,
        uid,
        gid,
    ).await?;
    // TODO - set deployment folder permission
    set_file_folder_permission(testbed_user_folder.clone().into(), uid, gid).await?;
    Ok("Added deployment files to folder".to_string())
}