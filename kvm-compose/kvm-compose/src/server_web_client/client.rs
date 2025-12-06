use crate::server_web_client::http_actions;
use crate::get_project_name;
use anyhow::{bail, Context};
use kvm_compose_schemas::cli_models::{DeploymentCmd, DeploymentSubCommand, Opts, SubCommand};
use kvm_compose_schemas::deployment_models::{Deployment, DeploymentCommand, DeploymentState};
use reqwest::Client;
use crate::orchestration::websocket::ws_orchestration_client;
use crate::server_web_client::deployment::reset_state;

pub async fn orchestration_action(
    client: &Client,
    opts: Opts,
) -> anyhow::Result<()> {
    let server_url = opts.server_connection.clone();
    let cmd_name = opts.sub_command.name();
    tracing::info!("running {cmd_name} command");
    let project_name = get_project_name(opts.project_name.clone())
        .context("getting project name")?;

    // get deployment or create
    let deployment = {
        if let Ok(deployment) = http_actions::check_deployment(&client, &project_name, &opts.server_connection).await {
            // make sure that the recorded deployment location is the same as the current folder
            ensure_current_folder_matches_deployment(&deployment)?;
            deployment
        } else {
            tracing::info!("deployment {project_name} doesnt exist, creating");
            http_actions::create_deployment(&client, &project_name, &opts.server_connection).await
                .context("creating deployment before orchestration")?;
            http_actions::check_deployment(&client, &project_name, &opts.server_connection).await
                .context("checking deployment before orchestration")?
        }
    };

    match opts.sub_command {
        SubCommand::Up(_) | SubCommand::Down | SubCommand::GenerateArtefacts |
        SubCommand::ClearArtefacts | SubCommand::Snapshot(_) | SubCommand::TestbedSnapshot(_) => {
            // these are destructive commands, don't allow running during other destructive cmds
            match &deployment.state {
                DeploymentState::Running => bail!("deployment in Running state, cannot run orchestration command"),
                _ => {}
            }
        }
        _ => {
            // allow other commands
        }
    }

    let action = get_deployment_action(&opts)?;

    // open websocket to server to start the orchestration
    let mut runner_url = format!(
        "{}api/orchestration/ws",
        &server_url,
    );
    runner_url = runner_url.replace("http://", "ws://");
    tracing::debug!("orchestration runner url = {runner_url}");

    // TODO - should this be triggered with a tokio::spawn() ?
    let orchestration_result = ws_orchestration_client(
        runner_url.clone(),
        deployment.clone(),
        action.clone(),
        opts,
    )
        .await
        .context("connecting websocket to testbed server for orchestration");

    // handle result from `orchestration_result`
    let success = match orchestration_result {
        Ok(success) => {
            tracing::info!("websocket closed Ok");
            success
        }
        Err(err) => {
            tracing::error!("there was a problem in the command running, will stop");
            err.chain().for_each(|cause| tracing::error!("because: {}", cause));
            false
        }
    };

    let check_deployment =
        http_actions::check_deployment(&client, &project_name, &server_url).await
            .context("checking deployment after orchestration")?;

    // TODO - make sure the database update matches result?
    tracing::debug!("deployment is now in {:?} state", &check_deployment.state);

    if success {
        Ok(())
    } else {
        bail!("Server reported command failure")
    }

}

pub fn get_deployment_action(
    opts: &Opts,
) -> anyhow::Result<DeploymentCommand> {
    Ok(match &opts.sub_command {
        SubCommand::GenerateArtefacts => {
            DeploymentCommand::GenerateArtefacts
        }
        SubCommand::ClearArtefacts => {
            DeploymentCommand::ClearArtefacts
        }
        SubCommand::Up(up_cmd) => {
            DeploymentCommand::Up { up_cmd: up_cmd.clone(), }
        }
        SubCommand::Down => {
            DeploymentCommand::Down
        }
        SubCommand::Snapshot(snp_cmd) => {
            DeploymentCommand::Snapshot { snapshot_cmd: snp_cmd.sub_command.clone(), }
        }
        SubCommand::TestbedSnapshot(tb_snp) => {
            DeploymentCommand::TestbedSnapshot { snapshot_guests: tb_snp.snapshot_guests, }
        }
        SubCommand::Tools(at_cmd) => {
            DeploymentCommand::Tool(at_cmd.clone())
        }
        SubCommand::Exec(exec_cmd) => {
            DeploymentCommand::Exec(exec_cmd.clone())
        }
        SubCommand::CloudImages => {
            DeploymentCommand::ListCloudImages
        }
        _ => bail!("not an orchestration command"),
    })
}

/// This set of commands will control the deployments on the server
pub async fn deployment_action(client: &Client, opts: &Opts, dep_cmd: &DeploymentCmd) -> anyhow::Result<()> {
    match &dep_cmd.sub_command {
        DeploymentSubCommand::Create(_name) => unimplemented!(),
        DeploymentSubCommand::Destroy(_name) => unimplemented!(),
        DeploymentSubCommand::List => unimplemented!(),
        DeploymentSubCommand::Info(_name) => unimplemented!(),
        // allow user to set the state manually in case it is stuck on running?
        DeploymentSubCommand::ResetState(name) => reset_state(name, client, opts).await,
    }
}

/// Prevent commands running in a folder with the same name as an existing deployment, which would
/// cause mismatch in configuration and artefact usage.
fn ensure_current_folder_matches_deployment(
    deployment: &Deployment
) -> anyhow::Result<()> {
    let current_dir = std::env::current_dir()?.display().to_string();
    if !deployment.project_location.eq(&current_dir) {
        bail!("there is an existing deployment with the same name '{}' at '{}'", deployment.name, deployment.project_location)
    }
    Ok(())
}
