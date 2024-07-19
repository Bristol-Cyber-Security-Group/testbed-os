use anyhow::Context;
use chrono::{DateTime, Utc};
use futures_util::future::try_join_all;
use kvm_compose_schemas::kvm_compose_yaml::machines::GuestType;
use crate::orchestration::{is_main_testbed, OrchestrationCommon, OrchestrationGuestTask, run_testbed_orchestration_command};
use crate::state::{State};

/// This function will create the whole testbed snapshot by preparing all the artefacts of the
/// guest VMs and the yaml file into a zip file in the project folder. This does not include the
/// state file as this is unique to a testbed configuration.
pub async fn run_testbed_snapshot_action(
    state: &State,
    common: &OrchestrationCommon,
) -> anyhow::Result<()> {
    // turn off guests
    tracing::info!("turning off all guests before continuing...");
    let mut guest_stop_futures = Vec::new();
    for (_, guest_data) in state.testbed_guests.0.iter() {
        match &guest_data.guest_type.guest_type {
            GuestType::Libvirt(libvirt) => {
                guest_stop_futures.push(libvirt.destroy_action(common.clone(), guest_data.clone()));
            }
            GuestType::Docker(_) => {}
            GuestType::Android(_) => {
                // TODO once android guests are placed in the artefacts folder
            }
        }
    }
    try_join_all(guest_stop_futures).await?;
    // pull all remote guest images
    let mut pull_image_futures = Vec::new();
    for (guest_name, guest_data) in state.testbed_guests.0.iter() {
        let testbed_host = guest_data.testbed_host.as_ref().context("getting testbed host name in run snapshot action")?;
        match &guest_data.guest_type.guest_type {
            GuestType::Libvirt(libvirt) => {
                if !is_main_testbed(common, testbed_host) {
                    // place remote images back into the artefacts folder, this will overwrite
                    // the original images but we want to preserve state of the current guest
                    // images
                    pull_image_futures.push(libvirt.pull_image_action(common.clone(), guest_data.clone()));
                }
            }
            GuestType::Docker(_) => {
                tracing::info!("Doing nothing for {guest_name}, docker guests currently not supported for testbed snapshot as images come from docker hub");
            }
            GuestType::Android(_) => {
                tracing::info!("Doing nothing for {guest_name}, android guests currently not supported for testbed snapshot");
            }
        }
    }
    try_join_all(pull_image_futures).await?;
    tracing::info!("zipping all contents of project folder minus the state json file.");
    zip_project(common).await?;
    tracing::info!("turning all guests back on...");
    let mut guest_start_futures = Vec::new();
    for (_, guest_data) in state.testbed_guests.0.iter() {
        match &guest_data.guest_type.guest_type {
            GuestType::Libvirt(libvirt) => {
                guest_start_futures.push(libvirt.create_action(common.clone(), guest_data.clone()));
            }
            GuestType::Docker(_) => {}
            GuestType::Android(_) => {
                // TODO once android guests are placed in the artefacts folder
            }
        }
    }
    try_join_all(guest_start_futures).await?;
    Ok(())
}

async fn zip_project(
    common: &OrchestrationCommon,
) -> anyhow::Result<()> {
    // we will zip the project folder and include the following:
    // - artefacts folder with the images, isos and domain xmls
    // - kvm-compose.yaml
    // - all other user files like scripts and data
    // but, we will not include the state file, as that is specific to the current testbed

    // we can assume this command is running from the project folder as the orchestration runner
    // will have changed directory

    let project_name = &common.project_name;
    let state_file = format!("{project_name}-state.json");
    let time_now: DateTime<Utc> = std::time::SystemTime::now().into();
    let project_folder = common.project_working_dir.to_string_lossy().to_string();
    let zip_name = format!("{project_name}-snapshot-{}.zip", time_now.format("%+"));
    let main_host: Vec<&String> = common.testbed_hosts.iter()
        .filter(|(_, h)| h.is_main_host)
        .map(|(n, _)| n)
        .collect();
    // TODO - should we try to find and ignore other snapshot zips to prevent accidentally including

    // recursively zip from current folder "." and exclude -x the state file
    // store the zip contents in /tmp during the zip
    // set the working folder so that we don't get the parent directories (awkward technical detail
    // of zip)
    let cmd = vec!["zip", "-r", &zip_name, ".", "-x", &state_file, "-b", "/tmp"];
    run_testbed_orchestration_command(
        common,
        main_host[0],
        "sudo",
        cmd,
        false,
        Some(project_folder.clone()),
    ).await?;
    Ok(())
}
