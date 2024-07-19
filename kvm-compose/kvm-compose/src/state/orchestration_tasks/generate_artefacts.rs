use kvm_compose_schemas::TESTBED_SETTINGS_FOLDER;
use std::path::PathBuf;
use anyhow::{bail, Context};
use nix::unistd::{Gid, Uid};
use kvm_compose_schemas::kvm_compose_yaml::MachineNetwork;
use kvm_compose_schemas::kvm_compose_yaml::machines::avd::ConfigAVDMachine;
use kvm_compose_schemas::kvm_compose_yaml::machines::docker::ConfigDockerMachine;
use kvm_compose_schemas::kvm_compose_yaml::machines::GuestType;
use kvm_compose_schemas::kvm_compose_yaml::machines::libvirt::{ConfigLibvirtMachine, LibvirtGuestOptions};
use crate::components::get_guest_interface_name;
use crate::components::helpers::{check_file_exists, serialisation};
use crate::components::helpers::android::{create_avd, download_system_image, get_sdk_string};
use crate::components::helpers::artefact_generation::{copy_and_set_permissions_orchestration, resize};
use crate::components::helpers::cloud_init::{create_meta_data, create_network_config, create_user_data};
use crate::components::helpers::xml::render_libvirt_domain_xml;
use crate::orchestration::{OrchestrationCommon, run_testbed_orchestration_command};
use crate::state::{State, StateTestbedGuest};

/// This is the generate artefacts version for State rather than Logical testbed
pub async fn generate_artefacts(
    state: &State,
    common: &OrchestrationCommon,
) -> anyhow::Result<()> {
    // for each guest, generate artefacts
    for (_, guest_config) in &state.testbed_guests.0 {
        match &guest_config.guest_type.guest_type {
            GuestType::Libvirt(c) => libvirt(c, guest_config, common).await?,
            GuestType::Docker(c) => docker(c, guest_config, common).await?,
            GuestType::Android(c) => android(c, guest_config, common).await?,
        }
    }

    Ok(())
}

