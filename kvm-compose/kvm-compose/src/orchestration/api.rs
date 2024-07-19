use std::path::PathBuf;
use anyhow::{anyhow, bail, Context};
use futures_util::future::join_all;
use nix::unistd::{Gid, Uid};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::Sender;
use kvm_compose_schemas::cli_models::{AnalysisToolsCmd, AnalysisToolsSubCmd, SnapshotSubCommand};
use kvm_compose_schemas::deployment_models::{Deployment, DeploymentCommand};
use kvm_compose_schemas::exec::ExecCmd;
use kvm_compose_schemas::kvm_compose_yaml::machines::GuestType;
use kvm_compose_schemas::kvm_compose_yaml::machines::libvirt_image_download::OnlineCloudImage;
use crate::analysis_tools::packet_capture::packet_capture;
use crate::exec::prepare_guest_exec_command;
use crate::orchestration::{create_remote_project_folders, OrchestrationCommon, OrchestrationGuestTask};
use crate::orchestration::ssh::SSHClient;
use crate::ovn::components::acl::LogicalACLRecord;
use crate::ovn::components::logical_router::LogicalRouter;
use crate::ovn::components::logical_router_port::LogicalRouterPort;
use crate::ovn::components::logical_switch::LogicalSwitch;
use crate::ovn::components::logical_switch_port::LogicalSwitchPort;
use crate::ovn::components::ovs::OvsPort;
use crate::ovn::configuration::dhcp::DhcpDatabaseEntry;
use crate::ovn::configuration::external_gateway::OvnExternalGateway;
use crate::ovn::configuration::nat::OvnNat;
use crate::ovn::configuration::route::OvnRoute;
use crate::ovn::OvnCommand;
use crate::snapshot::snapshot_cmd::run_snapshot_action;
use crate::snapshot::testbed_snapshot::run_testbed_snapshot_action;
use crate::snapshot::TestbedSnapshots;
use crate::state::orchestration_tasks::*;
use crate::state::orchestration_tasks::guests::*;
use crate::state::orchestration_tasks::ovn_network::*;
use crate::state::{State, StateTestbedGuest};
use crate::state::orchestration_tasks::generate_artefacts::generate_artefacts;

// here we define the different atomic things we can send to the testbed server to trigger an
// orchestration action, we wrap each component in an enum so that it is easy to de/serialise at
// the endpoint and at the sender

/// This struct is sent to the server for every resource that is to be created. It contains the orchestration
/// common, which has information about the state, and the resource itself. The resource is wrapped in enums
/// such that the code paths are easy to be determined and if in the future more are added then it is easy
/// to refactor code or add new code.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OrchestrationProtocol {
    // pub common: OrchestrationCommon,
    pub instruction: OrchestrationInstruction,
}

impl OrchestrationProtocol {
    /// Run all instructions in this protocol
    pub async fn run(&self, state: &State, common: &OrchestrationCommon, logging_send: &Sender<OrchestrationLogger>) -> anyhow::Result<OrchestrationProtocolResponse> {
        self.instruction.run(state, common, logging_send).await
    }

    /// Run only for init without state
    pub async fn run_init(self) -> anyhow::Result<OrchestrationProtocolResponse> {
        self.instruction.run_init().await
    }

    pub fn to_string(&self) -> serde_json::Result<String> {
        serde_json::to_string(self)
    }
}

/// Represents all possible instructions for the `OrchestrationProtocol`. Create, Destroy and Edit wrap the
/// `OrchestrationResource` in a `Vec` to allow batching of instructions.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum OrchestrationInstruction {
    /// Get the deployment info before any set of instructions over websockets can continue
    Init {
        deployment: Deployment,
        deployment_command: DeploymentCommand,
    },
    /// Check if the testbed hosts for the state of the given deployment are up
    TestbedHostCheck,
    /// Make sure artefact folder exists on all relevant testbed hosts
    Setup,
    /// Create the libvirt network for provisioning backing image Libvirt guests
    CreateTempNetwork(OrchestrationCommon),
    /// Destroy the libvirt network for provisioning backing image libvirt guests
    DestroyTempNetwork(OrchestrationCommon),
    /// Generate the artefacts for all the guests in the deployment
    GenerateArtefacts {
        project_path: String,
        uid: u32,
        gid: u32,
    },
    /// Destroy all guests (if up) and then clear the artefacts for the given deployment on all testbed hosts
    ClearArtefacts,
    /// Setup images for the guests in the list, this is based on the internal guest representation i.e. normal guest
    /// backing image, clone image - for all guest types where relevant
    SetupImage(Vec<OrchestrationResource>),
    /// Push all artefacts for each guest to the remote testbed hosts
    PushArtefacts(Vec<OrchestrationResource>),
    /// Push all backing images to the testbed hosts that will run guests that are linked clones
    PushBackingImages(Vec<OrchestrationResource>),
    /// Rebase all linked clones that exist on remote testbed hosts to point to the local copy of the backing image
    RebaseRemoteBackingImages(Vec<OrchestrationResource>),
    /// Deploy all testbed resources in the list
    Deploy(Vec<OrchestrationResource>),
    /// Destroy all testbed resources in the list
    Destroy(Vec<OrchestrationResource>),
    /// Edit all testbed resources in the list
    Edit(Vec<OrchestrationResource>),
    /// Run setup scripts for all guests in the list
    RunSetupScripts(Vec<OrchestrationResource>),
    /// Run snapshot command for one guest
    Snapshot(SnapshotSubCommand),
    /// Run the testbed snapshot command
    TestbedSnapshot {
        snapshot_guests: bool,
    },
    /// Run an analysis tool
    AnalysisTool(AnalysisToolsCmd),
    /// Return the list of supported cloud images
    ListCloudImages,
    /// Run an exec command
    Exec(ExecCmd),
    /// Instruct the orchestration to cancel
    Cancel,
    /// Internal use to show that the commands have finished generating
    End,
}

