use std::path::PathBuf;
use anyhow::{bail, Context};
use reqwest::Client;
use tokio::sync::mpsc::{Sender};
use kvm_compose_schemas::cli_models::Opts;
use crate::orchestration::{create_logical_testbed, OrchestrationTask, read_previous_state_request, write_state_request};
use crate::parse_config;
use crate::state::orchestration_tasks::{check_if_guest_images_exist, get_orchestration_common};
use crate::state::State;
use kvm_compose_schemas::deployment_models::{Deployment, DeploymentCommand, DeploymentState};
use kvm_compose_schemas::settings::TestbedClusterConfig;
use crate::orchestration::api::{OrchestrationInstruction, OrchestrationProtocol};
use crate::orchestration::websocket::{send_orchestration_instruction_over_channel};
use crate::state::orchestration_tasks::ovn_network::reapply_acl_action;


pub async fn run_orchestration(
    deployment: Deployment,
    action: DeploymentCommand,
    opts: Opts,
    sender: &mut Sender<OrchestrationProtocol>,
) -> anyhow::Result<Deployment> {
    tracing::debug!("running orchestration");

    // get kvm-compose config as it may be needed in orchestration i.e. for OVN
    let client = Client::new();
    let kvm_compose_config_resp = client.get(format!("{}api/config/cluster", opts.server_connection))
        .send()
        .await
        .context("getting kvm-compose-config for orchestrator");

    let kvm_compose_config = match kvm_compose_config_resp {
        Ok(ok) => {
            let serde_conversion: TestbedClusterConfig = serde_json::from_str(&ok.text().await.expect("getting server response"))
                .expect("deserialising kvm-compose-config"); // TODO - this could cause GUI channel to hang if this panics
            serde_conversion
        }
        Err(e) => {
            bail!(e);
        }
    };

    let command_result = orchestration_parse_command(
        action,
        deployment,
        kvm_compose_config,
        sender,
        client,
        opts.server_connection,
    ).await.context("running the chosen orchestration command")?;

    Ok(command_result)
}

