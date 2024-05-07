use std::path::PathBuf;
use tokio::process::{Child, Command};
use anyhow::{bail, Context};
use kvm_compose_lib::state::State;
use kvm_compose_schemas::deployment_models::Deployment;

pub mod handlers;
pub mod models;
pub mod providers;
pub mod db;
pub mod deployments;

pub fn run_orchestration_command(
    log_file_path: &String,
    deployment_name: &String,
    deployment_command_string: &String,
) -> anyhow::Result<Child> {
    Command::new("sudo")
        .arg("testbedos-orchestrate")
        .arg(log_file_path)
        .arg(deployment_name)
        .arg(deployment_command_string)
        .spawn()
        .with_context(|| "running orchestrator")
}

/// This function will get the state json for the deployment. It will check if it exists due to an
/// artefact generation being run previously.
pub async fn get_state_json(deployment: Deployment) -> anyhow::Result<State, anyhow::Error> {
    let project_name = &deployment.name;
    let state_json_location = format!(
        "{}/{}-state.json",
        deployment.project_location.clone(),
        project_name
    );

    let path = PathBuf::from(state_json_location);
    if path.is_file() {
        let text = tokio::fs::read_to_string(path).await?;
        let config: State = serde_json::from_str(&text)?;
        Ok(config)
    } else {
        bail!(
            "could not read state json for {project_name}, has artefact generation been executed?"
        )
    }
}
