use std::collections::HashSet;
use std::path::PathBuf;
use std::time::Duration;
use anyhow::{anyhow, bail, Context};
use async_trait::async_trait;
use futures_util::future::{try_join_all};
use glob::{glob};
use nix::unistd::{Gid, Uid};
use kvm_compose_schemas::kvm_compose_yaml::machines::avd::ConfigAVDMachine;
use kvm_compose_schemas::kvm_compose_yaml::machines::docker::ConfigDockerMachine;
use kvm_compose_schemas::kvm_compose_yaml::machines::GuestType;
use kvm_compose_schemas::kvm_compose_yaml::machines::libvirt::{ConfigLibvirtMachine, LibvirtGuestOptions};
use crate::components::get_guest_interface_name;
use crate::orchestration::{is_master, OrchestrationCommon, OrchestrationGuestTask, run_testbed_orchestration_command, run_testbed_orchestration_command_allow_fail};
use crate::orchestration::ssh::SSHClient;
use crate::ovn::components::logical_switch_port::LogicalSwitchPortType;
use crate::state::{State, StateNetwork, StateTestbedGuest, StateTestbedGuestList};


#[async_trait]
impl OrchestrationGuestTask for ConfigLibvirtMachine {
    async fn setup_image_action(&self, common: OrchestrationCommon, machine_config: StateTestbedGuest) -> anyhow::Result<()> {
        // we need to make a distinction between a backing image, a clone of a backing image and
        // a normal libvirt image that should be deployed

        let guest_name = format!("{}-{}", &common.project_name, &machine_config.guest_type.name);

        if machine_config.is_golden_image {
            // if the clone has a shared_setup script, then boot, run script then turn it off
            // otherwise just create the image
            // the backing image guest definition will have a scaling option
            if let Some(shared_setup) = self.scaling.clone().context("getting scaling config for libvirt guest")?.shared_setup {
                tracing::info!("setting up backing image guest {} as it has a shared setup script", &guest_name);
                // there is a shared setup script
                // artefact generation has already created a domain xml with the interface set to
                // the libvirt bridge created prior to this stage, so we can just deploy the guest
                // then run the script depending on the guest options
                tracing::info!("starting base backing image guest {guest_name} to run share setup script");
                match self.libvirt_type {
                    LibvirtGuestOptions::CloudImage { .. } => {
                        // cloud images will have the testbed guest public key pushed so we can
                        // use ssh
                        self.create_action(common.clone(), machine_config.clone()).await?;
                        // wait for guest to be up
                        tracing::info!("waiting for guest {guest_name} to start");
                        // TODO - observed the guest's ssh server may not start, why? and cant restart manually
                        wait_for_guest_to_be_up(&common, &machine_config, vec!["ls"]).await?;
                        tracing::info!("running shared setup script on guest {guest_name}");
                        let shared_setup_script = shared_setup.to_str().context("getting shared setup script path")?;
                        let path_to_shared_setup_script = format!("{}/{}", &common.project_working_dir.to_str().context("getting project working dir")?, shared_setup_script);
                        SSHClient::push_file_to_guest(
                            &common,
                            &path_to_shared_setup_script,
                            &"/tmp".to_string(),
                            self.username.as_ref().unwrap(),
                            &guest_name,
                        ).await?;
                        SSHClient::run_guest_command(
                            &common,
                            vec!["sudo", "bash", format!("/tmp/{shared_setup_script}").as_str()],
                            &machine_config,
                            false,
                        ).await?;
                        // turn off guest
                        tracing::info!("turning off guest {guest_name} now shared setup script has finished running");
                        self.destroy_action(common.clone(), machine_config).await?;
                    }
                    LibvirtGuestOptions::ExistingDisk { .. } => unimplemented!(),
                    LibvirtGuestOptions::IsoGuest { .. } => unimplemented!(),
                }
            } else {
                tracing::info!("backing image guest {} has no shared setup script", &guest_name);
            }

        } else if self.is_clone_of.is_some() {
            // is a clone of a backing image, create the clone
            let backing_image_clone = self.is_clone_of.as_ref().unwrap();
            tracing::info!("creating clone guest {} from {}", &machine_config.guest_type.name, backing_image_clone);

            let master_testbed = get_master_testbed_name(&common);

            let project_folder = common.project_working_dir.to_str().unwrap();
            let guest_name_no_project = &machine_config.guest_type.name;
            let clone_image_location = format!(
                "{project_folder}/artefacts/{guest_name_no_project}-linked-clone.qcow2");
            let backing_image_guest_folder = format!("{project_folder}/artefacts/");
            // need to find the image for this guest, it could be cloud init, existing disk, iso guest etc
            // TODO - when the backing image is not .img, might be .qcow2
            let backing_image_location = {
                let mut res = Err(anyhow!("could not find image"));
                for entry in glob(&format!("{backing_image_guest_folder}/{backing_image_clone}-*.img")).context("Fail to glob for the backing image disk")? {
                    match entry {
                        Ok(path) => {
                            res = Ok(path);
                            break;
                        }
                        Err(_) => {}
                    }
                }
                res
            }?;

            let cmd = vec!["qemu-img", "create", "-f", "qcow2", "-b", backing_image_location.to_str().unwrap(), "-F", "qcow2", &clone_image_location];
            let clone_create_res = run_testbed_orchestration_command(
                &common,
                &master_testbed,
                "sudo",
                cmd,
                false,
                None,
            ).await;
            match clone_create_res {
                Ok(_) => {}
                Err(err) => {
                    // dont fail if the clone is already running
                    let expected_err = "Is another process using the image".to_string();
                    return if !err.to_string().trim().contains(&expected_err) {
                        // error in joining blocking thread
                        Err(err)
                    } else {
                        tracing::warn!("the clone is already running, cannot rebase, continuing...");
                        Ok(())
                    }
                }
            }
            // set image user:group
            nix::unistd::chown(clone_image_location.as_str(), Some(Uid::from_raw(common.fs_user)), Some(Gid::from_raw(common.fs_group)))?;

        } else {
            // non clone libvirt images are prepped in artefact generation before this is run, so
            // nothing to do for these at this time

        }
        Ok(())
    }

