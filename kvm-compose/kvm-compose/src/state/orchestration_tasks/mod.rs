use std::path::{Path, PathBuf};
use anyhow::{bail, Context};
use async_trait::async_trait;
use futures_util::future::try_join_all;
use nix::unistd::{Gid, Uid};
use tokio::sync::mpsc::Sender;
use kvm_compose_schemas::kvm_compose_yaml::machines::GuestType;
use kvm_compose_schemas::kvm_compose_yaml::machines::libvirt::LibvirtGuestOptions;
use kvm_compose_schemas::settings::TestbedClusterConfig;
use crate::components::helpers::serialisation;
use crate::components::helpers::xml::render_libvirt_network_xml;
use crate::components::LogicalTestbed;
use crate::get_project_folder_user_group;
use crate::orchestration::*;
use crate::orchestration::api::{OrchestrationInstruction, OrchestrationLogger, OrchestrationProtocol};
use crate::orchestration::websocket::send_orchestration_instruction_over_channel;
use crate::state::orchestration_tasks::stages::*;
use crate::state::schema::{State, StateTestbedGuestList};

pub mod ovs_network;
pub mod ovn_network;
pub mod guests;
mod stages;
pub mod generate_artefacts;


pub async fn get_orchestration_common(
    state: &State,
    force_provisioning: bool,
    force_rerun_scripts: bool,
    reapply_acl: bool,
    kvm_compose_config: TestbedClusterConfig,
) -> anyhow::Result<OrchestrationCommon> {
    // permissions
    let user_group = get_project_folder_user_group().await?;
    Ok(OrchestrationCommon {
        testbed_hosts: state.testbed_hosts.0.clone(),
        testbed_guest_shared_config: state.testbed_guest_shared_config.clone(),
        project_name: state.project_name.clone(),
        project_working_dir: state.project_working_dir.clone(),
        force_provisioning,
        force_rerun_scripts,
        reapply_acl,
        kvm_compose_config,
        network: state.network.clone(),
        fs_user: user_group.0.as_raw(),
        fs_group: user_group.1.as_raw(),
        ..Default::default()
    })
}

/// This is the implementation for the `OrchestrationTask` trait for the `State` which is the
/// representation of the testbed. The `State` is the definitive description of the testbed, post
/// calculation from the `LogicalTestbed`. This is an important step for up and especially down.
/// Therefore what is in the `State` should be enough to deploy a testbed, and for down given the
/// same underlying environment. Since `LogicalTestbed` is dependant on the given underlying
/// environment, it is not guaranteed that two `LogicalTestbed` constructions are the same.
/// Therefore, in the case of destroying deployed components, it is important to use the data saved
/// in `State` rather than re-calculating `LogicalTestbed` then using that `State`.
#[async_trait]
impl OrchestrationTask for State {
    async fn create_action(&self, _common: &OrchestrationCommon) -> anyhow::Result<()> {
        // This create action function is not used for state anymore due to changes in the way
        // the testbed deploys, now using the request_create_action which allows for the communication
        // of logging back to the caller client.
        // Realistically, this OrchestrationTask trait abstraction has not been totally used, it has
        // only also been implemented for StateNetwork, so we should re-evaluate if we still need
        // it or at least in this form i.e. with the redundant functions - this is due to changes
        // in the testbed architecture moving to this client server model with bi-directional
        // communication. We may need this trait for future implementations i.e. provisioning
        // network interfaces on the host etc.
        unimplemented!();
    }

    async fn destroy_action(&self, common: &OrchestrationCommon, logging_send: &Sender<OrchestrationLogger>) -> anyhow::Result<()> {
        // TODO this is only used in the clear artefacts function, which could be changed to use
        //  request_destroy_action - needs testing, as this also sends data back to client ..
        //  see above create_action for more details
        tracing::info!("running destroy action for testbed State");

        check_if_testbed_hosts_up(&common).await?;

        // destroy guests
        let mut guest_destroy_futures = Vec::new();
        for (_guest_name, guest_data) in self.testbed_guests.0.iter() {
            match &guest_data.guest_type.guest_type {
                GuestType::Libvirt(libvirt) => {
                    guest_destroy_futures.push(libvirt.destroy_action(common.clone(), guest_data.clone(), logging_send));
                }
                GuestType::Docker(docker) => {
                    guest_destroy_futures.push(docker.destroy_action(common.clone(), guest_data.clone(), logging_send));
                }
                GuestType::Android(android) => {
                    guest_destroy_futures.push(android.destroy_action(common.clone(), guest_data.clone(), logging_send));
                }
            }
        }
        try_join_all(guest_destroy_futures).await?;

        // TODO - destroy all non testbed network interfaces

        // make sure temporary network is down, in case the up command failed
        turn_off_temporary_network(
            &common.project_name,
            &common.project_working_dir.to_str().context("getting project path")?.to_string()
        ).await?;

        self.network.destroy_action(common, logging_send).await?;

        Ok(())
    }

