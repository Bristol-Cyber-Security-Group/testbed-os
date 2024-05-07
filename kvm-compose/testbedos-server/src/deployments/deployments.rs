use std::os::linux::fs::MetadataExt;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::sync::Arc;
use http::StatusCode;
use serde::{Deserialize, Serialize};
use anyhow::Context;
use nix::unistd::{Gid, Uid};
use kvm_compose_schemas::kvm_compose_yaml::Config;
use crate::AppState;

#[derive(Serialize, Deserialize)]
pub struct ProjectAndPath {
    pub project_name: String,
}

pub async fn validate_project_name(
    db_config: &Arc<AppState>,
    project_name: String,
) -> (StatusCode, String) {
    // check if the project name and path are valid

    if project_name.eq("") {
        return (StatusCode::BAD_REQUEST, "Project name was empty".to_string());
    }

    // if true, then project already exists
    let project_exists = db_config.deployment_config_db
        .read()
        .await
        .get_deployment(project_name.clone())
        .await
        .is_ok();

    let project_name_result = if project_exists {
        "Project name taken".to_string()
    } else {
        "Project name OK".to_string()
    };

    if project_exists {
        (StatusCode::BAD_REQUEST, project_name_result)
    } else {
        (StatusCode::OK, project_name_result)
    }
}

pub fn validate_yaml(
    yaml: String,
) -> (StatusCode, String) {
    match string_to_yaml(yaml) {
        Ok(_) => (StatusCode::OK, "".to_string()),
        Err(err) => (StatusCode::BAD_REQUEST, format!("{err:#}")),
    }
}

pub fn string_to_yaml(body: String) -> anyhow::Result<Config> {
    let value: Config = serde_yaml::from_str(&body).with_context(|| "Parsing Config YAML")?;
    Ok(value)
}

pub async fn get_home_folder_user_group(username: String) -> anyhow::Result<(Uid, Gid)> {
    let metadata = tokio::fs::metadata(format!("/home/{username}")).await?;
    let uid = metadata.st_uid();
    let gid = metadata.st_gid();
    let uid = Uid::from(uid);
    let gid = Gid::from(gid);
    Ok((uid, gid))
}

pub async fn set_file_folder_permission(
    path: PathBuf,
    uid: Uid,
    gid: Gid,
) -> anyhow::Result<()> {
    let mut perms = tokio::fs::metadata(&path).await?.permissions();
    perms.set_mode(0o755);
    tokio::fs::set_permissions(&path, perms).await?;
    nix::unistd::chown(&path, Some(uid), Some(gid))?;
    Ok(())
}
