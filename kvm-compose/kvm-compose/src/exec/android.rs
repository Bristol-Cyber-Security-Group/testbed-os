use tokio::time::Duration;
use tokio::process::{Command};
use std::process::Output;
use anyhow::{bail, Context};
use std::path::Path;
use tokio::sync::mpsc::Sender;
use kvm_compose_schemas::kvm_compose_yaml::machines::GuestType;
use crate::orchestration::api::{OrchestrationLogger};
use crate::orchestration::OrchestrationCommon;
use crate::orchestration::ssh::SSHClient;
use crate::state::StateTestbedGuest;

pub async fn shell_command(
    command: &[String],
    guest_data: &StateTestbedGuest,
    common: &OrchestrationCommon,
    logging_send: &Sender<OrchestrationLogger>,
) -> anyhow::Result<()> {
    let cmd: Vec<&str> = command.iter()
        .map(|arg| arg.as_str())
        .collect();
    if cmd.is_empty() {
        bail!("No command was given");
    }
    // assuming we can access the guest through the network
    // TODO - if ~ is given as an argument, clap converts it to the hosts home before continuing
    //  how do we prevent clap from doing this?
    let res = match guest_data.guest_type.guest_type {
        GuestType::Libvirt(_) => {
            SSHClient::run_guest_command(
                common,
                cmd,
                guest_data,
                false,
            ).await
        }
        GuestType::Docker(_) => bail!("shell command (docker exec) not implemented"),
        GuestType::Android(_) => bail!("shell command (ADB shell) not implemented - see command: kvm-compose exec phone tool adb --help"),
    };
    match res {
        Ok(output) => {
            tracing::info!("command result:\n{output}");
            logging_send.send(OrchestrationLogger::info(output)).await?;
        }
        Err(err) => {
            if err.to_string().contains("Permission denied (publickey)") {
                bail!("Not enough permissions to use the SSH key, you may need to run this command as root");
            } else {
                bail!("The command resulted in a non 0 exit code, with error:\n{err:#}");
            }
        }
    }
    Ok(())
}

pub async fn adb_command(
    namespace: &str,
    command: &[String],
    logging_send: &Sender<OrchestrationLogger>,
) -> anyhow::Result<()> {

    let mut args = vec![
        "ip".to_string(),
        "netns".to_string(),
        "exec".to_string(),
        namespace.to_string()
    ];

    args.push("/opt/android-sdk/platform-tools/adb".to_string());
    args.extend_from_slice(command);

    tracing::info!("Running command: sudo {}", args.join(" "));

    let output = Command::new("sudo")
        .args(&args)
        .output()
        .await
        .context("Failed to execute adb command")?;

    if output.status.success() {
        let log = String::from_utf8_lossy(&output.stdout);
        tracing::info!("ADB output: {:?}", String::from_utf8_lossy(&output.stdout));
        logging_send.send(OrchestrationLogger::info(log.to_string())).await?;
    } else {
        bail!("ADB error: {:?}", String::from_utf8_lossy(&output.stderr));
    }


    Ok(())
}

pub async fn frida_setup(
    namespace: &str,
    logging_send: &Sender<OrchestrationLogger>,
) -> anyhow::Result<()> {

    // Install frida server if it doesn't exist
    if !Path::new("/var/lib/testbedos/tools/frida-server-16.1.4-android-x86").exists() {
        tracing::info!("Installing frida server");
        let output = Command::new("sudo")
            .arg("wget")
            .arg("https://github.com/frida/frida/releases/download/16.1.4/frida-server-16.1.4-android-x86.xz")
            .arg("-P")
            .arg("/var/lib/testbedos/tools/")
            .output()
            .await
            .context("Failed to install server")?;

        if !output.status.success() {
            bail!("ADB error: {:?}", String::from_utf8_lossy(&output.stderr));
        }

        Command::new("sudo")
            .arg("unxz")
            .arg("/var/lib/testbedos/tools/frida-server-16.1.4-android-x86.xz")
            .output()
            .await
            .context("Failed to extract server")?;
    }

    // Run adb as root, push frida server to emulator and make it executable
    let res = adb_command(namespace, &["root".to_string()], logging_send).await;
    match res {
        Ok(_) => {}
        Err(e) => {
            if e.to_string().contains("daemon not running; starting now at tcp:") && e.to_string().contains("daemon started successfully") {
                // daemon was already installed previously, ignore error
            } else {
                bail!(e);
            }
        }
    }

    // there seems to be a small race condition here after rooting, we will just add a sleep for now
    // TODO - can we check if the device is rooted with adb before continuing? a sleep is not robust
    tracing::info!("waiting to give a chance for rooting to complete before continuing ...");
    tokio::time::sleep(Duration::from_secs(2)).await;

    adb_command(namespace, &["push".to_string(), "/var/lib/testbedos/tools/frida-server-16.1.4-android-x86".to_string(), "/data/local/tmp".to_string()], logging_send).await?;
    adb_command(namespace, &["shell".to_string(), "chmod".to_string(), "755".to_string(), "/data/local/tmp/frida-server-16.1.4-android-x86".to_string()], logging_send).await?;

    // Added -D to daemonize and -C to ignore crashes, which seems to prevent frida from holding
    // up the terminal so it exits - unclear if this is causing side effects yet
    let res = adb_command(namespace, &["shell".to_string(), "/data/local/tmp/frida-server-16.1.4-android-x86 -D -C".to_string()], logging_send).await;
    match res {
        Ok(_) => {}
        Err(e) => {
            // we want to ignore the address already in use error, otherwise continue with the error
            if !e.to_string().contains("Address already in use") {
                bail!(e);
            } else {
                logging_send.send(OrchestrationLogger::info("Address already in use, continuing".to_string())).await?;
            }
        }
    }

    tracing::info!("frida server now running in the emulator");

    Ok(())
}

