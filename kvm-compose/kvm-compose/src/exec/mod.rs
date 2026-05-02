pub mod android;
pub mod docker;
pub mod libvirt;
pub mod file_transfer;

use std::sync::Arc;
use anyhow::{bail, Context};
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::Mutex;
use kvm_compose_schemas::exec::{ExecCmd, ExecCmdType, TestbedTools};
use kvm_compose_schemas::kvm_compose_yaml::machines::GuestType;
use crate::orchestration::OrchestrationCommon;
use crate::orchestration::api::OrchestrationLogger;
use crate::state::schema::{State, StateTestbedGuest};

/// Before running the exec command, we need to prepare some data and make sure that the guest
/// exists.
pub async fn prepare_guest_exec_command(
    project_name: &String,
    exec_cmd: &ExecCmd,
    state: &State,
    orchestration_common: &OrchestrationCommon,
    logging_send: &Sender<OrchestrationLogger>,
    cancel_token_recv: Arc<Mutex<Receiver<()>>>,
) -> anyhow::Result<bool> {

    logging_send.send(OrchestrationLogger::info(format!("running exec {:?} on {}", exec_cmd.command_type, exec_cmd.guest_name))).await?;

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
            let cmd_res = run_guest_exec_cmd(
                &corrected_guest_name,
                guest_data,
                &exec_cmd.command_type,
                &state,
                orchestration_common,
                &logging_send,
                cancel_token_recv,
            ).await;
            // check command running result
            match cmd_res {
                Ok(success) => {
                    // do nothing with a success for now
                    Ok(success)
                }
                Err(err) => {
                    // propagate the error
                    bail!(err);
                }
            }
        }
        Err(_) => {
            bail!("Could not find guest {guest_name} in the project state");
        }
    }
}

/// Run the exec command on the guest. This function will determine what command to use based on
/// the guest type.
pub async fn run_guest_exec_cmd(
    guest_name: &String,
    guest_data: &StateTestbedGuest,
    exec_cmd: &ExecCmdType,
    _state: &State,
    orchestration_common: &OrchestrationCommon,
    logging_send: &Sender<OrchestrationLogger>,
    cancel_token_recv: Arc<Mutex<Receiver<()>>>,
) -> anyhow::Result<bool> {
    check_command_on_guest_type(guest_data, exec_cmd)?;

    // we know the guest name coming into this function has the project name stripped, but we will
    // interface with the providers to run commands so will need to have the project name prefixed
    let guest_name_with_project = format!("{}-{}", &orchestration_common.project_name, &guest_name);

    // create common so that we can use orchestration commands
    let success = match &exec_cmd {
        ExecCmdType::ShellCommand(command) => {
            tracing::info!("running shell command on guest {guest_name}");
            // make sure there was a command given of at least one word
            let cmd: Vec<&str> = command.command.iter()
                .map(|arg| arg.as_str())
                .collect();
            if cmd.len() == 0 {
                bail!("No command was given");
            }

            let shell_command_result = match &guest_data.guest_type.guest_type {
                GuestType::Libvirt(_) => {
                    libvirt::shell_command(cmd, command.timeout_ms, guest_data, &guest_name_with_project, orchestration_common, &logging_send, command.suppress_logging, false).await?
                }
                GuestType::Docker(_) => {
                    docker::shell_command(cmd, guest_data, &guest_name_with_project, orchestration_common, &logging_send).await?
                }
                GuestType::Android(_) => {
                    android::shell_command(cmd, guest_data, &guest_name_with_project, orchestration_common, &logging_send).await?
                }
            };
            if shell_command_result.1 == 0 {
                true
            } else {
                false
            }
        }
        ExecCmdType::Push(transfer) => {
            tracing::info!("pushing {:?} to guest {guest_name}", &transfer.source_path);
            match &guest_data.guest_type.guest_type {
                GuestType::Libvirt(_) => libvirt::push(transfer, guest_data, &guest_name_with_project, orchestration_common, &logging_send).await?,
                _ => bail!("unsupported guest type"),
            }
            true
        }
        ExecCmdType::Pull(transfer) => {
            tracing::info!("pushing {:?} from guest {guest_name}", &transfer.source_path);
            match &guest_data.guest_type.guest_type {
                GuestType::Libvirt(_) => libvirt::pull(transfer, guest_data, &guest_name_with_project, orchestration_common, &logging_send).await?,
                _ => bail!("unsupported guest type"),
            }
            true
        }
        ExecCmdType::Tool(tool) => {
            tracing::info!("running tool on guest {guest_name_with_project}");
            let namespace = format!("{}-nmspc", guest_name_with_project);
            match &tool.tool {
                TestbedTools::ADB(command) => {
                    tracing::info!("ADB arguments = {:?}", command.command);
                    android::adb_command(&namespace, &command.command, &logging_send, false).await?;
                }
                TestbedTools::FridaSetup => {
                    tracing::info!("Running frida tools setup commands");
                    android::frida_setup(&namespace, &logging_send).await?;
                }
                TestbedTools::InstallApk(apk_file) => {
                    tracing::info!("Installing APK");
                    android::install_apk(&namespace, &apk_file.apk_file_path, &logging_send).await?
                }
                TestbedTools::TestPermissions(command) => {
                    tracing::info!("Running permissions tests");
                    android::test_permissions(&namespace, &command.command, &logging_send).await?;
                }
                TestbedTools::TLSIntercept(command) => {
                    tracing::info!("Running TLS interceptor");
                    android::tls_intercept(&namespace, &command.command, &logging_send, cancel_token_recv).await?;
                }
                TestbedTools::TestPrivacy(command) => {
                    tracing::info!("Running all privacy tests");
                    android::test_privacy(&namespace, &command.command, &logging_send).await?;
                }
            }
            true
        }
    };
    Ok(success)
}

/// Check if the command is available for the guest type
fn check_command_on_guest_type(
    guest_data: &StateTestbedGuest,
    exec_cmd: &ExecCmdType,
) -> anyhow::Result<()> {
    match exec_cmd {
        ExecCmdType::ShellCommand(_) => {}
        ExecCmdType::Push(_) => {
            match guest_data.guest_type.guest_type {
                GuestType::Libvirt(_) => {}
                GuestType::Android(_) => bail!("please use ADB commands for Android instead"),
                _ => bail!("Push command only compatible with Libvirt guests"),
            }
        }
        ExecCmdType::Pull(_) => {
            match guest_data.guest_type.guest_type {
                GuestType::Libvirt(_) => {}
                GuestType::Android(_) => bail!("please use ADB commands for Android instead"),
                _ => bail!("Pull command only compatible with Libvirt guests"),
            }
        }
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
                TestbedTools::InstallApk(_) => {
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
    }
    Ok(())
}