    async fn push_image_action(&self, common: OrchestrationCommon, machine_config: StateTestbedGuest) -> anyhow::Result<()> {

        let mut futures = Vec::new();

        // check if the guest is on master or not
        let testbed_host = machine_config.testbed_host.as_ref().unwrap();
        if !is_master(&common, testbed_host) {
            // not on master, must push files

            // get image
            let project_path = common.project_working_dir.to_str().unwrap().to_string();
            let local_image_folder_path = get_local_image_folder_path(&project_path);
            // get xml
            let local_xml_path = get_xml_path(&local_image_folder_path, &machine_config);

            tracing::info!("pushing artefacts for guest {} to {}", &machine_config.guest_type.name, &testbed_host);

            // different actions for the libvirt types
            match &self.libvirt_type {
                LibvirtGuestOptions::CloudImage { path, .. } => {
                    // send image
                    let image_name = if self.is_clone_of.is_some() {
                        format!("{}-linked-clone.qcow2", machine_config.guest_type.name)
                    } else {
                        format!("{}-cloud-disk.img", machine_config.guest_type.name)
                    };
                    let local_image_path = format!("{local_image_folder_path}/{image_name}");
                    let remote_dst = path.as_ref().unwrap().parent().unwrap().to_str().unwrap();
                    futures.push(SSHClient::push_file_to_remote_testbed(
                        &common,
                        testbed_host.clone(),
                        local_image_path,
                        remote_dst.to_string(),
                        false,
                    ));
                    // send xml
                    futures.push(SSHClient::push_file_to_remote_testbed(
                        &common,
                        testbed_host.clone(),
                        local_xml_path,
                        remote_dst.to_string(),
                        false,
                    ));
                    // push cloud init iso
                    let iso_name = if self.is_clone_of.is_some() {
                        format!("{}-linked-clone.iso", machine_config.guest_type.name)
                    } else {
                        format!("{}-cloud-init.iso", machine_config.guest_type.name)
                    };
                    let local_image_path = format!("{local_image_folder_path}/{iso_name}");
                    futures.push(SSHClient::push_file_to_remote_testbed(
                        &common,
                        testbed_host.clone(),
                        local_image_path,
                        remote_dst.to_string(),
                        false,
                    ));

                }
                LibvirtGuestOptions::ExistingDisk { path, .. } => {
                    // get file name with extension
                    let image_name = path.file_name().unwrap().to_str().unwrap();
                    let local_image_path = format!("{local_image_folder_path}/{image_name}");
                    let remote_dst = path.parent().unwrap().to_str().unwrap();
                    // send image
                    futures.push(SSHClient::push_file_to_remote_testbed(
                        &common,
                        testbed_host.clone(),
                        local_image_path,
                        remote_dst.to_string(),
                        false,
                    ));
                    // send xml
                    futures.push(SSHClient::push_file_to_remote_testbed(
                        &common,
                        testbed_host.clone(),
                        local_xml_path,
                        remote_dst.to_string(),
                        false,
                    ));
                }
                LibvirtGuestOptions::IsoGuest { .. } => unimplemented!(),
            }
        }
        let _ = try_join_all(futures).await?;

        Ok(())
    }

    async fn pull_image_action(&self, common: OrchestrationCommon, machine_config: StateTestbedGuest) -> anyhow::Result<()> {
        let target_testbed = machine_config.testbed_host.as_ref().unwrap();
        let guest_name = machine_config.guest_type.name.to_string();
        if is_master(&common, target_testbed) {
            // clone is local, dont rebase
            return Ok(());
        } else {
            tracing::info!("pulling guest {} image from testbed host {} to master", &guest_name, target_testbed);
            let remote_image_path = match &self.libvirt_type {
                LibvirtGuestOptions::CloudImage { path, .. } => path.as_ref().unwrap(),
                LibvirtGuestOptions::ExistingDisk { path, .. } => path,
                LibvirtGuestOptions::IsoGuest { path, .. } => path,
            };
            let local_dest = format!("{}/artefacts/", &common.project_working_dir.to_str().unwrap());
            SSHClient::pull_file_from_remote_testbed(
                &common,
                target_testbed,
                local_dest,
                remote_image_path.to_str().unwrap().to_string(),
                false
            ).await?;
            // can leave the iso and domain xml as they should not have changed
            Ok(())
        }
    }

    async fn rebase_image_action(&self, common: OrchestrationCommon, machine_config: StateTestbedGuest, guest_list: StateTestbedGuestList) -> anyhow::Result<()> {
        tracing::info!("rebasing image for guest {}", &machine_config.guest_type.name);
        let target_testbed = machine_config.testbed_host.as_ref().unwrap();
        if is_master(&common, target_testbed) {
            // clone is local, dont rebase
            return Ok(());
        }
        // need to work out path to the backing disk on the remote testbed for this
        // particular rebase
        let backing_guest_name = self.is_clone_of.as_ref().unwrap();
        let remote_backing_image_path = get_backing_image_remote_path(
            &common,
            &guest_list,
            backing_guest_name,
            target_testbed)?;
        // get the remote path to the clone we want to rebase
        let clone_remote_path = match &self.libvirt_type {
            LibvirtGuestOptions::CloudImage { path, .. } => path.as_ref().unwrap().to_str().unwrap(),
            LibvirtGuestOptions::ExistingDisk { path, .. } => path.to_str().unwrap(),
            LibvirtGuestOptions::IsoGuest { .. } => unimplemented!(),
        };
        let cmd = vec!["qemu-img", "rebase", "-u", "-f", "qcow2", "-b", &remote_backing_image_path, "-F", "qcow2", clone_remote_path];
        run_testbed_orchestration_command(
            &common,
            target_testbed,
            "sudo",
            cmd,
            false,
            None,
        ).await?;
        Ok(())
    }

