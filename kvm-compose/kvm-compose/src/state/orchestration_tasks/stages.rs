use anyhow::Context;
use tokio::sync::mpsc::{Sender};
use kvm_compose_schemas::kvm_compose_yaml::machines::GuestType;
use crate::orchestration::api::{OrchestrationInstruction, OrchestrationProtocol, OrchestrationResource};
use crate::orchestration::websocket::{send_orchestration_instruction_over_channel};
use crate::orchestration::{OrchestrationCommon};
use crate::state::orchestration_tasks::guests::{get_master_testbed_name};
use crate::state::State;

pub async fn deploy_guest_stage(
    state: &State,
    sender: &mut Sender<OrchestrationProtocol>,
    // receiver: &mut Receiver<OrchestrationProtocol>,
) -> anyhow::Result<()> {
    tracing::info!("Stage: deploying guests");

    let mut orchestration_resources_deploy_guests = Vec::new();
    for (_guest_name, guest_data) in state.testbed_guests.0.iter() {
        match &guest_data.guest_type.guest_type {
            GuestType::Libvirt(_) => {
                if !guest_data.is_golden_image {
                    orchestration_resources_deploy_guests.push(
                        OrchestrationResource::Guest(guest_data.clone())
                    );
                }
            }
            GuestType::Docker(docker) => {
                if docker.scaling.is_none() {
                    orchestration_resources_deploy_guests.push(
                        OrchestrationResource::Guest(guest_data.clone())
                    );
                }
            }
            GuestType::Android(android) => {
                if android.scaling.is_none() {
                    orchestration_resources_deploy_guests.push(
                        OrchestrationResource::Guest(guest_data.clone())
                    );
                }
            }
        }
    }
    if orchestration_resources_deploy_guests.is_empty() {
        return Ok(());
    }
    send_orchestration_instruction_over_channel(
        sender,
        // receiver,
        OrchestrationInstruction::Deploy(orchestration_resources_deploy_guests),
    ).await.context("requesting the deployment of guests")?;

    Ok(())
}

pub async fn destroy_guest_stage(
    state: &State,
    sender: &mut Sender<OrchestrationProtocol>,
    // receiver: &mut Receiver<OrchestrationProtocol>,
) -> anyhow::Result<()> {
    tracing::info!("Stage: destroying guests");

    let mut orchestration_resources_destroy_guests = Vec::new();
    for (_guest_name, guest_data) in state.testbed_guests.0.iter() {
        match &guest_data.guest_type.guest_type {
            GuestType::Libvirt(_) => {
                if !guest_data.is_golden_image {
                    orchestration_resources_destroy_guests.push(
                        OrchestrationResource::Guest(guest_data.clone())
                    );
                }
            }
            GuestType::Docker(docker) => {
                if docker.scaling.is_none() {
                    orchestration_resources_destroy_guests.push(
                        OrchestrationResource::Guest(guest_data.clone())
                    );
                }
            }
            GuestType::Android(android) => {
                if android.scaling.is_none() {
                    orchestration_resources_destroy_guests.push(
                        OrchestrationResource::Guest(guest_data.clone())
                    );
                }
            }
        }
    }
    if orchestration_resources_destroy_guests.is_empty() {
        return Ok(());
    }
    send_orchestration_instruction_over_channel(
        sender,
        // receiver,
        OrchestrationInstruction::Destroy(orchestration_resources_destroy_guests),
    ).await.context("requesting the destruction of guests")?;

    Ok(())
}

/// Filter all guests that are a "backing image" and request image setup
pub async fn setup_backing_image_stage(
    state: &State,
    sender: &mut Sender<OrchestrationProtocol>,
    // receiver: &mut Receiver<OrchestrationProtocol>,
) -> anyhow::Result<()> {

    let mut orchestration_resources = Vec::new();

    for (_guest_name, guest_data) in state.testbed_guests.0.iter() {
        if guest_data.is_golden_image {
            match &guest_data.guest_type.guest_type {
                GuestType::Libvirt(_) => {
                    if guest_data.is_golden_image {
                        orchestration_resources.push(
                            OrchestrationResource::Guest(guest_data.clone())
                        );
                    }
                }
                GuestType::Docker(_) => unimplemented!(), // build from Dockerfile
                GuestType::Android(_) => unimplemented!(), // create AVD
            }
        }
    }
    if orchestration_resources.is_empty() {
        return Ok(());
    }
    send_orchestration_instruction_over_channel(
        sender,
        // receiver,
        OrchestrationInstruction::SetupImage(orchestration_resources),
    ).await.context("requesting the setup of guest backing images ")?;

    Ok(())
}

/// Filter all guests that are "clones" of a "backing image" and request image setup
pub async fn setup_linked_clones_stage(
    state: &State,
    sender: &mut Sender<OrchestrationProtocol>,
    // receiver: &mut Receiver<OrchestrationProtocol>,
) -> anyhow::Result<()> {

    let mut orchestration_resources = Vec::new();

    for (_guest_name, guest_data) in state.testbed_guests.0.iter() {
        match &guest_data.guest_type.guest_type {
            GuestType::Libvirt(libvirt) => {
                if libvirt.is_clone_of.is_some() {
                    orchestration_resources.push(
                        OrchestrationResource::Guest(guest_data.clone())
                    );
                }
            }
            GuestType::Docker(_) => {} // not applicable
            GuestType::Android(_) => {} // not applicable
        }
    }
    if orchestration_resources.is_empty() {
        return Ok(());
    }
    send_orchestration_instruction_over_channel(
        sender,
        // receiver,
        OrchestrationInstruction::SetupImage(orchestration_resources),
    ).await.context("requesting the setup of clone guest images")?;

    Ok(())
}