async fn libvirt(
    libvirt_config: &ConfigLibvirtMachine,
    guest_config: &StateTestbedGuest,
    common: &OrchestrationCommon
) -> anyhow::Result<()> {

    let mut project_artefacts_folder = common.project_working_dir.clone();
    project_artefacts_folder.push("artefacts");
    let project_artefacts_folder = project_artefacts_folder.to_str()
        .context("converting artefacts folder from pathbuf to string")?
        .to_string();

    let client_name = format!("{}-{}", common.project_name, &guest_config.guest_type.name);
    let network_def = &guest_config.guest_type.network;

    // determine if the guest is on master or remote testbed, which impacts the path of the iso
    let master_host = common.get_master()?;
    let guests_testbed_host = guest_config.testbed_host.clone()
        .context("getting guest's testbed host")?;
    let artefacts_folder_at_final_location = if master_host.eq(&guests_testbed_host) {
        // on master
        project_artefacts_folder.clone()
    } else {
        // on remote
        let remote_host_username =
            &common.kvm_compose_config.testbed_host_ssh_config[&guests_testbed_host].user;
        let project_name = &common.project_name;
        format!("/home/{remote_host_username}/testbed-projects/{project_name}/artefacts/").to_owned()
    };

    // set up the XML template

    let mut tera_context = tera::Context::new();
    tera_context.insert("guest_name", &client_name);
    tera_context.insert("vcpu", &libvirt_config.cpus
        .context("getting n cpus for libvirt guest")?.to_string());
    tera_context.insert("memory", &libvirt_config.memory_mb
        .context("getting memory for libvirt guest")?.to_string());

    // main disk
    tera_context.insert("disk_driver", &"qcow2".to_string());
    // this is either on master or on remote
    let main_disk_img_path = match &libvirt_config.libvirt_type {
        LibvirtGuestOptions::CloudImage { path,.. } => {
            let img_path = path
                .clone()
                .context("getting disk path for libvirt cloud-image guest")?;
            tera_context.insert("disk_path", &img_path);
            // get cloud-init ISO based on name, it doesn't exist yet
            if libvirt_config.is_clone_of.is_some() {
                tera_context.insert(
                    "cloud_init_iso",
                    &format!("{}/{}-linked-clone.iso", &artefacts_folder_at_final_location, &guest_config.guest_type.name),
                );
            } else {
                tera_context.insert(
                    "cloud_init_iso",
                    &format!("{}/{}-cloud-init.iso", &artefacts_folder_at_final_location, &guest_config.guest_type.name),
                );
            }

            img_path
        }
        LibvirtGuestOptions::ExistingDisk { path,.. } => {
            tera_context.insert("disk_path", path);
            path.clone()
        }
        LibvirtGuestOptions::IsoGuest { path,.. } => {
            tera_context.insert("disk_path", path);
            path.clone()
        }
    };

    let main_disk_img_name = match main_disk_img_path.file_name() {
        None => bail!("could not get guest's filename and extension"),
        Some(os_str) => {
            os_str.to_str()
                .context("getting guest file name and extension from OsStr")?
                .to_string()
        }
    };

    // other options - TODO - this is currently disabled as this clashes if multiple deployments
    if let Some(tty) = &libvirt_config.tcp_tty_port {
        tera_context.insert("tcp_ttp_port", tty);
    }

    // create the interface name for the guest
    // add to integration bridge
    if libvirt_config.scaling.is_none() {
        // there may not be any network defined
        if let Some(some_network_def) = network_def {
            // vec of vec, outer vec is interface, inner vec is interface name+mac
            let mut guest_interfaces = Vec::new();
            for (idx, yaml_interface) in some_network_def.iter().enumerate() {
                if idx > 9 {
                    bail!("currently don't support a guest with more than 10 interfaces");
                }
                let interface = get_guest_interface_name(&common.project_name, guest_config.guest_id, idx);
                let mac = yaml_interface.mac.clone();
                let mut interface_id = format!("{idx}");
                if interface_id.len() == 1 {
                    interface_id = format!("0{interface_id}");
                }
                let interface_and_mac = vec![interface, mac, interface_id];
                guest_interfaces.push(interface_and_mac);

            }
            if !guest_interfaces.is_empty() {
                tera_context.insert("interfaces", &guest_interfaces);
            }
        }

    }

    // if we are using an existing disk guest, we will enable further video support as the guest
    // will likely have a desktop environment and without this there will be some significant delay
    // and tearing in the graphics - also add for iso guest as there will also be the likelihood
    // of a desktop environment
    // TODO - make this an option in the yaml
    match libvirt_config.libvirt_type {
        LibvirtGuestOptions::ExistingDisk { .. } => {
            tera_context.insert("extended_graphics_support", &true);
        }
        LibvirtGuestOptions::IsoGuest { .. } => {
            tera_context.insert("extended_graphics_support", &true);
        }
        _ => {}
    }

    // backing image for libvirt network, on project linux bridge and not on integration bridge
    if libvirt_config.scaling.is_some() {
        tera_context.insert("backing_image_network", &format!("{}-testbedos", common.project_name));
    }
    let xml = render_libvirt_domain_xml(tera_context)?;
    let xml_dest = format!(
        "{}/{}-domain.xml",
        &project_artefacts_folder, &guest_config.guest_type.name
    );
    serialisation::write_file_with_permissions(
        xml_dest.clone(),
        xml.to_string(),
        0o755,
        Uid::from_raw(common.fs_user),
        Gid::from_raw(common.fs_group),
    ).await?;
    // end of xml templating

    // get disk expand value
    let disk_expand = match &libvirt_config.libvirt_type {
        LibvirtGuestOptions::CloudImage {
            expand_gigabytes, ..
        } => {
            if expand_gigabytes.is_some() {
                expand_gigabytes.context("getting expand_gigabytes value for cloud image libvirt guest")?
            } else {
                0
            }
        }
        LibvirtGuestOptions::ExistingDisk { .. } => 0,
        LibvirtGuestOptions::IsoGuest { expand_gigabytes, .. } => {
            if expand_gigabytes.is_some() {
                expand_gigabytes.context("getting expand gigabytes for iso libvirt guest")?
            } else {
                0
            }
        }
    };

    // get the local disk path even if guest will be on remote
    let disk_path_on_master = format!("{project_artefacts_folder}/{main_disk_img_name}");

    // get the location of image specified in the yaml and create a copy of the image in the guest artefact folder, if
    // it is a non clone or is a backing image guest
    // implementation for each libvirt type
    match &libvirt_config.libvirt_type {
        LibvirtGuestOptions::CloudImage { name, .. } => {
            if libvirt_config.is_clone_of.is_some() {
                // is a clone, dont need to copy image as orchestration will create the linked
                // clone image once the backing image is deployed and setup
            } else {
                // not a clone, get copy of cloud init from testbed folder
                let cloud_init_image_path = name
                    .resolve_path(format!("{TESTBED_SETTINGS_FOLDER}/images/").into()).await?;
                // if disk already exists, leave it unless force provisioning is true
                if !check_file_exists(&disk_path_on_master) || common.force_provisioning {
                    // create a copy of the cloud image into the artefacts folder
                    copy_and_set_permissions_orchestration(&cloud_init_image_path, &disk_path_on_master, 0o755, common).await?;
                    // resize disk - todo fn
                    if disk_expand > 0 {
                        tracing::info!("expanding {} disk by +{}G at {}", client_name, disk_expand, disk_path_on_master);
                        resize(disk_path_on_master, disk_expand)
                            .await
                            .context("resizing guest disk")?;
                    }
                } else {
                    tracing::warn!("cloud image {disk_path_on_master} already exists, skipping create");
                }
            }
        }
        LibvirtGuestOptions::ExistingDisk { create_deep_copy, .. } => {
            if libvirt_config.is_clone_of.is_some() {
                // is a clone, dont need to copy image as orchestration will create the linked
                // clone image once the backing image is deployed and setup
            } else {
                // if already exists dont copy over unless force provisioning is true
                if !check_file_exists(&disk_path_on_master) || common.force_provisioning {

                    let reference_image = guest_config
                        .extra_info
                        .reference_image
                        .as_ref()
                        .context("getting reference image for existing disk, must exist")?;

                    if *create_deep_copy {
                        // create a copy of the image into the artefacts folder
                        copy_and_set_permissions_orchestration(
                            &PathBuf::from(reference_image),
                            &disk_path_on_master,
                            0o755,
                            common
                        ).await?;
                    } else {
                        // create a linked clone, unless user has specified to make a raw copy
                        let cmd = vec!["qemu-img", "create", "-f", "qcow2", "-b", reference_image, "-F", "qcow2", &disk_path_on_master];
                        run_testbed_orchestration_command(
                            common,
                            &master_host,
                            "sudo",
                            cmd,
                            false,
                            None,
                        ).await.context("creating a linked clone of the reference existing disk")?;
                    }

                    // resize disk
                    if disk_expand > 0 {
                        tracing::info!("expanding {} disk by +{}G at {}", client_name, disk_expand, disk_path_on_master);
                        resize(disk_path_on_master, disk_expand)
                            .await
                            .context("resizing guest disk")?;
                    }
                } else {
                    tracing::warn!("existing disk image {disk_path_on_master} already exists, skipping copy from original");
                }
            }
        }
        LibvirtGuestOptions::IsoGuest { .. } => {
            if libvirt_config.is_clone_of.is_some() {
                bail!("scaling is not supported currently for libvirt iso guests");
            } else {
                // if already exists dont copy over unless force provisioning is true
                if !check_file_exists(&disk_path_on_master) || common.force_provisioning {

                    let reference_image = guest_config
                        .extra_info
                        .reference_image
                        .as_ref()
                        .context("getting reference image for iso guest, must exist")?;

                    // create a copy of the iso into the artefacts folder
                    copy_and_set_permissions_orchestration(&PathBuf::from(reference_image), &disk_path_on_master, 0o755, common).await?;

                    // resize disk
                    // TODO, need to create a disk image then mount the iso into the guest
                    //  otherwise as it is now iso guest does not work
                    if disk_expand > 0 {
                        tracing::info!("expanding {} disk by +{}G at {}", client_name, disk_expand, disk_path_on_master);
                        resize(disk_path_on_master, disk_expand)
                            .await
                            .context("resizing guest disk")?;
                    }
                } else {
                    tracing::warn!("iso guest image {disk_path_on_master} already exists, skipping create");
                }
            }
        }
    };

    // todo -  add extended graphics entry to support GUI desktop environments, otherwise the rendering is software
    //  which can be very slow on less powerful hardware

    // set up cloud init data only for cloud images
    match libvirt_config.libvirt_type {
        LibvirtGuestOptions::CloudImage { .. } => cloud_init_setup(
            common,
            network_def,
            libvirt_config,
            client_name.clone(),
            project_artefacts_folder.clone(),
            guest_config,
        ).await?,
        _ => {}
    }

    Ok(())
}