impl OrchestrationInstruction {

    fn format_message(
        result: Vec<anyhow::Result<()>>,
        result_messages: &mut Vec<OrchestrationInstructionResultMessage>,
        res_name_list: Vec<String>
    ) {
        for idx in 0..result.len() {
            let (is_success, message) = match &result[idx] {
                Ok(_) => {
                    (true, res_name_list[idx].clone())
                }
                Err(err) => {
                    (false, format!("{} with error: {:#}", res_name_list[idx].clone(), err))
                }
            };
            result_messages.push(OrchestrationInstructionResultMessage {
                is_success,
                message,
            });
        }
    }

    /// Get name of the instructions for logging, include the resource name for instructions that are lists
    pub fn name(&self) -> String {
        let mut instruction = String::new();
        match self {
            OrchestrationInstruction::Init { .. } => instruction.push_str("Init"),
            OrchestrationInstruction::TestbedHostCheck => instruction.push_str("Testbed Host Check"),
            OrchestrationInstruction::Setup => instruction.push_str("Setup"),
            OrchestrationInstruction::Deploy(items) => {
                instruction.push_str("Deploy ");
                format_instruction_message(&mut instruction, items);
            }
            OrchestrationInstruction::Destroy(items) => {
                instruction.push_str("Destroy ");
                format_instruction_message(&mut instruction, items);
            }
            OrchestrationInstruction::Edit(items) => {
                instruction.push_str("Edit ");
                format_instruction_message(&mut instruction, items);
            }
            OrchestrationInstruction::CreateTempNetwork(_) => instruction.push_str("Create Temporary Network"),
            OrchestrationInstruction::DestroyTempNetwork(_) => instruction.push_str("Destroy Temporary Network"),
            OrchestrationInstruction::PushArtefacts(items) => {
                instruction.push_str("Pushing Artefacts for ");
                format_instruction_message(&mut instruction, items);
            }
            OrchestrationInstruction::SetupImage(items) => {
                instruction.push_str("Setting Up Images for ");
                format_instruction_message(&mut instruction, items);
            }
            OrchestrationInstruction::PushBackingImages(items) => {
                instruction.push_str("Pushing Backing Images for ");
                format_instruction_message(&mut instruction, items);
            }
            OrchestrationInstruction::RebaseRemoteBackingImages(items) => {
                instruction.push_str("Rebasing Backing Images for ");
                format_instruction_message(&mut instruction, items);
            }
            OrchestrationInstruction::RunSetupScripts(items) => {
                instruction.push_str("Running Setup Scripts for ");
                format_instruction_message(&mut instruction, items);
            }
            OrchestrationInstruction::GenerateArtefacts{ .. } => instruction.push_str("Generate Artefacts"),
            OrchestrationInstruction::ClearArtefacts{ .. } => instruction.push_str("Clear Artefacts"),
            OrchestrationInstruction::Snapshot(s) => instruction.push_str(&format!("Snapshot {}", s.name())),
            OrchestrationInstruction::TestbedSnapshot { snapshot_guests } => {
                if *snapshot_guests {
                    instruction.push_str("Testbed Snapshot with all guest Snapshot")
                } else {
                    instruction.push_str("Testbed Snapshot")
                }
            }
            OrchestrationInstruction::AnalysisTool(at) => {
                instruction.push_str(&format!("Analysis Tool {}", at.name()))
            }
            OrchestrationInstruction::Exec(e) => {
                instruction.push_str(&format!("Exec {}", e.name()))
            }
            OrchestrationInstruction::ListCloudImages => instruction.push_str("List Cloud Images"),
            OrchestrationInstruction::Cancel => {
                instruction.push_str("Cancel")
            }
            OrchestrationInstruction::End => {
                instruction.push_str("End")
            }
        }
        instruction
    }

    /// Special run for init
    async fn run_init(&self) -> anyhow::Result<OrchestrationProtocolResponse> {
        Ok(match self {
            OrchestrationInstruction::Init { .. } => {
                // always true because the actual setup for init happens just after the websocket handshake in
                // `handle_orchestration_socket()` on the server side
                OrchestrationProtocolResponse::Generic { is_success: true, message: "Running Init".to_string() }
            }
            _ => bail!("Did not start orchestration with an Init instruction"),
        })
    }