    async fn create_action(&self, common: OrchestrationCommon, machine_config: StateTestbedGuest) -> anyhow::Result<()> {
        tracing::info!("deploying guest {}", &machine_config.guest_type.name);
        let testbed_host = machine_config.testbed_host.as_ref().unwrap();
        let project_path = if is_master(&common, testbed_host) {
            common.project_working_dir.to_str().unwrap().to_string()
        } else {
            let testbed_user = &common.testbed_hosts.get(testbed_host)
                .unwrap()
                .username;
            format!("/home/{testbed_user}/testbed-projects/{}/", common.project_name)
        };
        let local_image_parent_path = get_local_image_folder_path(&project_path);
        let local_xml_path = get_xml_path(&local_image_parent_path, &machine_config);
        tracing::info!("guest {} local_xml_path {}", &machine_config.guest_type.name,local_xml_path);
        // to start the guest, we need to create the guest so that it's interface is made then add
        // the interface to the OVN integration bridge
        let cmd = vec!["virsh", "create", &local_xml_path];
        let create_guest_res = run_testbed_orchestration_command(
            &common,
            testbed_host,
            "sudo",
            cmd,
            false,
            None,
        ).await;
        match create_guest_res {
            Ok(_) => {}
            Err(err) => {
                // dont fail if the guest already exists
                let expected_err = "already exists with uuid".to_string();
                return if !err.to_string().trim().contains(&expected_err) {
                    // error in joining blocking thread
                    Err(err)
                } else {
                    tracing::warn!("tried to start guest {} but is already running, continuing...", &machine_config.guest_type.name);
                    Ok(())
                }
            }
        }
        // only provision an OVN port of is not a backing image guest (linked clones)
        if self.scaling.is_none() {
            // add guest's interface to network
            // TODO - need to get OVN integration bridge name from config rather than hardcode
            let net = &machine_config.guest_type.network
                .context("getting guest network in libvirt create action")?;

            for (idx, machine_network) in net.iter().enumerate() {
                let interface = get_guest_interface_name(&common.project_name, machine_config.guest_id, idx);
                tracing::info!("creating guest {} OVS port {} to bind to OVN network", &machine_config.guest_type.name, &interface);

                let lsp = format!("{}-{}-{}-{}", &common.project_name, machine_network.switch, &machine_config.guest_type.name, idx);
                let ext_id = format!("external_ids:iface-id={}", &lsp);

                let integration_bridge = &common.kvm_compose_config.testbed_host_ssh_config.get(testbed_host)
                    .unwrap().ovn.bridge;

                let cmd = vec![
                    "ovs-vsctl", "add-port", integration_bridge, &interface,
                    "--", "set", "Interface", &interface,
                    &ext_id,
                ];
                let create_guest_port_res = run_testbed_orchestration_command(
                    &common,
                    testbed_host,
                    "sudo",
                    cmd,
                    false,
                    None,
                ).await;
                match create_guest_port_res {
                    Ok(_) => {}
                    Err(err) => {
                        let expected_err = "already exists on bridge".to_string();
                        return if !err.to_string().trim().contains(&expected_err) {
                            // error in joining blocking thread
                            Err(err)
                        } else {
                            tracing::warn!("tried to add guest {} interface {} but it already exists, continuing...", &machine_config.guest_type.name, &interface);
                            Ok(())
                        }
                    }
                }
            }
        }

        Ok(())
    }

    async fn setup_action(&self, common: OrchestrationCommon, machine_config: StateTestbedGuest) -> anyhow::Result<()> {
        // run any setup
        let guest_name = format!("{}-{}", &common.project_name, &machine_config.guest_type.name);
        match &self.libvirt_type {
            LibvirtGuestOptions::CloudImage { setup_script, .. } => {
                if let Some(script) = setup_script {
                    tracing::info!("running setup script on guest {}", &machine_config.guest_type.name);
                    wait_for_guest_to_be_up(&common, &machine_config, vec!["ls"]).await?;
                    let local_script_path = script.to_str().unwrap().to_string();
                    SSHClient::push_file_to_guest(
                        &common,
                        &local_script_path,
                        &"/tmp".to_string(),
                        self.username.as_ref().unwrap(),
                        &guest_name,
                    ).await?;
                    SSHClient::run_guest_command(
                        &common,
                        vec!["sudo", "bash", format!("/tmp/{local_script_path}").as_str()],
                        &machine_config,
                        false,
                    ).await?;
                }
            }
            LibvirtGuestOptions::ExistingDisk { .. } => {}
            LibvirtGuestOptions::IsoGuest { .. } => {}
        }
        Ok(())
    }

    async fn run_action(&self, _common: OrchestrationCommon, _machine_config: StateTestbedGuest) -> anyhow::Result<()> {
        todo!()
    }

    async fn destroy_action(&self, common: OrchestrationCommon, machine_config: StateTestbedGuest) -> anyhow::Result<()> {
        let guest_name = format!("{}-{}", common.project_name, &machine_config.guest_type.name);
        if machine_config.is_golden_image {
            tracing::info!("making sure backing image guest {} is off", &guest_name);
        } else {
            tracing::info!("turning off guest {}", &machine_config.guest_type.name);
        }
        let cmd = vec!["virsh", "destroy", &guest_name];
        run_testbed_orchestration_command_allow_fail(
            &common,
            machine_config.testbed_host.as_ref().unwrap(),
            "sudo",
            cmd,
            false,
            None,
        ).await?;
        if machine_config.is_golden_image {
            // skip OVN bit for backing image guests, they dont have a port
            return Ok(());
        }
        // destroy the port(s)

        for (idx, _) in machine_config.guest_type.network.iter().enumerate() {
            let interface = get_guest_interface_name(&common.project_name, machine_config.guest_id, idx);
            let cmd = vec!["ovs-vsctl", "del-port", &interface];
            run_testbed_orchestration_command_allow_fail(
                &common,
                machine_config.testbed_host.as_ref().unwrap(),
                "sudo",
                cmd,
                false,
                None,
            ).await?;
        }

        Ok(())
    }