/// For all guests, push the image to remote testbed hosts if applicable
pub async fn push_guest_images_stage(
    state: &State,
    sender: &mut Sender<OrchestrationProtocol>,
    // receiver: &mut Receiver<OrchestrationProtocol>,
) -> anyhow::Result<()> {
    let mut orchestration_resources = Vec::new();

    for (_guest_name, guest_data) in state.testbed_guests.0.iter() {
        match &guest_data.guest_type.guest_type {
            GuestType::Libvirt(_) => {
                orchestration_resources.push(
                    OrchestrationResource::Guest(guest_data.clone())
                );
            }
            GuestType::Docker(_) => {
                orchestration_resources.push(
                    OrchestrationResource::Guest(guest_data.clone())
                );
            }
            GuestType::Android(_) => {} // Android guests currently only supported on master testbed host
        }
    }
    if orchestration_resources.is_empty() {
        return Ok(());
    }
    send_orchestration_instruction_over_channel(
        sender,
        // receiver,
        OrchestrationInstruction::PushArtefacts(orchestration_resources),
    ).await.context("requesting the pushing of guest images to remote testbed hosts")?;

    Ok(())
}

pub async fn push_backing_guest_images_stage(
    state: &State,
    common: &OrchestrationCommon,
    sender: &mut Sender<OrchestrationProtocol>,
    // receiver: &mut Receiver<OrchestrationProtocol>,
) -> anyhow::Result<()> {
    let mut orchestration_resources = Vec::new();

    // based on `calculate_backing_images_to_push`

    let master_testbed_name = get_master_testbed_name(common);
    for (_guest_name, guest_data) in state.testbed_guests.0.iter() {
        match &guest_data.guest_type.guest_type {
            GuestType::Libvirt(libvirt) => {
                // check if guest is a clone and if not on the master testbed
                let guest_testbed = guest_data.testbed_host.as_ref().unwrap();
                if libvirt.is_clone_of.is_some() && !guest_testbed.eq(&master_testbed_name) {
                    // need to push a copy
                    orchestration_resources.push(
                        OrchestrationResource::Guest(guest_data.clone())
                    );
                }
            }
            GuestType::Docker(_) => {}
            GuestType::Android(_) => {}
        }
    }
    if orchestration_resources.is_empty() {
        return Ok(());
    }
    send_orchestration_instruction_over_channel(
        sender,
        // receiver,
        OrchestrationInstruction::PushBackingImages(orchestration_resources),
    ).await.context("requesting the pushing of guest backing images to remote testbed hosts")?;

    Ok(())
}

pub async fn rebase_clone_images_stage(
    state: &State,
    common: &OrchestrationCommon,
    sender: &mut Sender<OrchestrationProtocol>,
    // receiver: &mut Receiver<OrchestrationProtocol>,
) -> anyhow::Result<()> {
    let mut orchestration_resources = Vec::new();

    for (_guest_name, guest_data) in state.testbed_guests.0.iter() {
        // only rebase on remote testbeds
        if !guest_data.testbed_host.as_ref().unwrap().eq(&get_master_testbed_name(&common)) {
            match &guest_data.guest_type.guest_type {
                GuestType::Libvirt(libvirt) => {
                    if libvirt.is_clone_of.is_some() {
                        orchestration_resources.push(
                            OrchestrationResource::Guest(guest_data.clone())
                        );
                    }
                }
                _ => {} // no rebasing for docker or android
            }
        }
    }
    if orchestration_resources.is_empty() {
        return Ok(());
    }
    send_orchestration_instruction_over_channel(
        sender,
        // receiver,
        OrchestrationInstruction::RebaseRemoteBackingImages(orchestration_resources),
    ).await.context("requesting the rebasing of guest images to remote testbed hosts")?;

    Ok(())
}

pub async fn run_guest_setup_scripts_stage(
    state: &State,
    sender: &mut Sender<OrchestrationProtocol>,
    // receiver: &mut Receiver<OrchestrationProtocol>,
) -> anyhow::Result<()> {
    let mut orchestration_resources = Vec::new();

    for (_guest_name, guest_data) in state.testbed_guests.0.iter() {
        match &guest_data.guest_type.guest_type {
            GuestType::Libvirt(_) => {
                orchestration_resources.push(
                    OrchestrationResource::Guest(guest_data.clone())
                );
            }
            GuestType::Docker(_) => {} // not applicable at this time
            GuestType::Android(_) => {} // not applicable at this time
        }
    }
    if orchestration_resources.is_empty() {
        return Ok(());
    }
    send_orchestration_instruction_over_channel(
        sender,
        // receiver,
        OrchestrationInstruction::RunSetupScripts(orchestration_resources),
    ).await.context("requesting the execution of guest setup scripts")?;

    Ok(())
}