    /// This will run the instruction received from the client. For Create, Destroy and Edit we support batch sending
    /// of instructions.
    async fn run(
        &self,
        state: &State,
        orchestration_common: &OrchestrationCommon,
        logging_send: &Sender<OrchestrationLogger>
    ) -> anyhow::Result<OrchestrationProtocolResponse> {
        let protocol_response = match self {
            OrchestrationInstruction::Init { .. } => bail!("Sent an Init instruction after an Init has already been sent"),
            OrchestrationInstruction::TestbedHostCheck => {
                let mut message = String::new();
                let mut is_success = false;
                let up_check = check_if_testbed_hosts_up(orchestration_common).await;
                if up_check.is_ok() {
                    message.push_str("Testbeds are up");
                } else {
                    message.push_str("Testbeds are not up");
                }
                if up_check.is_ok()  {
                    is_success = true;
                }
                OrchestrationProtocolResponse::Single(
                    OrchestrationInstructionResultMessage {
                        is_success,
                        message,
                    }
                )
            }
            OrchestrationInstruction::Setup => {
                let folder_create_res = create_remote_project_folders(orchestration_common).await;

                let mut message = String::new();
                let mut is_success = false;

                if folder_create_res.is_ok() {
                    message.push_str("folders were created");
                } else {
                    message.push_str("folders could not be created");
                }

                if folder_create_res.is_ok() {
                    is_success = true;
                }

                OrchestrationProtocolResponse::Single(
                    OrchestrationInstructionResultMessage {
                        is_success,
                        message,
                    }
                )
            }
            OrchestrationInstruction::Deploy(create_list) => {
                // create futures and get the name list, since the vec is ordered by insertion then the name list will
                // be in the same order
                let mut create_futures = Vec::new();
                let mut res_name_list = Vec::new();

                for create in create_list {
                    create_futures.push(create.get_create_future(orchestration_common.clone()));
                    res_name_list.push(create.name());
                }
                // join all futures and collect results
                let result = join_all(create_futures).await;

                let mut result_messages = Vec::new();
                // create message list
                Self::format_message(result, &mut result_messages, res_name_list);

                // finally return the response
                OrchestrationProtocolResponse::List(result_messages)
            }
            OrchestrationInstruction::Destroy(destroy_list) => {
                // create futures and get the name list, since the vec is ordered by insertion then the name list will
                // be in the same order
                let mut destroy_futures = Vec::new();
                let mut res_name_list = Vec::new();

                for create in destroy_list {
                    destroy_futures.push(create.get_destroy_future(orchestration_common.clone()));
                    res_name_list.push(create.name());
                }
                // join all futures and collect results
                let result = join_all(destroy_futures).await;

                let mut result_messages = Vec::new();
                // create message list
                Self::format_message(result, &mut result_messages, res_name_list);

                // finally return the response
                OrchestrationProtocolResponse::List(result_messages)
            }
            OrchestrationInstruction::Edit(_) => {
                unimplemented!()
            }
            OrchestrationInstruction::CreateTempNetwork(common) => {
                let res = turn_on_temporary_network(
                    &common.project_name,
                    &common.project_working_dir.to_str().context("getting project path")?.to_string(),
                    common,
                ).await;
                let is_success = res.is_ok();
                OrchestrationProtocolResponse::Generic {
                    is_success,
                    message: "Creating Temp Network".to_string(),
                }
            }
            OrchestrationInstruction::DestroyTempNetwork(common) => {
                let res = turn_off_temporary_network(
                    &common.project_name,
                    &common.project_working_dir.to_str().context("getting project path")?.to_string()
                ).await;
                let is_success = res.is_ok();
                OrchestrationProtocolResponse::Generic {
                    is_success,
                    message: "Destroying Temp Network".to_string(),
                }
            }
            OrchestrationInstruction::GenerateArtefacts{ project_path, uid, gid } => {
                let artefacts_folder = PathBuf::from(format!("{project_path}/artefacts"));
                // try to create folder, otherwise return false as failed
                let create_folder_res = if !artefacts_folder.exists() {
                    if tokio::fs::create_dir(&artefacts_folder).await.is_ok() {
                        let chown_res = nix::unistd::chown(&artefacts_folder, Some(Uid::from_raw(*uid)), Some(Gid::from_raw(*gid)));
                        match chown_res {
                            Ok(_) => Ok(()),
                            Err(err) => Err(anyhow!("{err:#}")),
                        }
                    } else {
                        Err(anyhow!("could not create artefacts folder"))
                    }
                } else {
                    Ok(())
                };
                match create_folder_res {
                    Ok(_) => {
                        let res = generate_artefacts(
                            state,
                            orchestration_common,
                        ).await;
                        match res {
                            Ok(_) => OrchestrationProtocolResponse::Generic {
                                is_success: true,
                                message: "Generating Artefacts".to_string(),
                            },
                            Err(err) => OrchestrationProtocolResponse::Generic {
                                is_success: false,
                                message: format!("Generating Artefacts failed with error: {err:#}"),
                            }
                        }
                    }
                    Err(err) => OrchestrationProtocolResponse::Generic {
                        is_success: true,
                        message: format!("Generating Artefacts failed with error: {err:#}"),
                    }
                }
            }
            OrchestrationInstruction::ClearArtefacts => {
                let res = clear_artefacts(state, orchestration_common).await;
                match res {
                    Ok(_) => OrchestrationProtocolResponse::Generic { is_success: true, message: "Clearing Artefacts".to_string() },
                    Err(err) => OrchestrationProtocolResponse::Generic { is_success: false, message: format!("Clearing Artefacts, {err:#}") },
                }
            }
            OrchestrationInstruction::SetupImage(list) => {
                // this handles setting up backing images and clones of backing image
                
                Self::setup_image_helper(list, orchestration_common).await
            }
            OrchestrationInstruction::PushArtefacts(list) => {
                // create futures and get the name list, since the vec is ordered by insertion then the name list will
                // be in the same order
                let mut futures = Vec::new();
                let mut res_name_list = Vec::new();

                for create in list {
                    futures.push(create.get_push_image_future(orchestration_common.clone()));
                    res_name_list.push(create.name());
                }
                // join all futures and collect results
                let result = join_all(futures).await;

                let mut result_messages = Vec::new();
                // create message list
                Self::format_message(result, &mut result_messages, res_name_list);

                // finally return the response
                OrchestrationProtocolResponse::List(result_messages)
            }
            OrchestrationInstruction::PushBackingImages(list) => {
                // based on `calculate_backing_images_to_push` code from old state deploy

                let mut futures = Vec::new();
                let mut res_name_list = Vec::new();

                for resource in list {
                    match resource {
                        OrchestrationResource::Guest(g) => {
                            match &g.guest_type.guest_type {
                                GuestType::Libvirt(l) => {
                                    let backing_image_name = l.is_clone_of.as_ref().unwrap();
                                    let guest_testbed = g.testbed_host.as_ref().unwrap();
                                    // from the images_to_push set, work out the local path on master and the remote path
                                    // on the target testbed host
                                    let local_src = get_backing_image_local_path(
                                        &state.testbed_guests,
                                        backing_image_name)?;
                                    let backing_image_remote_path = get_backing_image_remote_path(
                                        orchestration_common,
                                        &state.testbed_guests,
                                        backing_image_name,
                                        guest_testbed)?;
                                    // remove the filename so we have just the parent folder
                                    let remote_dst = PathBuf::from(backing_image_remote_path)
                                        .parent().context("getting parent for backing image folder path")?
                                        .to_str().context("converting parent folder to string")?
                                        .to_string();

                                    let target_testbed_host = guest_testbed.clone();
                                    futures.push(Box::pin(SSHClient::push_file_to_remote_testbed(orchestration_common, target_testbed_host, local_src, remote_dst, false)));
                                    res_name_list.push(resource.name());
                                }
                                GuestType::Docker(_) => {}
                                GuestType::Android(_) => {}
                            }
                        }
                        OrchestrationResource::Network(_) => unreachable!(),
                    }
                }
                let result = join_all(futures).await;
                let mut result_messages = Vec::new();
                // create message list
                Self::format_message(result, &mut result_messages, res_name_list);

                // finally return the response
                OrchestrationProtocolResponse::List(result_messages)

            }
            OrchestrationInstruction::RebaseRemoteBackingImages(list) => {
                // create futures and get the name list, since the vec is ordered by insertion then the name list will
                // be in the same order
                let mut futures = Vec::new();
                let mut res_name_list = Vec::new();

                for create in list {
                    futures.push(create.get_rebase_clone_future(orchestration_common.clone(), state));
                    res_name_list.push(create.name());
                }
                // join all futures and collect results
                let result = join_all(futures).await;

                let mut result_messages = Vec::new();
                // create message list
                Self::format_message(result, &mut result_messages, res_name_list);

                // finally return the response
                OrchestrationProtocolResponse::List(result_messages)
            }
            OrchestrationInstruction::RunSetupScripts(list) => {
                // create futures and get the name list, since the vec is ordered by insertion then the name list will
                // be in the same order
                let mut futures = Vec::new();
                let mut res_name_list = Vec::new();

                for create in list {
                    futures.push(create.get_run_setup_script_future(orchestration_common.clone()));
                    res_name_list.push(create.name());
                }
                // join all futures and collect results
                let result = join_all(futures).await;

                let mut result_messages = Vec::new();
                // create message list
                Self::format_message(result, &mut result_messages, res_name_list);

                // finally return the response
                OrchestrationProtocolResponse::List(result_messages)
            }
            OrchestrationInstruction::Snapshot(cmd) => {

                // get info on all guest images that are libvirt
                let testbed_snapshots = TestbedSnapshots::new(state, orchestration_common).await?;
                match run_snapshot_action(state, &testbed_snapshots, cmd, orchestration_common, logging_send).await {
                    Ok(ok) => OrchestrationProtocolResponse::Generic {
                        is_success: true,
                        message: ok,
                    },
                    Err(err) => OrchestrationProtocolResponse::Generic {
                        is_success: false,
                        message: format!("Snapshot error: {err:#}"),
                    }
                }
            }
            OrchestrationInstruction::TestbedSnapshot { snapshot_guests } => {
                if *snapshot_guests {

                    // create a snapshot of all the guests before continuing
                    let testbed_snapshots = TestbedSnapshots::new(state, orchestration_common).await?;
                    testbed_snapshots.snapshot_all_guests(orchestration_common).await?;
                }
                match run_testbed_snapshot_action(state, orchestration_common).await {
                    Ok(_) => OrchestrationProtocolResponse::Generic {
                        is_success: true,
                        message: "Created Testbed Snapshot".to_string(),
                    },
                    Err(err) => OrchestrationProtocolResponse::Generic {
                        is_success: false,
                        message: format!("Testbed Snapshot error: {err:#}"),
                    }
                }
            }
            OrchestrationInstruction::AnalysisTool(at) => {
                let analysis_tool_res = match at.tool {
                    AnalysisToolsSubCmd::TcpDump { .. } => {
                        packet_capture(at).await
                    }
                };
                match analysis_tool_res {
                    Ok(_) => OrchestrationProtocolResponse::Generic {
                        is_success: true,
                        message: "Analysis tool succeeded".to_string(),
                    },
                    Err(err) => OrchestrationProtocolResponse::Generic {
                        is_success: false,
                        message: format!("Analysis tool error: {err:#}"),
                    },
                }

            }
            OrchestrationInstruction::Exec(exec_cmd) => {

                match prepare_guest_exec_command(&orchestration_common.project_name, exec_cmd, state, orchestration_common, logging_send).await {
                    Ok(_) => OrchestrationProtocolResponse::Generic {
                        is_success: true,
                        message: format!("Exec command {:?} succeeded", exec_cmd.command_type),
                    },
                    Err(err) => OrchestrationProtocolResponse::Generic {
                        is_success: false,
                        message: format!("Exec command {:?} error: {err:#}", exec_cmd.command_type),
                    }
                }

            }
            OrchestrationInstruction::ListCloudImages => {
                let images = OnlineCloudImage::pretty_to_string()?;
                logging_send.send(OrchestrationLogger::info("Available cloud images:".to_string())).await?;
                for img in images {
                    logging_send.send(OrchestrationLogger::info(img)).await?;
                }
                OrchestrationProtocolResponse::Generic {
                    is_success: true,
                    message: "End List Cloud Images".to_string(),
                }
            }
            OrchestrationInstruction::Cancel => {
                OrchestrationProtocolResponse::Generic {
                    is_success: false,
                    message: "Cancel request".to_string(),
                }
            }
            OrchestrationInstruction::End => {
                OrchestrationProtocolResponse::Generic {
                    is_success: true,
                    message: "End Orchestration".to_string(),
                }
            }
        };

        // tell the log listener to finish listening
        logging_send.send(OrchestrationLogger::End).await?;

        Ok(protocol_response)
    }

