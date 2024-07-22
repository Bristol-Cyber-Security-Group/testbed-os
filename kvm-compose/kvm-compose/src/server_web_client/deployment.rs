use anyhow::bail;
use reqwest::Client;
use kvm_compose_schemas::cli_models::{DeploymentName, Opts};
use kvm_compose_schemas::deployment_models::{Deployment, DeploymentState};

/// This is a helper function to change the state of the deployment from whatever it is, back to the
/// `Down` state. This is only useful while we still have the current DeploymentState implementation
/// which can cause the state to be stuck in `Running` if something goes wrong.
pub async fn reset_state(
    deployment_name: &DeploymentName,
    client: &Client,
    opts: &Opts
) -> anyhow::Result<()> {

    // get the current deployment via server API
    let server_api = format!("{}api/deployments/{}", &opts.server_connection, &deployment_name.name);
    tracing::trace!("api url used = {:?}", &server_api);
    let resp = client.get(&server_api).send().await?;

    let mut deployment = if resp.status().is_success() {
        let serde_conversion: Deployment = serde_json::from_str(&resp.text().await?)?;
        serde_conversion
    } else {
        bail!("could not get deployment from server for '{}'", &deployment_name.name);
    };

    // edit to update the state to down
    deployment.state = DeploymentState::Down;

    // send back to the server
    let resp = client.put(&server_api)
        .json(&deployment)
        .send().await?;

    if !resp.status().is_success() {
        bail!("could not reset deployment '{}' state to down", &deployment_name.name);
    }

    tracing::info!("reset deployment '{}' state to down", &deployment_name.name);

    Ok(())
}
