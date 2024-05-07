pub mod android;

use anyhow::{bail, Context};
use tokio::sync::mpsc::Sender;
use kvm_compose_schemas::exec::{ExecCmd, ExecCmdType, TestbedTools};
use kvm_compose_schemas::kvm_compose_yaml::machines::GuestType;
use crate::state::{State, StateTestbedGuest};
use crate::orchestration::{OrchestrationCommon};
use crate::orchestration::api::{OrchestrationLogger};

pub async fn prepare_guest_exec_command(
    project_name: &String,
    exec_cmd: &ExecCmd,
    state: &State,
    orchestration_common: &OrchestrationCommon,
    logging_send: &Sender<OrchestrationLogger>,
) -> anyhow::Result<()> {

    // make sure we use the guest name without the project name internally
    let guest_name = &exec_cmd.guest_name;
    let project_name_hyphen = format!("{}-", &project_name);
    let corrected_guest_name = if guest_name.starts_with(&project_name_hyphen) {
        guest_name.strip_prefix(&project_name_hyphen).unwrap()
    } else {
        guest_name.as_str()
    }.to_string();
    // check if guest exists
    let guest_data_res = state.testbed_guests.0
        .get(&corrected_guest_name)
        .context("Getting guest data to run exec command");
    match guest_data_res {
        Ok(guest_data) => {
            // finally run the command
            run_guest_exec_cmd(
                &corrected_guest_name,
                guest_data,
                &exec_cmd.command_type,
                &state,
                orchestration_common,
                &logging_send,
            ).await?;
        }
        Err(_) => {
            bail!("Could not find guest {guest_name} in the project state");
        }
    }
    Ok(())
}

pub async fn run_guest_exec_cmd(
    guest_name: &String,
    guest_data: &StateTestbedGuest,
    exec_cmd: &ExecCmdType,
    state: &State,
    orchestration_common: &OrchestrationCommon,
    logging_send: &Sender<OrchestrationLogger>,
) -> anyhow::Result<()> {
    check_command_on_guest_type(guest_data, exec_cmd)?;

    // create common so that we can use orchestration commands
    match &exec_cmd {
        ExecCmdType::ShellCommand(command) => {
            tracing::info!("running shell command on guest {guest_name}");
            android::shell_command(&command.command, guest_data, orchestration_common, &logging_send).await?;
        }
        ExecCmdType::Tool(tool) => {
            tracing::info!("running tool on guest {guest_name}");
            let namespace = format!("{}-{}-nmspc", state.project_name, guest_name);
            match &tool.tool {
                TestbedTools::ADB(command) => {
                    tracing::info!("ADB arguments = {:?}", command.command);
                    android::adb_command(&namespace, &command.command, &logging_send).await?;
                }
                TestbedTools::FridaSetup => {
                    tracing::info!("Running frida tools setup commands");
                    android::frida_setup(&namespace, &logging_send).await?;
                }
                TestbedTools::TestPermissions(command) => {
                    tracing::info!("Running permissions tests");
                    android::test_permissions(&namespace, &command.command, &logging_send).await?;
                }
                TestbedTools::TLSIntercept(command) => {
                    tracing::info!("Running TLS interceptor");
                    android::tls_intercept(&namespace, &command.command, &logging_send).await?;
                }
                TestbedTools::TestPrivacy(command) => {
                    tracing::info!("Running all privacy tests");
                    android::test_privacy(&namespace, &command.command, &logging_send).await?;
                }
            }
        }
        ExecCmdType::UserScript(user_script) => {
            if user_script.run_on_master {
                tracing::info!("run on master flag enabled");
            }
            tracing::info!("running user script {:?} on guest {guest_name}", user_script.script);
            bail!("unimplemented");
        }
    }
    Ok(())
}

/// Check if the command is available for the guest type
fn check_command_on_guest_type(
    guest_data: &StateTestbedGuest,
    exec_cmd: &ExecCmdType,
) -> anyhow::Result<()> {
    match exec_cmd {
        ExecCmdType::ShellCommand(_) => {}
        ExecCmdType::Tool(tool) => {
            match tool.tool {
                TestbedTools::ADB(_) => {
                    match guest_data.guest_type.guest_type {
                        GuestType::Android(_) => {}
                        _ => bail!("ADB tool only compatible with android guests"),
                    }
                }
                TestbedTools::FridaSetup => {
                    match guest_data.guest_type.guest_type {
                        GuestType::Android(_) => {}
                        _ => bail!("ADB tool only compatible with android guests"),
                    }
                }
                TestbedTools::TestPermissions(_) => {
                    match guest_data.guest_type.guest_type {
                        GuestType::Android(_) => {}
                        _ => bail!("ADB tool only compatible with android guests"),
                    }
                }
                TestbedTools::TestPrivacy(_) => {
                    match guest_data.guest_type.guest_type {
                        GuestType::Android(_) => {}
                        _ => bail!("ADB tool only compatible with android guests"),
                    }
                }
                TestbedTools::TLSIntercept(_) => {
                    match guest_data.guest_type.guest_type {
                        GuestType::Android(_) => {}
                        _ => bail!("ADB tool only compatible with android guests"),
                    }
                }
            }
        }
        ExecCmdType::UserScript(_) => {}
    }
    Ok(())
}