    /// There are a couple of instructions that all use the same setup image implementation, the only difference is the
    /// list that is given and the desired logging output
    async fn setup_image_helper(
        list: &Vec<OrchestrationResource>,
        orchestration_common: &OrchestrationCommon,
    ) -> OrchestrationProtocolResponse {
        // create futures and get the name list, since the vec is ordered by insertion then the name list will
        // be in the same order
        let mut create_futures = Vec::new();
        let mut res_name_list = Vec::new();

        for create in list {
            create_futures.push(create.get_setup_image_future(orchestration_common.clone()));
            res_name_list.push(create.name());
        }
        // join all futures and collect results
        let result = join_all(create_futures).await;

        let mut result_messages = Vec::new();
        // create message list
        Self::format_message(result, &mut result_messages, res_name_list);

        // finally return the response
        OrchestrationProtocolResponse::List(result_messages)
    }
}

/// Represents one resource that orchestration will action on, based on the wrapping `OrchestrationInstruction` enum
#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum OrchestrationResource {
    Guest(StateTestbedGuest),
    Network(OrchestrationResourceNetworkType),
}

impl OrchestrationResource {
    /// Get the name of the resource for logging
    pub fn name(&self) -> String {
        let mut name = String::new();
        match self {
            OrchestrationResource::Guest(guest) => {
                let guest_type = guest.guest_type.guest_type.name();
                let guest_name = &guest.guest_type.name;
                name.push_str(&format!("{guest_type} guest {guest_name}"));
            }
            OrchestrationResource::Network(network) => {
                match network {
                    OrchestrationResourceNetworkType::Ovn(ovn) => {
                        name.push_str("Ovn ");
                        match ovn {
                            OrchestrationResourceNetwork::Switch(switch) => {
                                name.push_str(&format!("Logical Switch {}", &switch.name))
                            }
                            OrchestrationResourceNetwork::SwitchPort(lsp) => {
                                name.push_str(&format!("Logical Switch Port {}", &lsp.name))
                            }
                            OrchestrationResourceNetwork::Router(router) => {
                                name.push_str(&format!("Logical Router {}", &router.name))
                            }
                            OrchestrationResourceNetwork::RouterPort(lrp) => {
                                name.push_str(&format!("Logical Router Port {}", &lrp.name))
                            }
                            OrchestrationResourceNetwork::OvsPort(ovs) => {
                                name.push_str(&format!("Bridge {} Ovs Port {}", &ovs.integration_bridge_name, &ovs.name))
                            }
                            OrchestrationResourceNetwork::DhcpOption(dhcp) => {
                                name.push_str(&format!("DHCP Option rule (cidr: {} router: {})", &dhcp.cidr.to_string(), &dhcp.router))
                            }
                            OrchestrationResourceNetwork::ExternalGateway(gateway) => {
                                name.push_str(&format!("External Gateway ({:?}, {:?}) on LRP {}", &gateway.router_port_name, &gateway.chassis_name, &gateway.router_port_name))
                            }
                            OrchestrationResourceNetwork::Nat(nat) => {
                                name.push_str(&format!("Nat Rule ({:?}, {:?}, {:?}) on LR {}", &nat.external_ip, &nat.logical_ip, &nat.nat_type, &nat.logical_router_name))
                            }
                            OrchestrationResourceNetwork::Route(route) => {
                                name.push_str(&format!("Static Route ({:?}, {:?}) on LR {}", &route.prefix, &route.next_hop, &route.router_name))
                            }
                            OrchestrationResourceNetwork::ACL(acl) => {
                                name.push_str(&format!("ACL (type: {}, action: {}, match: {}, priority: {}) on {}", &acl.direction, &acl.action, &acl._match, &acl.priority, &acl.entity_name))
                            }
                        }
                    }
                }
            }
        }
        name
    }