pub async fn orchestration_parse_command(
    command: DeploymentCommand,
    mut deployment: Deployment,
    kvm_compose_config: TestbedClusterConfig,
    sender: &mut Sender<OrchestrationProtocol>,
    http_client: Client,
    server_conn: String,
) -> anyhow::Result<Deployment> {
    let yaml = format!("{}/kvm-compose.yaml", deployment.project_location.clone());
    let project_location = PathBuf::from(&deployment.project_location);

    tracing::info!("project location: {:?}", &project_location);

    let project_name = &deployment.name;
    let result_deployment = match command {
        DeploymentCommand::Up { ref up_cmd } => {
            // if we are reapplying acl, shortcut here otherwise continue
            let reapply_acl = up_cmd.reapply_acl.clone();
            if reapply_acl {
                let previous_state = read_previous_state_request(&http_client, &server_conn, &project_name).await?;
                let logical_testbed = create_logical_testbed(&yaml, &deployment, &project_location, false)
                    .await
                    .context("Creating logical testbed")?;
                reapply_acl_action(
                    &previous_state,
                    logical_testbed,
                    sender,
                    deployment.clone(),
                    command.clone(),
                    project_name.clone(),
                    &http_client,
                    &server_conn,
                ).await?;
                Ok(deployment)
            } else {
                // for now, we will ignore the previous state, this will cause state drift bugs
                // will be fixed when we move to OVN, for now just note that there is a previous state
                // so that we can prevent re-running scripts
                let force_provision = up_cmd.provision.clone();
                let force_rerun_scripts = up_cmd.rerun_scripts.clone();
                let previous_state = read_previous_state_request(&http_client, &server_conn, &project_name).await;

                let mut state = match previous_state {
                    Ok(state) => {
                        tracing::info!("There was a previous state file found");
                        if force_provision && force_rerun_scripts {
                            tracing::info!("Provisioning and rerun scripts flags enabled, will rebuild guest images and rerun guest scripts");
                        } else if !force_provision && force_rerun_scripts {
                            tracing::info!("Rerun scripts flag enabled, will rerun guest scripts");
                        } else if force_provision && !force_rerun_scripts {
                            tracing::info!("Provisioning flag enabled, will rebuild guest images");
                        } else if !force_provision && !force_rerun_scripts {
                            tracing::info!("Provisioning and rerun scripts flags not enabled, only making sure the network is up and guests are up");
                        }

                        // need to check if artefacts exist in case state does and artefacts don't exist
                        let mut artefacts_folder = project_location.clone();
                        artefacts_folder.push("artefacts");
                        if !artefacts_folder.exists() && !force_provision {
                            bail!("Artefacts folder does not exist but state does, cannot continue. Either delete the state file or rerun up with the --provision flag.");
                        }

                        // either return a new state or the old state based on force_provision
                        if force_provision {
                            let logical_testbed = create_logical_testbed(&yaml, &deployment, &project_location, force_provision)
                                .await
                                .context("Creating logical testbed")?;
                            tracing::info!("parsed {project_name} kvm-compose.yaml");
                            let state = State::new(&logical_testbed)
                                .context("Creating state from logical testbed")?;
                            write_state_request(&http_client, &server_conn, &project_name, &state)
                                .await
                                .context("Sending the state json file to server to save to disk.")?;

                            tracing::info!("written state for {project_name}");
                            // send the deployment and the command and receive OK
                            send_orchestration_instruction_over_channel(
                                sender,
                                OrchestrationInstruction::Init {
                                    deployment: deployment.clone(),
                                    deployment_command: command.clone(),
                                },
                            ).await.context("sending Init request to server")?;
                            // generate guest images
                            logical_testbed.request_generate_artefacts(sender)
                                .await
                                .context("Generating artefacts from logical testbed")?;
                            state
                        } else {
                            // nothing to do, just init before continue
                            // send the deployment and the command and receive OK
                            send_orchestration_instruction_over_channel(
                                sender,
                                OrchestrationInstruction::Init {
                                    deployment: deployment.clone(),
                                    deployment_command: command.clone(),
                                },
                            ).await.context("sending Init request to server")?;
                            // return the previous state
                            state
                        }
                    }
                    Err(_) => {
                        tracing::info!("There was no state file found, starting a fresh testbed deployment");
                        let logical_testbed = create_logical_testbed(&yaml, &deployment, &project_location, force_provision)
                            .await
                            .context("Creating logical testbed")?;
                        tracing::info!("parsed {project_name} kvm-compose.yaml");
                        let mut state = State::new(&logical_testbed)
                            .context("Creating state from logical testbed")?;
                        // if no state file but one or more guest images exist, dont run scripts to
                        // preserve state, even if generate artefacts does create some images
                        let images_already_exist = check_if_guest_images_exist(&logical_testbed)?;
                        if images_already_exist {
                            tracing::info!("one or more guest images found already to exist, will not run guest scripts unless --provision is set");
                            // set this to true so that we skip setup script stage
                            state.state_provisioning.guests_provisioned = true;
                        }
                        write_state_request(&http_client, &server_conn, &project_name, &state)
                            .await
                            .context("Sending the state json file to server to save to disk.")?;
                        tracing::info!("written state for {project_name}");
                        // send the deployment and the command and receive OK
                        send_orchestration_instruction_over_channel(
                            sender,
                            OrchestrationInstruction::Init {
                                deployment: deployment.clone(),
                                deployment_command: command.clone(),
                            },
                        ).await.context("sending Init request to server")?;
                        // generate guest images
                        logical_testbed.request_generate_artefacts(sender)
                            .await
                            .context("Generating artefacts from logical testbed")?;
                        state
                    }
                };

                let common = get_orchestration_common(&state, force_provision, force_rerun_scripts, reapply_acl, kvm_compose_config).await?;
                // now the state has been written, we can ask the server to run orchestration
                let up_res = state.request_create_action(&common, sender).await.context("requesting create action");
                match up_res {
                    Ok(_) => deployment.state = DeploymentState::Up,
                    Err(err) => {
                        deployment.state = DeploymentState::Failed(command.clone());
                        bail!("{err:#}");
                    },
                }
                // update state with provisioning state, even if fail provisioning may have been partial
                state.state_provisioning.guests_provisioned = true;
                write_state_request(&http_client, &server_conn, &project_name, &state)
                    .await
                    .context("Sending the state json file to server to save to disk.")?;
                Ok(deployment)
            }
        }
        DeploymentCommand::Down => {
            let mut state_path = project_location.clone();
            state_path.push(format!("{}-state.json", project_name));
            tracing::info!("reading existing state at {state_path:?}");

            send_orchestration_instruction_over_channel(
                sender,
                OrchestrationInstruction::Init {
                    deployment: deployment.clone(),
                    deployment_command: command.clone(),
                },
            ).await.context("sending Init request to server")?;

            // get state from file, should destroy only what is up
            let previous_state = read_previous_state_request(&http_client, &server_conn, &project_name).await;
            match previous_state {
                Ok(old_state) => {
                    let common = get_orchestration_common(&old_state, false, false, false, kvm_compose_config).await?;
                    match old_state.request_destroy_action(&common, sender).await {
                        Ok(_) => deployment.state = DeploymentState::Down,
                        Err(err) => {
                            deployment.state = DeploymentState::Failed(command.clone());
                            bail!("{err:#}");
                        },
                    }
                }
                Err(err) => {
                    tracing::error!("{err:#}");
                    deployment.state = DeploymentState::Failed(command.clone());
                }
            }
            Ok(deployment)
        }
        DeploymentCommand::GenerateArtefacts => {
            // TODO - disallow this if a deployment is already up as we dont want to overwrite the
            //  state as this could be different
            // create logical testbed to get a state
            let logical_testbed = parse_config(
                yaml.clone(),
                Some(deployment.name.clone()),
                true,
                project_location.clone(),
                false,
            )
                .await
                .context("Failed to parse the yaml config and create a logical testbed")?;
            let project_name = &logical_testbed.common.project.clone();
            tracing::info!("parsed {project_name} config");
            let state = State::new(&logical_testbed)
                .context("creating state from logical testbed")?;
            // write state
            write_state_request(&http_client, &server_conn, &project_name, &state)
                .await
                .context("Sending the state json file to server to save to disk.")?;
            tracing::info!("written state for {project_name}");

            send_orchestration_instruction_over_channel(
                sender,
                OrchestrationInstruction::Init {
                    deployment: deployment.clone(),
                    deployment_command: command.clone(),
                },
            ).await.context("sending Init request to server")?;

            // generate guest images
            match logical_testbed.request_generate_artefacts(sender)
                .await
                .context("Failed to generate artefacts from logical testbed") {
                Ok(_) => {}
                Err(err) => {
                    deployment.state = DeploymentState::Failed(command.clone());
                    bail!("{err:#}");
                },
            }
            Ok(deployment)
        }
        DeploymentCommand::ClearArtefacts => {
            // destroy_remote_project_folders(&common).await?;
            if let Ok(_) = read_previous_state_request(&http_client, &server_conn, &project_name).await {

                send_orchestration_instruction_over_channel(
                    sender,
                    OrchestrationInstruction::Init {
                        deployment: deployment.clone(),
                        deployment_command: command.clone(),
                    },
                ).await.context("sending Init request to server")?;

                // run clear artefacts on the server
                send_orchestration_instruction_over_channel(
                    sender,
                    OrchestrationInstruction::ClearArtefacts,
                ).await.context("requesting the execution of guest setup scripts")?;

            } else {
                tracing::error!("could not run clear artefacts, no state file");
                deployment.state = DeploymentState::Failed(command.clone());
            }
            Ok(deployment)
        }
        DeploymentCommand::Snapshot { ref snapshot_cmd } => {
            if let Ok(_) = read_previous_state_request(&http_client, &server_conn, &project_name).await {

                send_orchestration_instruction_over_channel(
                    sender,
                    OrchestrationInstruction::Init {
                        deployment: deployment.clone(),
                        deployment_command: command.clone(),
                    },
                ).await.context("sending Init request to server")?;

                send_orchestration_instruction_over_channel(
                    sender,
                    OrchestrationInstruction::Snapshot(snapshot_cmd.clone()),
                ).await.context("sending snapshot request to server")?;

            } else {
                tracing::error!("could not run snapshot command, no state file");
            }
            Ok(deployment)
        }
        DeploymentCommand::TestbedSnapshot { snapshot_guests } => {
            if let Ok(_) = read_previous_state_request(&http_client, &server_conn, &project_name).await {

                send_orchestration_instruction_over_channel(
                    sender,
                    OrchestrationInstruction::Init {
                        deployment: deployment.clone(),
                        deployment_command: command.clone(),
                    },
                ).await.context("sending Init request to server")?;

                send_orchestration_instruction_over_channel(
                    sender,
                    OrchestrationInstruction::TestbedSnapshot {
                        snapshot_guests,
                    },
                ).await.context("sending snapshot request to server")?;

                tracing::info!("testbed snapshot complete");
            } else {
                tracing::error!("could not run testbed snapshot command, no state file");
            }
            Ok(deployment)
        }
        DeploymentCommand::AnalysisTool(ref tool) => {
            if let Ok(_) = read_previous_state_request(&http_client, &server_conn, &project_name).await {
                tracing::info!("running analysis tool: {tool:?}");

                send_orchestration_instruction_over_channel(
                    sender,
                    OrchestrationInstruction::Init {
                        deployment: deployment.clone(),
                        deployment_command: command.clone(),
                    },
                ).await.context("sending Init request to server")?;

                send_orchestration_instruction_over_channel(
                    sender,
                    OrchestrationInstruction::AnalysisTool(tool.clone()),
                ).await.context("sending snapshot request to server")?;


            } else {
                tracing::error!("could not run testbed analysis tool command, no state file");
            }
            Ok(deployment)
        }
        DeploymentCommand::Exec(ref exec_cmd) => {
            if let Ok(_) = read_previous_state_request(&http_client, &server_conn, &project_name).await {

                send_orchestration_instruction_over_channel(
                    sender,
                    OrchestrationInstruction::Init {
                        deployment: deployment.clone(),
                        deployment_command: command.clone(),
                    },
                ).await.context("sending Init request to server")?;

                send_orchestration_instruction_over_channel(
                    sender,
                    OrchestrationInstruction::Exec(exec_cmd.clone()),
                ).await.context("sending Exec request to server")?;

            } else {
                tracing::error!("could not run testbed snapshot command, no state file");
            }
            Ok(deployment)
        }
        DeploymentCommand::ListCloudImages => {
            send_orchestration_instruction_over_channel(
                sender,
                OrchestrationInstruction::Init {
                    deployment: deployment.clone(),
                    deployment_command: command.clone(),
                },
            ).await.context("sending Init request to server")?;
            send_orchestration_instruction_over_channel(
                sender,
                OrchestrationInstruction::ListCloudImages,
            ).await.context("sending List Cloud Images request to server")?;
            Ok(deployment)
        }
    };
    tracing::info!("finished generating commands");
    result_deployment
}