    async fn is_up(&self, common: OrchestrationCommon, machine_config: StateTestbedGuest) -> anyhow::Result<bool> {
        let testbed_host = machine_config.testbed_host.as_ref().unwrap();
        let guest_name = format!("{}-{}", &common.project_name, &machine_config.guest_type.name);

        let cmd = vec!["virsh", "dominfo", &guest_name];
        let res = run_testbed_orchestration_command(
            &common,
            testbed_host,
            "sudo",
            cmd,
            false,
            None,
        ).await?;
        if res.contains("running") {
            Ok(true)
        } else if res.contains("shut off") {
            Ok(false)
        } else {
            bail!("for guest {guest_name} could not find libvirt definition");
        }

    }
}

#[async_trait]
impl OrchestrationGuestTask for ConfigDockerMachine {
    async fn setup_image_action(&self, _common: OrchestrationCommon, _machine_config: StateTestbedGuest) -> anyhow::Result<()> {
        todo!()
    }

    async fn push_image_action(&self, common: OrchestrationCommon, machine_config: StateTestbedGuest) -> anyhow::Result<()> {

        let mut futures = Vec::new();
        let testbed_host = machine_config.testbed_host.as_ref().unwrap();
        if !is_master(&common, testbed_host) {
            // TODO - if the image is a local only image, needs to be pushed

            tracing::info!("pushing artefacts for guest {} to {}", &machine_config.guest_type.name, &testbed_host);

            // need to push any context to the remote testbed hosts, this will be any env files
            // and any volumes
            let project_folder = common.project_working_dir.to_str().unwrap();
            let remote_project_folder = get_remote_project_folder(&common, testbed_host)?;
            if let Some(env_file) = &self.env_file {
                // has env file, push
                let local_env_file = format!("{project_folder}/{env_file}");
                futures.push(SSHClient::push_file_to_remote_testbed(
                    &common,
                    testbed_host.clone(),
                    local_env_file,
                    remote_project_folder.clone(),
                    false,
                ));
            }
            if let Some(volumes) = &self.volumes {
                for vol in volumes {
                    let source = if vol.source.contains("${PWD}") {
                        // need to manually replace PWD as there is no shell
                        vol.source.replace("${PWD}", common.project_working_dir.to_str().unwrap())
                    } else {
                        format!("{project_folder}/{}", &vol.source)
                    };
                    let local_volume_path = source;
                    futures.push(SSHClient::push_file_to_remote_testbed(
                        &common,
                        testbed_host.clone(),
                        local_volume_path,
                        remote_project_folder.clone(),
                        false,
                    ));
                }
            }

        }
        try_join_all(futures).await?;
        Ok(())
    }

    async fn pull_image_action(&self, _common: OrchestrationCommon, _machine_config: StateTestbedGuest) -> anyhow::Result<()> {
        todo!()
    }

    async fn rebase_image_action(&self, _common: OrchestrationCommon, _machine_config: StateTestbedGuest, _guest_list: StateTestbedGuestList) -> anyhow::Result<()> {
        todo!()
    }