    /// Get the future for the create action for the resource
    pub async fn get_create_future(&self, orchestration_common: OrchestrationCommon) -> anyhow::Result<()> {
        match self {
            OrchestrationResource::Guest(guest) => {
                match &guest.guest_type.guest_type {
                    GuestType::Libvirt(libvirt) => {
                        libvirt.create_action(orchestration_common, guest.clone()).await
                    }
                    GuestType::Docker(docker) => {
                        docker.create_action(orchestration_common, guest.clone()).await
                    }
                    GuestType::Android(android) => {
                        android.create_action(orchestration_common, guest.clone()).await
                    }
                }
            }
            OrchestrationResource::Network(network) => {
                match network {
                    OrchestrationResourceNetworkType::Ovn(ovn) => {
                        // we return Ok for all these since we want to keep the return Result<String> for mock testing

                        match ovn {
                            OrchestrationResourceNetwork::Switch(r) => {
                                r.create_command(ovn_run_cmd, (None, orchestration_common.clone())).await?;
                                Ok(())
                            }
                            OrchestrationResourceNetwork::SwitchPort(r) => {
                                r.create_command(ovn_run_cmd, (None, orchestration_common.clone())).await?;
                                Ok(())
                            }
                            OrchestrationResourceNetwork::Router(r) => {
                                r.create_command(ovn_run_cmd, (None, orchestration_common.clone())).await?;
                                Ok(())
                            }
                            OrchestrationResourceNetwork::RouterPort(r) => {
                                r.create_command(ovn_run_cmd, (None, orchestration_common.clone())).await?;
                                Ok(())
                            }
                            OrchestrationResourceNetwork::OvsPort(r) => {
                                r.create_command(
                                    ovn_run_cmd,
                                    (
                                        Some(chassis_to_tb_host(&r.chassis, &orchestration_common)?),
                                        orchestration_common.clone()
                                    )
                                ).await?;
                                Ok(())
                            }
                            OrchestrationResourceNetwork::DhcpOption(r) => {
                                r.create_command(ovn_run_cmd, (None, orchestration_common.clone())).await?;
                                Ok(())
                            }
                            OrchestrationResourceNetwork::ExternalGateway(r) => {
                                r.create_command(ovn_run_cmd, (None, orchestration_common.clone())).await?;
                                Ok(())
                            }
                            OrchestrationResourceNetwork::Nat(r) => {
                                r.create_command(ovn_run_cmd, (None, orchestration_common.clone())).await?;
                                Ok(())
                            }
                            OrchestrationResourceNetwork::Route(r) => {
                                r.create_command(ovn_run_cmd, (None, orchestration_common.clone())).await?;
                                Ok(())
                            }
                            OrchestrationResourceNetwork::ACL(r) => {
                                r.create_command(ovn_run_cmd, (None, orchestration_common.clone())).await?;
                                Ok(())
                            }
                        }
                    }
                }
            }
        }
    }