async fn docker(
    _docker_config: &ConfigDockerMachine,
    _guest_config: &StateTestbedGuest,
    _common: &OrchestrationCommon
) -> anyhow::Result<()> {
    Ok(())
}

async fn android(
    _android_config: &ConfigAVDMachine,
    guest_config: &StateTestbedGuest,
    common: &OrchestrationCommon
) -> anyhow::Result<()> {
    // get the avd settings
    let guest_options = match &guest_config.guest_type.guest_type {
        GuestType::Android(avd_guest) => avd_guest,
        _ => unreachable!(),
    };

    let project_name = &common.project_name;
    let name = &guest_config.guest_type.name;
    let avd_name = format!("{project_name}-{name}");
    let project_path = common.project_working_dir.to_str()
        .context("getting project path from common")?;

    // if has scaling parameter, then we have already created clone configs so we dont spawn the
    // original otherwise we would have one more than was asked for
    if guest_options.scaling.is_some() {
        return Ok(());
    }

    // create avd image dependant on AVD guest option
    let options = get_sdk_string(&guest_options.avd_type)?;

    // make sure the system image is downloaded
    let options_clone = options.clone();
    tokio::task::spawn_blocking( move || {
        download_system_image(&options_clone).expect("downloading vm system image")
    })
        .await
        .context("downloading cloud image")?;


    // build base command, force overwrite as we are generating artefacts
    let avd_path = format!("{project_path}/artefacts/{avd_name}");


    if !check_file_exists(&avd_path) || common.force_provisioning {
        let avd_path_clone = avd_path.clone();
        tokio::task::spawn_blocking( move || {
            let build_avd_command = vec![
                "/opt/android-sdk/cmdline-tools/latest/bin/avdmanager", "create", "avd",
                "-n", &avd_name, "-k", &options, "--force",
                "--path", &avd_path_clone,
            ];
            create_avd(&avd_name, build_avd_command).expect("create avd in spawned thread")
        })
            .await
            .context("spawning avd")?
    } else {
        tracing::warn!("AVD guest {avd_name} already created, skipping create");
    }

    common.apply_user_file_perms(&avd_path.into())?;

    Ok(())
}