    async fn create_action(&self, common: OrchestrationCommon, machine_config: StateTestbedGuest) -> anyhow::Result<()> {

        tracing::info!("deploying guest {}", &machine_config.guest_type.name);

        let testbed_host = machine_config.testbed_host.as_ref().unwrap();
        let guest_name = format!("{}-{}", &common.project_name, &machine_config.guest_type.name);
        let mut cmd_string = vec!["sudo".to_string(), "docker".to_string(), "run".to_string(), "--rm".to_string(), "--name".to_string(), guest_name.clone(), "-it".to_string(), "-d".to_string()];
        // add entrypoint if set
        if let Some(entrypoint) = &self.entrypoint {
            cmd_string.push("--entrypoint".to_string());
            cmd_string.push(entrypoint.clone());
        }
        // add environment, if set
        if let Some(environment) = &self.environment {
            for (left, right) in environment.iter() {
                let left = left.clone();
                let right = right.clone();
                let env = format!("{left}={right}");
                cmd_string.push("-e".to_string());
                cmd_string.push(env);
            }
        }

        // add device, if set
        if let Some(devices) = &self.device {
            for device in devices.iter() {
                cmd_string.push(format!("--device={device}"))
            }
        }

        // add privileged, if set
        if let Some(_privileged) = &self.privileged {
            cmd_string.push("--privileged".to_string())
        }

        // add env file, if set
        if let Some(env_file) = &self.env_file {
            cmd_string.push("--env-file".to_string());
            // need to set the absolute path for env file since it might be running on remote
            if is_master(&common, testbed_host) {
                cmd_string.push(env_file.to_string());
            } else {
                // on remote
                let remote_project_folder = get_remote_project_folder(&common, testbed_host)?;
                cmd_string.push(format!("{remote_project_folder}/{env_file}"));
            }
        }

        // remove container from docker networking, to be attached to a testbed bridge in orchestration
        cmd_string.push("--net=none".to_string());

        // set volumes if any
        if let Some(volumes) = &self.volumes {
            for volume in volumes.iter() {
                // TODO - support docker notation for read only ":ro" etc
                let source = if volume.source.contains("${PWD}") {
                    // need to manually replace PWD as there is no shell
                    volume.source.replace("${PWD}", common.project_working_dir.to_str().unwrap())
                } else {
                    volume.source.clone()
                };
                cmd_string.push("-v".to_string());
                // let mount_arg = format!("{}:{}", &volume.source, &volume.target);
                cmd_string.push(format!("{}:{}", source, &volume.target));
            }
        }

        let net = &machine_config.guest_type.network
            .context("getting guest network in docker create action")?;

        // add dns servers to prevent the use of the host's dns. add cloudflare and the libvirt dns
        // assume there is only one interface for docker
        if !net.is_empty() {
            // for now assume only one interface
            if let Some(gateway) = &net[0].gateway {
                cmd_string.push(format!("--dns={}", gateway));
            }
        }

        // cmd_string.push(format!("--dns=1.0.0.1"));
        // let gateway = format!("--dns={}.1", common.network_subnet);
        // cmd_string.push(gateway.clone());

        if let Some(user) = &self.user {
            cmd_string.push("-u".to_string());
            cmd_string.push(user.clone());
        }

        // add hostname to container
        cmd_string.push("-h".to_string());
        cmd_string.push(guest_name.to_string());

        // add the docker image name
        cmd_string.push(self.image.clone());

        // add command for the container, if set
        if let Some(command) = &self.command {
            cmd_string.push(command.clone());
        }

        let mut ovs_cmd = vec!["sudo".to_string(), "ovs-docker".to_string(), "add-port".to_string()];

        let integration_bridge = &common.kvm_compose_config.testbed_host_ssh_config.get(testbed_host)
            .unwrap().ovn.bridge;

        ovs_cmd.push(integration_bridge.clone());
        ovs_cmd.push("eth0".to_string());
        ovs_cmd.push(guest_name.clone());



        match &common.network {
            StateNetwork::Ovn(ovn) => {
                if !net.is_empty() {
                    // assume only interface
                    let lsp_name = format!("{}-{}-{}-0", &common.project_name, &net[0].switch, &machine_config.guest_type.name);
                    let lsp = ovn.switch_ports.get(&lsp_name)
                        .context(format!("Getting LSP for docker guest {}", &guest_name))?;
                    // do ip address
                    // if guest has been given a dynamic ip, need to check on OVN for the assigned ip
                    let ip = &net[0].ip;
                    if ip.eq("dynamic") {
                        let dynamic_ip = get_lsp_dynamic_ip(&lsp_name, testbed_host, &common).await?;
                        ovs_cmd.push(format!("--ipaddress={dynamic_ip}/24"));
                    } else {
                        ovs_cmd.push(format!("--ipaddress={ip}/24"));
                    }

                    // do mac address
                    let mac = match &lsp.port_type {
                        LogicalSwitchPortType::Internal { mac_address, .. } => mac_address.address.clone(),
                        _ => unreachable!(),
                    };
                    ovs_cmd.push(format!("--macaddress={mac}"));
                }
            }
            StateNetwork::Ovs(_) => unimplemented!(),
        };


        // assume only one interface
        if !net.is_empty() {
            if let Some(gateway) = &net[0].gateway {
                ovs_cmd.push(format!("--gateway={gateway}"));
            }
        }

        // in the case when container is already defined on target host, but is not running we need
        // to remove the container and then continue with the code below
        let cmd = vec!["docker", "container", "inspect", "-f", "{{.State.Running}}", &guest_name];
        let guest_inspect_res = run_testbed_orchestration_command(
            &common,
            testbed_host,
            "sudo",
            cmd,
            false,
            None,
        ).await;
        match &guest_inspect_res {
            Ok(ok) => {
                // only remove container and port if we are forcing reprovision
                if common.force_provisioning {
                    // tracing::warn!("guest container {guest_name} already defined, removing container and associated port");
                    // check if running state is true or false
                    if ok.contains("false") {
                        // container created but not running, remove it
                        tracing::warn!("guest container {guest_name} found but not running, removing before continuing");
                        let cmd = vec!["docker", "container", "rm", &guest_name];
                        run_testbed_orchestration_command(
                            &common,
                            testbed_host,
                            "sudo",
                            cmd,
                            false,
                            None,
                        ).await?;
                    } else {
                        // container created and running, stop and remove it
                        tracing::warn!("guest container {guest_name} found running, removing before continuing");
                        let cmd = vec!["docker", "container", "rm", "-f", &guest_name];
                        run_testbed_orchestration_command(
                            &common,
                            testbed_host,
                            "sudo",
                            cmd,
                            false,
                            None,
                        ).await?;
                    }
                    if !net.is_empty() {
                        // and remove the ovs bridge - assume one interface
                        let port = format!("{}-{}-{}-0", common.project_name, &net[0].switch, &machine_config.guest_type.name);
                        let cmd = vec!["ovs-docker", "del-port", &port, "eth0", &guest_name];
                        run_testbed_orchestration_command(
                            &common,
                            testbed_host,
                            "sudo",
                            cmd,
                            false,
                            None,
                        ).await?;
                    }

                } else {
                    tracing::info!("guest container {guest_name} already defined")
                }
            }
            Err(err) => {
                // make sure we get the no such container error, meaning we can continue, any other
                // error we don't know how to handle yet
                let expected_error = "No such container";
                if !err.to_string().contains(expected_error) {
                    // error was not expected, cannot handle error here
                    return Err(anyhow!("{err:#}"));
                }
            }
        }

        // we want to run the docker create command as long as there are no existing containers, or
        // we want to force provisioning anyway. guest_inspect_res will be true if error as no
        // container was found, will be false if one was found but force provisioning will still
        // make the if true
        if guest_inspect_res.is_err() || common.force_provisioning {
            // create vec &str from generated command to avoid .. lifetime issues
            let cmd: Vec<&str> = cmd_string.iter()
                .map(|string| { string.as_str() })
                .collect();
            let create_guest_res = run_testbed_orchestration_command(
                &common,
                testbed_host,
                "sudo",
                cmd,
                false,
                None,
            ).await;

            match create_guest_res {
                Ok(_) => {
                    // successfully created container, now add it to the port
                    // create vec &str from generated command to avoid .. lifetime issues
                    let cmd: Vec<&str> = ovs_cmd.iter()
                        .map(|string| { string.as_str() })
                        .collect();
                    // panic if this doesn't work
                    run_testbed_orchestration_command(
                        &common,
                        testbed_host,
                        "sudo",
                        cmd,
                        false,
                        None,
                    ).await?;
                }
                Err(err) => {
                    let expected_err = format!("Conflict. The container name \"/{guest_name}\" is already in use by con");
                    if !err.to_string().trim().contains(&expected_err) {
                        return Err(err);
                    } else {
                        tracing::warn!("{err}");
                    }
                }
            }
            // now we need to add the guest to the OVN network, first we need the random generated
            // id of the port on the integration bridge
            let container_id = format!("external_ids:container_id={}", &guest_name);
            let port_uuid_cmd = vec![
                "ovs-vsctl", "--data=bare", "--no-heading", "--columns=name", "find", "interface",
                &container_id,
                // we hardcoded eth0 above so no need to derive it
                "external_ids:container_iface=eth0"
            ];
            let port_uuid = run_testbed_orchestration_command(
                &common,
                testbed_host,
                "sudo",
                port_uuid_cmd,
                false,
                None,
            ).await?;
            // need to remove trailing new line
            let port_uuid = port_uuid.strip_suffix('\n')
                .context("stripping newline from docker port uuid")?.to_string();
            // now set the external id on this interface with the OVN data so that it is bound to
            // our logical network
            if !net.is_empty() {
                // assume one interface
                let iface_id = format!("external_ids:iface-id={}-{}-{}-0", &common.project_name, &net[0].switch, &machine_config.guest_type.name);
                let set_interface_cmd = vec![
                    "ovs-vsctl", "set", "interface", &port_uuid, &iface_id
                ];
                tracing::debug!("docker set interface cmd: {:?}", set_interface_cmd);
                run_testbed_orchestration_command(
                    &common,
                    testbed_host,
                    "sudo",
                    set_interface_cmd,
                    false,
                    None,
                ).await?;
            }
        }

        Ok(())
    }

