use anyhow::bail;
use kvm_compose_schemas::deployment_models::{
    Deployment, NewDeployment,
};
use reqwest::Response;

/// Reusable helper method for parsing the response from the server for command results
async fn parse_response(resp: Response, command_name: String) -> anyhow::Result<String> {
    let http_code = resp.status();
    let text_response = resp.text().await?;

    if http_code.is_success() {
        tracing::info!("{command_name} command successful");
    } else {
        tracing::error!(
            "command was not successful with code {:?}",
            http_code.to_string()
        );
        // exit cli with error as we cannot continue
        bail!("server response: {:?}", text_response);
    }
    Ok(text_response)
}

/// This will create a deployment on the server
pub async fn create_deployment(
    client: &reqwest::Client,
    project_name: &String,
    server_url: &String,
) -> anyhow::Result<()> {
    tracing::info!("creating deployment for '{project_name}'");
    let server_api = format!("{}api/deployments", server_url);
    tracing::trace!("api url used = {:?}", &server_api);

    let json = NewDeployment {
        name: project_name.clone(),
        project_location: std::env::current_dir()?.display().to_string(),
    };

    let resp = client.post(server_api).json(&json).send().await?;

    parse_response(resp, "create deployment".into()).await?;
    Ok(())
}

/// This checks if the deployments exists in the server and returns the deployment data if it does
pub async fn check_deployment(
    client: &reqwest::Client,
    project_name: &String,
    server_url: &String,
) -> anyhow::Result<Deployment> {
    // TODO - make sure server returns a 404 if doesnt exist
    tracing::debug!("getting deployment info for '{project_name}'");
    let server_api = format!("{}api/deployments/{project_name}", server_url);
    tracing::trace!("api url used = {:?}", &server_api);
    let resp = client.get(server_api).send().await?;
    if resp.status().is_success() {
        let serde_conversion: Deployment = serde_json::from_str(&resp.text().await?)?;
        Ok(serde_conversion)
    } else {
        // deployment not returned, probably doesnt yet exist
        bail!(resp.text().await?)
    }
}

/// Update a deployments state on the server database
pub async fn update_deployment_state(
    client: &reqwest::Client,
    project_name: &String,
    server_url: &String,
    deployment: Deployment,
) -> anyhow::Result<()> {
    tracing::info!("updating deployment info for '{project_name}'");
    let server_api = format!("{}api/deployments/{project_name}", server_url);
    tracing::trace!("api url used = {:?}", &server_api);


    let resp = client
        .put(server_api)
        .json(&deployment)
        .send()
        .await?;

    parse_response(resp, "update deployment state".into()).await?;
    Ok(())
}