async fn cloud_init_setup(
    common: &OrchestrationCommon,
    network_def: &Option<Vec<MachineNetwork>>,
    libvirt_config: &ConfigLibvirtMachine,
    client_name: String,
    project_artefacts_folder: String,
    guest_config: &StateTestbedGuest,
) -> anyhow::Result<()> {
    // cloud init options, place cloud init assets into guest artefacts folder
    let public_ssh_key_contents =
        tokio::fs::read_to_string(&common.kvm_compose_config.ssh_public_key_location)
            .await
            .context(
                format!(
                    "Could not read ssh public key at {}",
                    &common.kvm_compose_config.ssh_public_key_location
                ),
            )?;

    let meta_data = create_meta_data(
        client_name,
        public_ssh_key_contents,
        match &libvirt_config.libvirt_type {
            LibvirtGuestOptions::CloudImage { environment, ..  } => {
                environment.clone()
            }
            _ => {
                unreachable!()
            }
        },
        // set_ip
    );
    let meta_dest_str = format!("{}/meta-data", &project_artefacts_folder);
    let meta_data_dest = if !check_file_exists(&meta_dest_str) || common.force_provisioning {
        let meta_data_dest = serialisation::write_file_with_permissions(
            meta_dest_str,
            meta_data,
            0o755,
            Uid::from_raw(common.fs_user),
            Gid::from_raw(common.fs_group),
        ).await.context("writing meta-data file")?;
        Some(meta_data_dest)
    } else {
        None
    };


    let user_data = create_user_data();
    let user_data_str = format!("{}/user-data", &project_artefacts_folder);
    let user_data_dest = if !check_file_exists(&user_data_str) || common.force_provisioning {
        let user_data_dest = serialisation::write_file_vecu8_with_permissions_orchestration(
            user_data_str,
            user_data,
            0o755,
            common,
        ).await.context("writing user-data file")?;
        Some(user_data_dest)
    } else {
        None
    };


    let network_config = create_network_config(network_def)?;
    let network_config_str = format!("{}/network-config", &project_artefacts_folder);
    let network_config_dest = if !check_file_exists(&network_config_str) || common.force_provisioning {
        let network_config_dest = serialisation::write_file_vecu8_with_permissions_orchestration(
            network_config_str,
            network_config,
            0o755,
            common,
        ).await.context("writing network-config file")?;
        Some(network_config_dest)
    } else {
        None
    };
    // write cloud init iso
    if meta_data_dest.is_some() && user_data_dest.is_some() && network_config_dest.is_some() {
        let mut cloud_init_inputs = vec![
            meta_data_dest.unwrap(),
            user_data_dest.unwrap(),
            network_config_dest.unwrap(),
        ];
        let iso_dest = if libvirt_config.is_clone_of.is_some() {
            format!(
                "{}/{}-linked-clone.iso",
                &project_artefacts_folder, &guest_config.guest_type.name
            )
        } else {
            format!(
                "{}/{}-cloud-init.iso",
                &project_artefacts_folder, &guest_config.guest_type.name
            )
        };
        // add context if present in yaml
        match &libvirt_config.libvirt_type {
            LibvirtGuestOptions::CloudImage { context, .. } => match &context {
                None => {}
                Some(context) => {
                    let context_dest = PathBuf::from(format!("{}/context.tar", &project_artefacts_folder));
                    serialisation::tar_cf(&context_dest, context).await?;
                    cloud_init_inputs.push(context_dest);
                }
            },
            LibvirtGuestOptions::ExistingDisk { .. } => {
                unreachable!()
            }
            LibvirtGuestOptions::IsoGuest { .. } => {
                unreachable!()
            }
        }
        serialisation::genisoimage_orchestration(PathBuf::from(&iso_dest).as_path(), cloud_init_inputs.clone(), common)
            .await
            .context("Could not create cloud init iso image")?;

        // delete the iso image files
        for ff in cloud_init_inputs {
            tokio::fs::remove_file(ff).await?;
        }
    }

    Ok(())
}