    async fn setup_action(&self, _common: OrchestrationCommon, _machine_config: StateTestbedGuest) -> anyhow::Result<()> {
        todo!()
    }

    async fn run_action(&self, _common: OrchestrationCommon, _machine_config: StateTestbedGuest) -> anyhow::Result<()> {
        todo!()
    }

    async fn destroy_action(&self, common: OrchestrationCommon, machine_config: StateTestbedGuest) -> anyhow::Result<()> {
        tracing::info!("turning off guest {}", &machine_config.guest_type.name);
        let testbed_host = machine_config.testbed_host.as_ref().unwrap();
        let guest_name = format!("{}-{}", &common.project_name, &machine_config.guest_type.name);

        let mut cmd_ovs = vec!["sudo".to_string(), "ovs-docker".to_string(), "del-port".to_string()];
        let integration_bridge = &common.kvm_compose_config.testbed_host_ssh_config.get(testbed_host)
            .unwrap().ovn.bridge;
        cmd_ovs.push(integration_bridge.clone());
        cmd_ovs.push("eth0".to_string());
        cmd_ovs.push(guest_name.clone());
        let cmd: Vec<&str> = cmd_ovs.iter()
            .map(|string| {string.as_str()})
            .collect();
        let destroy_guest_ovs_res = run_testbed_orchestration_command(
            &common,
            testbed_host,
            "sudo",
            cmd,
            false,
            None,
        ).await;
        match destroy_guest_ovs_res {
            Ok(_) => {}
            Err(err) => {
                tracing::warn!("port for guest {guest_name} probably already deleted");
                let expected_err = "Failed to find any attached port for CONTAINER".to_string();
                if !err.to_string().trim().contains(&expected_err) {
                    return Err(err);
                }
            }
        }


        let cmd = vec!["sudo", "docker", "stop", &guest_name];
        let destroy_guest_res = run_testbed_orchestration_command(
            &common,
            testbed_host,
            "sudo",
            cmd,
            false,
            None,
        ).await;
        match destroy_guest_res {
            Ok(_) => {}
            Err(err) => {
                tracing::warn!("guest {guest_name} probably already deleted");
                let expected_err = format!("No such container: {guest_name}");
                if !err.to_string().trim().contains(&expected_err) {
                    return Err(err);
                }
            }
        }

        Ok(())
    }

    async fn is_up(&self, _common: OrchestrationCommon, _machine_config: StateTestbedGuest) -> anyhow::Result<bool> {
        todo!()
    }
}

#[async_trait]
impl OrchestrationGuestTask for ConfigAVDMachine {
    async fn setup_image_action(&self, _common: OrchestrationCommon, _machine_config: StateTestbedGuest) -> anyhow::Result<()> {
        todo!()
    }

    async fn push_image_action(&self, _common: OrchestrationCommon, _machine_config: StateTestbedGuest) -> anyhow::Result<()> {
        todo!()
    }

    async fn pull_image_action(&self, _common: OrchestrationCommon, _machine_config: StateTestbedGuest) -> anyhow::Result<()> {
        todo!()
    }

    async fn rebase_image_action(&self, _common: OrchestrationCommon, _machine_config: StateTestbedGuest, _guest_list: StateTestbedGuestList) -> anyhow::Result<()> {
        todo!()
    }