    async fn request_create_action(&self, common: &OrchestrationCommon, sender: &mut Sender<OrchestrationProtocol>) -> anyhow::Result<()> {
        tracing::info!("running request create action for testbed State");

        // do preflight checkups from the server i.e. make sure testbed hosts are up, folder structure exists
        send_orchestration_instruction_over_channel(
            sender,
            // receiver,
            OrchestrationInstruction::TestbedHostCheck,
        ).await.context("requesting if testbed hosts are up")?;

        send_orchestration_instruction_over_channel(
            sender,
            // receiver,
            OrchestrationInstruction::Setup,
        ).await.context("requesting setup orchestration instruction")?;

        // do all network components
        self.network.request_create_action(common, sender)
            .await
            .context("requesting network components to be created")?;

        if !self.state_provisioning.guests_provisioned || common.force_provisioning {
            tracing::info!("Stage: setting up any libvirt backing image guests");
            let net_provision = check_provision_temporary_network(&self.testbed_guests);
            if net_provision {
                tracing::info!("turning on temporary network for backing images with shared setup scripts");
                send_orchestration_instruction_over_channel(
                    sender,
                    // receiver,
                    OrchestrationInstruction::CreateTempNetwork(common.clone()),
                ).await.context("requesting create temporary network instruction")?;
            }

            setup_backing_image_stage(&self, sender).await?;

            if net_provision {
                tracing::info!("turning on temporary network for backing images with shared setup scripts");
                send_orchestration_instruction_over_channel(
                    sender,
                    // receiver,
                    OrchestrationInstruction::DestroyTempNetwork(common.clone()),
                ).await.context("requesting destroy temporary network instruction")?;
            }

            tracing::info!("Stage: creating any libvirt clones of backing image guests");
            setup_linked_clones_stage(&self, sender).await?;
        } else {
            tracing::info!("skipping setting up backing image and creating clones as they have already been provisioned");
        }

        // if already been provisioned previously, this will only check if the images exist on the
        // remote
        tracing::info!("Stage: pushing guest images to remote testbed hosts");
        push_guest_images_stage(&self, sender).await?;
        push_backing_guest_images_stage(&self, common, sender).await?;

        tracing::info!("Stage: rebasing clones on remote testbed hosts");
        rebase_clone_images_stage(&self, common, sender).await?;

        tracing::info!("Stage: deploying guests");
        deploy_guest_stage(&self, sender).await?;

        if !self.state_provisioning.guests_provisioned || common.force_rerun_scripts {
            tracing::info!("Stage: running any guest setup scripts");
            run_guest_setup_scripts_stage(&self, sender).await?;
        } else {
            tracing::info!("Skipping guest setup scripts as guest have already been provisioned");
        }

        // run scripts should always be run on the guest
        tracing::info!("Stage: running any guest run scripts");
        run_guest_run_scripts_stage(&self, sender).await?;

        Ok(())
    }

    async fn request_destroy_action(&self, common: &OrchestrationCommon, sender: &mut Sender<OrchestrationProtocol>) -> anyhow::Result<()> {

        // check if testbed hosts are up
        send_orchestration_instruction_over_channel(
            sender,
            // receiver,
            OrchestrationInstruction::TestbedHostCheck,
        ).await.context("requesting if testbed hosts are up")?;

        // destroy guests
        destroy_guest_stage(&self, sender).await?;

        // make sure temporary network is down
        send_orchestration_instruction_over_channel(
            sender,
            // receiver,
            OrchestrationInstruction::DestroyTempNetwork(common.clone()),
        ).await.context("requesting destroy temporary network instruction")?;

        // destroy network components
        self.network.request_destroy_action(common, sender)
            .await
            .context("requesting network components to be created")?;

        Ok(())
    }
}