pub async fn test_permissions(
    namespace: &str,
    command: &[String],
    logging_send: &Sender<OrchestrationLogger>,
) -> anyhow::Result<()> {

    // Get path of poetry venv
    let output = get_frida_tools_env().await?;

    let venv = String::from_utf8_lossy(&output.stdout);
    let venv_path = format!("{}/bin/python", venv.trim_end());

    tracing::info!("Poetry env is {}", venv_path);

    let mut args = vec![
        "ip".to_string(),
        "netns".to_string(),
        "exec".to_string(),
        namespace.to_string()
    ];

    args.push(venv_path.to_string());
    args.push("/var/lib/testbedos/tools/Frida-Tools/permissions/log-permissions.py".to_string());

    args.extend_from_slice(command);

    tracing::info!("Running command: sudo {}", args.join(" "));

    let output = Command::new("sudo")
        .args(&args)
        .output()
        .await
        .context("Failed to execute log permissions command")?;

    if output.status.success() {
        let log = String::from_utf8_lossy(&output.stdout);
        tracing::info!("output: {:?}", log);
        logging_send.send(OrchestrationLogger::info(log.to_string())).await?;
    } else {
        bail!("error: {:?}", String::from_utf8_lossy(&output.stderr));
    }

    Ok(())
}

pub async fn tls_intercept(
    namespace: &str,
    command: &[String],
    logging_send: &Sender<OrchestrationLogger>,
) -> anyhow::Result<()> {

    // Get path of poetry venv
    let output = get_frida_tools_env().await?;

    let venv = String::from_utf8_lossy(&output.stdout);
    let venv_path = format!("{}/bin/python", venv.trim_end());

    tracing::info!("Poetry env is {}", venv_path);

    let mut args = vec![
        "ip".to_string(),
        "netns".to_string(),
        "exec".to_string(),
        namespace.to_string()
    ];

    args.push(venv_path.to_string());
    args.push("/var/lib/testbedos/tools/Frida-Tools/TLS-intercept/intercept.py".to_string());

    args.extend_from_slice(command);

    tracing::info!("Running command: sudo {}", args.join(" "));

    let output = Command::new("sudo")
        .args(&args)
        .output()
        .await
        .context("Failed to execute intercept command")?;

    if output.status.success() {
        let log = String::from_utf8_lossy(&output.stdout);
        tracing::info!("output: {:?}", log);
        logging_send.send(OrchestrationLogger::info(log.to_string())).await?;
    } else {
        bail!("error: {:?}", String::from_utf8_lossy(&output.stderr));
    }

    Ok(())
}

pub async fn test_privacy(
    namespace: &str,
    command: &[String],
    logging_send: &Sender<OrchestrationLogger>,
) -> anyhow::Result<()> {

    let mut args = vec![
        "ip".to_string(),
        "netns".to_string(),
        "exec".to_string(),
        namespace.to_string()
    ];

    args.push("/var/lib/testbedos/tools/Frida-Tools/test-privacy.sh".to_string());

    args.extend_from_slice(command);

    tracing::info!("Running command: sudo {}", args.join(" "));

    let output = Command::new("sudo")
        .args(&args)
        .output()
        .await
        .context("Failed to execute test privacy command")?;

    if output.status.success() {
        let log = String::from_utf8_lossy(&output.stdout);
        tracing::info!("output: {:?}", log);
        logging_send.send(OrchestrationLogger::info(log.to_string())).await?;
    } else {
        bail!("error: {:?}", String::from_utf8_lossy(&output.stdout));
    }

    Ok(())
}

async fn get_frida_tools_env() -> anyhow::Result<Output> {
    Command::new("sudo")
        .arg("/var/lib/testbedos/tools/frida_tools_venv/bin/poetry")
        .arg("env")
        .arg("info")
        .arg("-p")
        .current_dir("/var/lib/testbedos/tools/Frida-Tools")
        .output()
        .await
        .context("Failed to get python environment for frida tools, is it installed at '/var/lib/testbedos/tools/Frida-Tools'?")
}