    async fn create_action(&self, common: OrchestrationCommon, machine_config: StateTestbedGuest) -> anyhow::Result<()> {

        tracing::info!("deploying guest {}", &machine_config.guest_type.name);
        let testbed_host = machine_config.testbed_host.as_ref().unwrap();
        if !is_master(&common, testbed_host) {
            bail!("currently dont support AVD guests on remote testbed hosts");
        }

        // the avd will have been created by the artefact-generation, here we just need to create
        // the network namespace and add it to the network, then start the AVD

        let guest_name = &machine_config.guest_type.name;
        let guest_project_name = format!("{}-{}", &common.project_name, &machine_config.guest_type.name);
        // let gateway = format!("{}.1", &common.network_subnet);
        let project_name = &common.project_name;
        let namespace = format!("{project_name}-{guest_name}-nmspc");
        let net = &machine_config.guest_type.network
            .context("getting guest network in android create action")?;
        if !net.is_empty() {
            // only one interface allowed
            let guest_interface = get_guest_interface_name(&common.project_name, machine_config.guest_id, 0);
            let lsp_name = format!("{}-{}-{}-0", &common.project_name, &net[0].switch, &machine_config.guest_type.name);
            let mac = match &common.network {
                StateNetwork::Ovn(ovn) => {
                    let lsp = ovn.switch_ports.get(&lsp_name)
                        .context(format!("Getting LSP for android guest {}", &guest_name))?;
                    match &lsp.port_type {
                        LogicalSwitchPortType::Internal { mac_address, .. } => mac_address.address.clone(),
                        _ => unreachable!(),
                    }
                }
                StateNetwork::Ovs(_) => unimplemented!(),
            };
            let gateway = &net[0].gateway.as_ref()
                .context("android guest was not given a gateway")?;

            // create port for android guest
            let iface_id = format!("external_ids:iface-id={}", &lsp_name);
            let integration_bridge = &common.kvm_compose_config.testbed_host_ssh_config.get(testbed_host)
                .unwrap().ovn.bridge;
            let cmd = vec![
                "ovs-vsctl", "--may-exist", "add-port", integration_bridge, &guest_interface,
                "--", "set", "Interface", &guest_interface, "type=internal",
                "--", "set", "Interface", &guest_interface, &iface_id,
            ];
            run_testbed_orchestration_command_allow_fail(
                &common,
                testbed_host,
                "sudo",
                cmd,
                false,
                None,
            ).await?;


            // create namespace
            let cmd = vec!["ip", "netns", "add", &namespace];
            run_testbed_orchestration_command_allow_fail(
                &common,
                testbed_host,
                "sudo",
                cmd,
                false,
                None,
            ).await?;
            // make sure loopback is up
            let cmd = vec!["ip", "netns", "exec", &namespace, "ip", "link", "set", "dev", "lo", "up"];
            run_testbed_orchestration_command_allow_fail(
                &common,
                testbed_host,
                "sudo",
                cmd,
                false,
                None,
            ).await?;
            // put ovs port into namespace
            let cmd = vec![
                "ip", "link", "set", &guest_interface, "netns", &namespace,
            ];
            run_testbed_orchestration_command_allow_fail(
                &common,
                testbed_host,
                "sudo",
                cmd,
                false,
                None,
            ).await?;
            // set mac address of ovs port
            let cmd = vec![
                "ip", "netns", "exec", &namespace, "ip", "link", "set", &guest_interface, "address", &mac
            ];
            run_testbed_orchestration_command_allow_fail(
                &common,
                testbed_host,
                "sudo",
                cmd,
                false,
                None,
            ).await?;
            // set ip of ovs port
            let ip = if net[0].ip.eq("dynamic") {
                let lsp_name = format!("{}-{}-{}-0", &common.project_name, &net[0].switch, &machine_config.guest_type.name);
                
                get_lsp_dynamic_ip(&lsp_name, testbed_host, &common).await?
            } else {
                net[0].ip.clone()
            };
            // TODO - get mask for this ip
            let namespace_ip = format!("{ip}/24");
            let cmd = vec![
                "ip", "netns", "exec", &namespace, "ip", "addr", "add", &namespace_ip, "dev", &guest_interface,
            ];
            run_testbed_orchestration_command_allow_fail(
                &common,
                testbed_host,
                "sudo",
                cmd,
                false,
                None,
            ).await?;

            // set ovs port up
            let cmd = vec![
                "ip", "netns", "exec", &namespace, "ip", "link", "set", &guest_interface, "up"
            ];
            run_testbed_orchestration_command_allow_fail(
                &common,
                testbed_host,
                "sudo",
                cmd,
                false,
                None,
            ).await?;
            // set default route for ovs port
            let cmd = vec![
                "ip", "netns", "exec", &namespace, "ip", "route", "add", "default", "via", gateway, "dev", &guest_interface,
            ];
            run_testbed_orchestration_command_allow_fail(
                &common,
                testbed_host,
                "sudo",
                cmd,
                false,
                None,
            ).await?;
        }


        // finally, deploy avd in background
        let cmd = vec![
            "ip", "netns", "exec", &namespace, "/opt/android-sdk/emulator/emulator",
            "-avd", &guest_project_name,
            // qemu options

        ];
        run_testbed_orchestration_command(
            &common,
            testbed_host,
            "sudo",
            cmd,
            true,
            None,
        ).await?;

        Ok(())
    }

    async fn setup_action(&self, _common: OrchestrationCommon, _machine_config: StateTestbedGuest) -> anyhow::Result<()> {
        todo!()
    }

    async fn run_action(&self, _common: OrchestrationCommon, _machine_config: StateTestbedGuest) -> anyhow::Result<()> {
        todo!()
    }

    async fn destroy_action(&self, common: OrchestrationCommon, machine_config: StateTestbedGuest) -> anyhow::Result<()> {
        tracing::info!("turning off guest {}", &machine_config.guest_type.name);
        let guest_name = machine_config.guest_type.name;
        let testbed_host = machine_config.testbed_host.as_ref().unwrap();
        let project_name = &common.project_name;
        let namespace = format!("{project_name}-{guest_name}-nmspc");

        let guest_interface = get_guest_interface_name(&common.project_name, machine_config.guest_id, 0);

        // kill avd using ADB - will be the only emulator in namespace so will be called emulator-5554
        // with respect to the ADB server, since it is isolated
        let cmd = vec!["ip", "netns", "exec", &namespace, "/opt/android-sdk/platform-tools/adb", "-s", "emulator-5554", "emu", "kill"];
        run_testbed_orchestration_command_allow_fail(
            &common,
            testbed_host,
            "sudo",
            cmd,
            false,
            None,
        ).await?;
        // destroy the ovs port for the namespace
        let integration_bridge = &common.kvm_compose_config.testbed_host_ssh_config.get(testbed_host)
            .unwrap().ovn.bridge;
        let cmd = vec!["ovs-vsctl", "del-port", integration_bridge, &guest_interface];
        run_testbed_orchestration_command_allow_fail(
            &common,
            testbed_host,
            "sudo",
            cmd,
            false,
            None,
        ).await?;
        // destroy the namespace which will delete the veth as well
        let cmd = vec!["ip", "netns", "delete", &namespace];
        run_testbed_orchestration_command_allow_fail(
            &common,
            testbed_host,
            "sudo",
            cmd,
            false,
            None,
        ).await?;

        Ok(())
    }

    async fn is_up(&self, _common: OrchestrationCommon, _machine_config: StateTestbedGuest) -> anyhow::Result<bool> {
        todo!()
    }
}

fn get_local_image_folder_path(
    project_working_dir: &String,
) -> String {
    format!(
        "{}/artefacts/",
        project_working_dir,
    )
}

fn get_xml_path(
    local_image_parent_path: &String,
    machine_config: &StateTestbedGuest,
) -> String {
    let domain_xml_name = format!("{}-domain.xml", machine_config.guest_type.name);
    format!("{local_image_parent_path}/{domain_xml_name}")
}