/// Dedicated task to clear artefacts on both the main, and remote testbed hosts if any. Will
/// check if guests are running first before clearing artefacts.
pub async fn clear_artefacts(
    state: &State,
    common: &OrchestrationCommon,
    logging_send: &Sender<OrchestrationLogger>,
) -> anyhow::Result<()> {
    let project_name = &common.project_name;
    // make sure testbed is down
    state.destroy_action(&common, logging_send).await?;
    // special case for android
    for (guest_name, guest_data) in state.testbed_guests.0.iter() {
        match &guest_data.guest_type.guest_type {
            GuestType::Android(android_config) => {
                // is android, run avd delete command
                let avd_name = format!("{project_name}-{guest_name}");
                if android_config.scaling.is_some() {
                    // skip the scaling reference definition, no avd was created
                    continue;
                }
                tracing::info!("deleting avd {avd_name}");
                let cmd = vec![
                    "/opt/android-sdk/cmdline-tools/latest/bin/avdmanager", "delete", "avd",
                    "-n", &avd_name
                ];
                run_testbed_orchestration_command_allow_fail(
                    &common,
                    &guest_data.testbed_host.as_ref().unwrap(),
                    "sudo",
                    cmd,
                    false,
                    None,
                ).await?;
            }
            _ => {}
        }
    }
    // remove remote folder
    tracing::info!("deleting remote artefacts folders");
    destroy_remote_project_folders(&common).await?;
    // remove local folder
    tracing::info!("deleting local artefacts folder");
    let mut artefacts_folder = common.project_working_dir.clone();
    artefacts_folder.push("artefacts");
    if Path::new(&artefacts_folder).is_dir() {
        tokio::fs::remove_dir_all(artefacts_folder).await?;
        // fs::remove_file(format!("{}-state.json", project_name))?;
    }
    Ok(())
}

/// Function to check if the guest images already exist given a logical testbed, to determine
/// if we need to run guest scripts. If one or more images for the deployment defined by the yaml
/// file and therefore defined in the logical testbed, then we will assume that this is a deployment
/// that has been shared. We will take the cautious approach and prevent overwriting state for the
/// images and point the user to the --provision flag for the up command if they do face the
/// scenario where some guest images are missing.
pub fn check_if_guest_images_exist(
    logical_testbed: &LogicalTestbed,
) -> anyhow::Result<bool> {
    for guest in logical_testbed.logical_guests.iter() {
        let machine = guest.get_machine_definition();
        match machine.guest_type {
            GuestType::Libvirt(libvirt) => {
                let image_path = match libvirt.libvirt_type {
                    LibvirtGuestOptions::CloudImage { path, .. } => path,
                    LibvirtGuestOptions::ExistingDisk { path, .. } => Some(path),
                    LibvirtGuestOptions::IsoGuest { path, .. } => Some(path),
                };
                if let Some(path) = image_path {
                    if path.exists() {
                        // we found one image that already exists, exit early
                        return Ok(true)
                    }
                }
            }
            GuestType::Docker(_) => {}
            GuestType::Android(_) => {}
        }
    }
    // no images found
    Ok(false)
}

pub async fn check_if_testbed_hosts_up(
    common: &OrchestrationCommon,
) -> anyhow::Result<()> {
    for (name, host) in common.testbed_hosts.iter() {
        if host.is_main_host {
            continue;
        }
        tracing::info!("checking if testbed host {} is up", name);
        let res = run_subprocess_command(
            "curl",
            vec!["-s", &format!("http://{}:3355/api/config/status", host.ip)],
            false,
            None,
        ).await;
        match res {
            Ok(ok) => {
                if ok.contains("Failed to connect to") {
                    bail!("could not reach client testbed host {name}, is the client testbed server running?")
                }
            }
            Err(err) => {
                bail!("could not reach client testbed host {name} with err {err:#}");
            }
        }
    }
    Ok(())
}