    /// Get the future for the destroy action for the resource
    pub async fn get_destroy_future(&self, orchestration_common: OrchestrationCommon) -> anyhow::Result<()> {
        match self {
            OrchestrationResource::Guest(guest) => {
                match &guest.guest_type.guest_type {
                    GuestType::Libvirt(libvirt) => {
                        libvirt.destroy_action(orchestration_common, guest.clone()).await
                    }
                    GuestType::Docker(docker) => {
                        docker.destroy_action(orchestration_common, guest.clone()).await
                    }
                    GuestType::Android(android) => {
                        android.destroy_action(orchestration_common, guest.clone()).await
                    }
                }
            }
            OrchestrationResource::Network(network) => {
                match network {
                    OrchestrationResourceNetworkType::Ovn(ovn) => {
                        // we return Ok for all these since we want to keep the return Result<String> for mock testing

                        match ovn {
                            OrchestrationResourceNetwork::Switch(r) => {
                                r.destroy_command(ovn_run_cmd_allow_fail, (None, orchestration_common.clone())).await?;
                                Ok(())
                            }
                            OrchestrationResourceNetwork::SwitchPort(r) => {
                                r.destroy_command(ovn_run_cmd_allow_fail, (None, orchestration_common.clone())).await?;
                                Ok(())
                            }
                            OrchestrationResourceNetwork::Router(r) => {
                                r.destroy_command(ovn_run_cmd_allow_fail, (None, orchestration_common.clone())).await?;
                                Ok(())
                            }
                            OrchestrationResourceNetwork::RouterPort(r) => {
                                r.destroy_command(ovn_run_cmd_allow_fail, (None, orchestration_common.clone())).await?;
                                Ok(())
                            }
                            OrchestrationResourceNetwork::OvsPort(r) => {
                                r.destroy_command(
                                    ovn_run_cmd_allow_fail,
                                    (
                                        Some(chassis_to_tb_host(&r.chassis, &orchestration_common)?),
                                        orchestration_common.clone()
                                    )
                                ).await?;
                                Ok(())
                            }
                            OrchestrationResourceNetwork::DhcpOption(r) => {
                                r.destroy_command(ovn_run_cmd_allow_fail, (None, orchestration_common.clone())).await?;
                                Ok(())
                            }
                            OrchestrationResourceNetwork::ExternalGateway(r) => {
                                r.destroy_command(ovn_run_cmd_allow_fail, (None, orchestration_common.clone())).await?;
                                Ok(())
                            }
                            OrchestrationResourceNetwork::Nat(r) => {
                                r.destroy_command(ovn_run_cmd_allow_fail, (None, orchestration_common.clone())).await?;
                                Ok(())
                            }
                            OrchestrationResourceNetwork::Route(r) => {
                                r.destroy_command(ovn_run_cmd_allow_fail, (None, orchestration_common.clone())).await?;
                                Ok(())
                            }
                            OrchestrationResourceNetwork::ACL(r) => {
                                r.destroy_command(ovn_run_cmd_allow_fail, (None, orchestration_common.clone())).await?;
                                Ok(())
                            }
                        }
                    }
                }
            }
        }
    }

