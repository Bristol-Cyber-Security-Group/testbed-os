use std::os::linux::fs::MetadataExt;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::sync::Arc;
use http::StatusCode;
use serde::{Deserialize, Serialize};
use serde_yaml::Value;
use anyhow::{Result, Context, Error};
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
    // Validate yaml structure before parsing to Config
    let yaml_value: Value = serde_yaml::from_str(&body).with_context(|| "Parsing raw YAML")?;
    validate_guest_types(&yaml_value)?;

    // Validate Config semantics if manual checks OK
    let value: Config = serde_yaml::from_str(&body).with_context(|| "Parsing Config YAML")?;
    value.validate().with_context(|| "Validating Config semantics")?;
    Ok(value)
}

fn validate_guest_types(yaml_value: &Value) -> Result<()> {
    if let Value::Mapping(mapping) = yaml_value {
        // Check guests only have one type (e.g. libvirt, docker)
        if let Some(Value::Sequence(machines_seq)) = mapping.get(&Value::String("machines".to_string())) {
            for machine in machines_seq {
                if let Value::Mapping(machine_map) = machine {
                    let mut guest_type_count = 0;
                    if machine_map.contains_key(&Value::String("libvirt".to_string())) { guest_type_count += 1;}
                    if machine_map.contains_key(&Value::String("docker".to_string())) {guest_type_count += 1;}
                    if machine_map.contains_key(&Value::String("android".to_string())) {guest_type_count += 1;}
                    if guest_type_count > 1 {
                        return Err(Error::msg("Machine has more than one guest type defined"));
                    }
                }
            }
        }
    }
    Ok(())
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