/// For each libvirt guest in the guests definition, we need to find if any backing image guests
/// have a shared setup script. If there is a shared setup script then we count this guest. If there
/// are any guests with a shared setup script then we will want to provision a temporary network
/// for them.
fn check_provision_temporary_network(state_testbed_guest_list: &StateTestbedGuestList) -> bool {
    // if there are backing images with a setup script, we need to provision a temporary
    // network
    let provision_temporary_network = state_testbed_guest_list.0
        .iter()
        .filter(|(_, g)| {
            match &g.guest_type.guest_type {
                GuestType::Libvirt(libvirt) => {
                    if let Some(scaling) = &libvirt.scaling {
                        scaling.shared_setup.is_some() // could be true or false
                    } else {
                        false // no scaling, ignore
                    }
                }
                _ => false, // not libvirt, ignore
            }
        })
        .count();
    if provision_temporary_network > 0 {
        true
    } else {
        false
    }
}

/// Create a network with Virsh to place the backing image libvirt guests.
pub async fn turn_on_temporary_network(
    project_name: &String,
    project_folder: &String,
    orchestration_common: &OrchestrationCommon,
) -> anyhow::Result<()> {
    // generate the temporary network xml
    let mut tera_context = tera::Context::new();
    tera_context.insert("network_name", &format!("{project_name}-testbedos"));
    tera_context.insert("bridge_name", &format!("temp-net-tbos")); // TODO - do we make this unique?
    let xml = render_libvirt_network_xml(tera_context)?;
    // write to artefacts folder
    let xml_dest = format!(
        "{}/artefacts/temporary-network.xml",
        &project_folder,
    );
    tracing::info!("xml_dest = {xml_dest}");
    serialisation::write_file_with_permissions(
        xml_dest.clone(),
        xml.to_string(),
        0o755,
        Uid::from_raw(orchestration_common.fs_user),
        Gid::from_raw(orchestration_common.fs_group),
    ).await?;
    // create network
    let cmd = vec![
        "virsh", "net-create", &xml_dest,
    ];
    run_subprocess_command_allow_fail(
        "sudo",
        cmd,
        false,
        None,
    ).await?;
    Ok(())
}

pub async fn turn_off_temporary_network(
    project_name: &String,
    project_folder: &String,
) -> anyhow::Result<()> {
    let network_name = format!("{project_name}-testbedos");
    let xml_dest = format!(
        "{}/artefacts/temporary-network.xml",
        &project_folder,
    );
    // destroy network
    let cmd = vec![
        "virsh", "net-destroy", &network_name,
    ];
    run_subprocess_command_allow_fail(
        "sudo",
        cmd,
        false,
        None,
    ).await?;
    // delete the temporary xml
    let res = tokio::fs::remove_file(&xml_dest).await;
    match res {
        Ok(_) => {}
        Err(_) => tracing::debug!("temporary network file already deleted"),
    }
    Ok(())
}

// ///
// pub async fn make_orchestration_request(
//     client: &reqwest::Client,
//     api_endpoint: String,
//     orchestration_resource: &OrchestrationResource,
//     orchestration_common: &OrchestrationCommon,
// ) -> anyhow::Result<()> {
//
//     let component_name = orchestration_resource.name();
//     tracing::info!("making request to create component {component_name}");
//
//     // we can use json macro or OrchestrationProtocol, either is fine
//     let json = json!({
//         "common": orchestration_common,
//         "resource": orchestration_resource,
//     });
//
//     let result = client
//         .post(api_endpoint)
//         .json(&json)
//         .send()
//         .await
//         .context("sending OrchestrationResource to server")?;
//
//     result
//         .error_for_status()
//         .context("checking http code for orchestration request")?;
//
//
//     Ok(())
// }

/// Take in a path, if the path is an absolute path, then return as is, otherwise append the project
/// directory as we will assume the path of the file specified in the yaml is relative to the
/// project root.
pub fn parse_path_with_deployment_config(
    path: &PathBuf,
    common: &OrchestrationCommon,
) -> anyhow::Result<PathBuf> {

    let path: &Path = &path;

    let absolute_path = if path.is_absolute() {
        path.canonicalize()?
    } else {
        common.project_working_dir.join(path).canonicalize()?
    };

    Ok(absolute_path)
}