    pub async fn get_push_image_future(&self, orchestration_common: OrchestrationCommon) -> anyhow::Result<()> {
        match self {
            OrchestrationResource::Guest(g) => {
                match &g.guest_type.guest_type {
                    GuestType::Libvirt(l) => {
                        l.push_image_action(orchestration_common, g.clone()).await
                    }
                    GuestType::Docker(d) => {
                        d.push_image_action(orchestration_common, g.clone()).await
                    }
                    GuestType::Android(_) => unreachable!()
                }
            }
            OrchestrationResource::Network(_) => unreachable!()
        }
    }

    pub async fn get_setup_image_future(&self, orchestration_common: OrchestrationCommon) -> anyhow::Result<()> {
        match self {
            OrchestrationResource::Guest(g) => {
                match &g.guest_type.guest_type {
                    GuestType::Libvirt(l) => {
                        l.setup_image_action(orchestration_common, g.clone()).await
                    }
                    GuestType::Docker(_) => unreachable!(),
                    GuestType::Android(_) => unreachable!()
                }
            }
            OrchestrationResource::Network(_) => unreachable!(),
        }
    }

    pub async fn get_rebase_clone_future(&self, orchestration_common: OrchestrationCommon, state: &State) -> anyhow::Result<()> {
        match self {
            OrchestrationResource::Guest(g) => {
                match &g.guest_type.guest_type {
                    GuestType::Libvirt(l) => {
                        l.rebase_image_action(orchestration_common, g.clone(), state.testbed_guests.clone()).await
                    }
                    GuestType::Docker(_) => unreachable!(),
                    GuestType::Android(_) => unreachable!()
                }
            }
            OrchestrationResource::Network(_) => unreachable!(),
        }
    }