async fn wait_for_guest_to_be_up(
    common: &OrchestrationCommon,
    machine_config: &StateTestbedGuest,
    command: Vec<&str>,
) -> anyhow::Result<()> {
    // we will poll the guest with the given command in a loop, for a number of attempts in a time
    // limit
    let attempt_limit = 24;
    let wait_time_in_seconds = 5;
    let guest_name = &machine_config.guest_type.name;
    let mut counter = 0;
    loop {
        tracing::info!("trying to poll guest {guest_name} to see if it is up ...");
        let poll_res = SSHClient::run_guest_command(
            common,
            command.clone(),
            machine_config,
            false,
        ).await;
        if counter > attempt_limit && poll_res.is_err() {
            // we waited 12 times with a wait, the command didnt work so this has failed
            bail!("could not connect to guest {}-{} to check if it is up, might not have booted successfully", &common.project_name, &machine_config.guest_type.name);
        } else if poll_res.is_ok() {
            // successful connection, return ok
            break;
        }
        tracing::info!("attempt {counter}/{attempt_limit} guest {guest_name} not up yet, waiting 5s and trying again");
        counter += 1;
        tokio::time::sleep(Duration::from_secs(wait_time_in_seconds)).await;

    }
    Ok(())
}

pub async fn calculate_backing_images_to_push(
    state: &State,
    common: &OrchestrationCommon,
) -> anyhow::Result<HashSet<(String, String)>> {
    // look at only the clones to see which remote testbed hosts they are being assigned so that we
    // can push a copy of the backing image to that testbed host
    let master_testbed_name = get_master_testbed_name(common);
    // use a set to prevent duplicates
    let mut assignment = HashSet::new();
    for (_guest_name, guest_data) in state.testbed_guests.0.iter() {
        match &guest_data.guest_type.guest_type {
            GuestType::Libvirt(libvirt) => {
                // check if guest is a clone and if not on the master testbed
                let guest_testbed = guest_data.testbed_host.as_ref().unwrap();
                if libvirt.is_clone_of.is_some() && !guest_testbed.eq(&master_testbed_name) {
                    // need to push a copy
                    let backing_image_name = libvirt.is_clone_of.as_ref().unwrap();
                    assignment.insert((backing_image_name.clone(), guest_testbed.clone()));
                }
            }
            GuestType::Docker(_) => {}
            GuestType::Android(_) => {}
        }
    }
    // tracing::info!("assignment = {assignment:?}");

    Ok(assignment)
}

pub fn get_master_testbed_name(
    common: &OrchestrationCommon,
) -> String {
    let master_testbed = {
        let temp: Vec<_> = common.testbed_hosts
            .iter()
            .filter(|(_, data)| {
                data.is_master_host
            })
            .map(|(name, _)| {name})
            .collect();
        temp[0] // get just the master as the vec is len 1
    };
    master_testbed.clone()
}

pub fn get_backing_image_local_path(
    testbed_guests: &StateTestbedGuestList,
    backing_guest_name: &String,
) -> anyhow::Result<String> {
    let backing_guest = testbed_guests.0.get(backing_guest_name).unwrap();
    let local_src = match &backing_guest.guest_type.guest_type {
        GuestType::Libvirt(libvirt) => {
            match &libvirt.libvirt_type {
                LibvirtGuestOptions::CloudImage { path, .. } => {
                    path.as_ref().unwrap().to_str().unwrap()
                }
                LibvirtGuestOptions::ExistingDisk { path, .. } => {
                    path.to_str().unwrap()
                }
                LibvirtGuestOptions::IsoGuest { .. } => unimplemented!(),
            }
        }
        _ => unreachable!(), // no linked clones for docker or android
    }.to_string();
    Ok(local_src)
}

pub fn get_backing_image_remote_path(
    common: &OrchestrationCommon,
    testbed_guests: &StateTestbedGuestList,
    backing_guest_name: &String,
    target_testbed: &String,
) -> anyhow::Result<String> {
    let local_src = get_backing_image_local_path(testbed_guests, backing_guest_name)?;
    // we need to know the filename of the local image
    let image_name_extension = PathBuf::from(&local_src)
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    // need to know details about the remote filesystem path
    let remote_user = &common.testbed_hosts.get(target_testbed)
        .unwrap()
        .username;
    let project_name = &common.project_name;
    let remote_project_folder = format!("/home/{remote_user}/testbed-projects/{project_name}");
    Ok(format!("{remote_project_folder}/artefacts/{image_name_extension}"))

}

fn get_remote_project_folder(
    common: &OrchestrationCommon,
    testbed_host: &String,
) -> anyhow::Result<String> {
    let remote_testbed = common.testbed_hosts.get(testbed_host)
        .context("Could not find remote testbed in config")?;
    let project_name = &common.project_name;
    Ok(format!("/home/{}/testbed-projects/{project_name}/", &remote_testbed.username))
}

/// Get the ip address assigned by OVN to a logical switch port that has been given a dynamic IP
/// address.
async fn get_lsp_dynamic_ip(
    lsp_name: &String,
    testbed_host: &String,
    orchestration_common: &OrchestrationCommon,
) -> anyhow::Result<String> {
    tracing::info!("getting dynamic ip address assigned to logical switch port {lsp_name}");
    let name = format!("name={lsp_name}");
    let res = run_testbed_orchestration_command(
        orchestration_common,
        testbed_host,
        "sudo",
        vec!["ovn-nbctl", "--bare", "--columns=dynamic_addresses", "find", "Logical_Switch_Port", &name],
        false,
        None,
    ).await?;
    // result will be "mac ip" so we need to make sure there are two results, get the second, and
    // then also remove the trailing newline
    let split: Vec<_> = res.split(' ').collect();
    if split.len() != 2 {
        bail!("could not get the dynamic ip for lsp {lsp_name} as result from NB DB was {}", &res);
    }
    let ip = split[1];
    let ip = ip.strip_suffix('\n')
        .context("stripping newline from dynamic ip")?;

    Ok(ip.to_string())
}
