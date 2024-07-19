use std::io::SeekFrom;
use std::path::PathBuf;
use tokio::process::{Child, Command};
use anyhow::{bail, Context};
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncSeekExt};
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
        // get the file
        let mut state_file = File::open(&path).await?;
        // now we will reset the cursor to the start of the file in case of a previous read leaving
        // the cursor at the end
        state_file.seek(SeekFrom::Start(0)).await?;
        // read the file into the buffer to then be parsed by serde
        let mut buffer = String::new();
        state_file.read_to_string(&mut buffer).await.context("read state json from disk")?;

        let config: State = serde_json::from_str(&buffer).context("parse state json with serde")?;
        Ok(config)
    } else {
        bail!(
            "could not read state json for {project_name}, has artefact generation been executed?"
        )
    }
}

/// This function will set the state json for the deployment.
pub async fn set_state_json(deployment: Deployment, state: State) -> anyhow::Result<()> {
    let project_name = &deployment.name;
    let project_location = PathBuf::from(deployment.project_location.clone());
    state
        .write(&project_name, &project_location)
        .await?;
    Ok(())
}