    pub async fn get_run_setup_script_future(&self, orchestration_common: OrchestrationCommon) -> anyhow::Result<()> {
        match self {
            OrchestrationResource::Guest(g) => {
                match &g.guest_type.guest_type {
                    GuestType::Libvirt(l) => {
                        l.setup_action(orchestration_common, g.clone()).await
                    }
                    GuestType::Docker(_) => unreachable!(),
                    GuestType::Android(_) => unreachable!()
                }
            }
            OrchestrationResource::Network(_) => unreachable!(),
        }
    }

}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum OrchestrationResourceNetworkType {
    Ovn(OrchestrationResourceNetwork),
    // Ovs,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum OrchestrationResourceNetwork {
    Switch(LogicalSwitch),
    SwitchPort(LogicalSwitchPort),
    Router(LogicalRouter),
    RouterPort(LogicalRouterPort),
    OvsPort(OvsPort),
    DhcpOption(DhcpDatabaseEntry),
    ExternalGateway(OvnExternalGateway),
    Nat(OvnNat),
    Route(OvnRoute),
    ACL(LogicalACLRecord),
}

/// This enum is to be sent from the server back to the client as a response to the result of `OrchestrationProtocol`,
/// so that we can handle the logging to the user and determine if the orchestration can continue
#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum OrchestrationProtocolResponse {
    /// Generic response with either success or fail
    Generic {
        is_success: bool,
        message: String,
    },
    /// Response for a single instruction
    Single(OrchestrationInstructionResultMessage),
    /// Response for a list of instructions
    List(Vec<OrchestrationInstructionResultMessage>),
}

/// This enum is used during command running to send useful logging messages back to the client,
/// as the `OrchestrationProtocolResponse` is only used to send messages related to instructions
#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum OrchestrationLogger {
    Log {
        message: String,
        level: OrchestrationLoggerLevel,
    },
    End,
}

impl OrchestrationLogger {
    pub fn info(message: String) -> Self {
        Self::Log { message, level: OrchestrationLoggerLevel::Info }
    }
    pub fn error(message: String) -> Self {
        Self::Log { message, level: OrchestrationLoggerLevel::Error }
    }
}

/// This dictates the log level of the `OrchestrationLogger` for formatting purposes
#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum OrchestrationLoggerLevel {
    Info,
    Error,
}

impl OrchestrationProtocolResponse {

    /// Get the result message based on the enum variant. For the Responses that are batched, need to give a combined
    /// message for all the commands.
    pub fn get_result_messages(&self) -> anyhow::Result<ResultMessages> {
        match self {
            OrchestrationProtocolResponse::Generic { is_success, message } => {
                if *is_success {
                    Ok(ResultMessages {
                        success_message: Some(vec![message.to_string()]),
                        fail_message: None
                    })
                } else {
                    Ok(ResultMessages {
                        success_message: None,
                        fail_message: Some(vec![message.to_string()])
                    })
                }
            }
            OrchestrationProtocolResponse::Single(single) => {
                let mut messages = ResultMessages::default();
                format_response_message(&mut messages, &[single.clone()]);
                Ok(messages)
            }
            OrchestrationProtocolResponse::List(list) => {
                let mut messages = ResultMessages::default();
                format_response_message(&mut messages, list);
                Ok(messages)
            }
        }
    }

    pub fn is_success(&self) -> anyhow::Result<bool> {
        match self {
            OrchestrationProtocolResponse::Single(r) => Ok(r.is_success),
            OrchestrationProtocolResponse::List(r) => {
                Ok(Self::status_from_batch(r))
            }
            OrchestrationProtocolResponse::Generic { is_success, .. } => Ok(*is_success),
        }
    }

    fn status_from_batch(r: &[OrchestrationInstructionResultMessage]) -> bool {
        let any_failed = r
            .iter()
            .any(|res| !res.is_success );
        !any_failed
    }
}

#[derive(Default)]
pub struct ResultMessages {
    pub success_message: Option<Vec<String>>,
    pub fail_message: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OrchestrationInstructionResultMessage {
    is_success: bool,
    message: String,
}

fn format_instruction_message(
    instruction: &mut String,
    items: &[OrchestrationResource],
) {
    if items.is_empty() {
        instruction.push_str("[]");
    } else if items.len() > 1 {
        instruction.push_str("[ ");
        instruction.push_str(&items[0].name());
        for res in items.iter().skip(1) {
            instruction.push_str(", ");
            instruction.push_str(&res.name());
        }
        instruction.push_str(" ]");
    } else {
        instruction.push_str(&items[0].name());
    }
}

fn format_response_message(
    messages: &mut ResultMessages,
    items: &[OrchestrationInstructionResultMessage],
) {
    let mut success_message = Vec::new();
    let mut fail_message = Vec::new();
    if !items.is_empty() {
        if items[0].is_success {
            success_message.push(items[0].message.to_string());
        } else {
            fail_message.push(items[0].message.to_string());
        }
        for res in items.iter().skip(1) {
            if res.is_success {
                success_message.push(res.message.to_string());
            } else {
                fail_message.push(res.message.to_string());
            }
        }
    }
    if !success_message.is_empty() {
        // success_message = format!("[ {success_message} ]");
        messages.success_message = Some(success_message);
    }
    if !fail_message.is_empty() {
        // fail_message = format!("[ {fail_message} ]");
        messages.fail_message = Some(fail_message);
    }
}
