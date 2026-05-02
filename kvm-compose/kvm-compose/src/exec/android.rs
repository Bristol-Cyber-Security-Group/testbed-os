use tokio::time::Duration;
use tokio::process::{Command};
use anyhow::{bail, Context};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use command_group::AsyncCommandGroup;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::Mutex;
use crate::orchestration::api::{OrchestrationLogger};
use crate::orchestration::OrchestrationCommon;
use crate::state::schema::StateTestbedGuest;

pub async fn shell_command(
    _command: Vec<&str>,
    _guest_data: &StateTestbedGuest,
    _guest_name_with_project: &String,
    _common: &OrchestrationCommon,
    logging_send: &Sender<OrchestrationLogger>,
) -> anyhow::Result<(String, i32)> {
    let error_str = "shell command (ADB shell) not implemented - see command: kvm-compose exec phone tool adb --help";
    logging_send.send(OrchestrationLogger::error(error_str.to_string())).await?;
    bail!(error_str)
}

pub async fn adb_command(
    namespace: &str,
    command: &Vec<String>,
    logging_send: &Sender<OrchestrationLogger>,
    suppress_output: bool,
) -> anyhow::Result<String> {

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
        tracing::info!("ADB output: {:?}", log);
        if !suppress_output {
            logging_send.send(OrchestrationLogger::info(log.to_string())).await?;
        }
        Ok(log.to_string())
    } else {
        bail!("ADB error: {:?}", String::from_utf8_lossy(&output.stderr));
    }

}

pub async fn install_apk(
    namespace: &str,
    apk_file_path: &PathBuf,
    logging_send: &Sender<OrchestrationLogger>,
) -> anyhow::Result<()> {
    adb_command(namespace, &vec!["start-server".to_string()], &logging_send, false)
        .await
        .context("making sure adb server is running on device")?;

    tracing::info!("Installing APK");
    let apk_file_existing_path = apk_file_path.as_path();
    
    if apk_file_existing_path.exists() {
        
        let args = vec![
            "install".to_string(),
            apk_file_path.to_str().unwrap().to_string(),
        ];

        adb_command(namespace, &args, &logging_send, false)
            .await
            .context("APK installation")?;
        
        tracing::info!("APK installation complete");
        
        Ok(())
    } else {
        bail!("APK file not found");
    }
}


pub async fn frida_setup(
    namespace: &str,
    logging_send: &Sender<OrchestrationLogger>,
) -> anyhow::Result<()> {

    adb_command(namespace, &vec!["start-server".to_string()], &logging_send, false)
        .await
        .context("making sure adb server is running on device")?;

    // we need to know if the emulator is x86 or x86_64, we can use an adb command to do this
    let res = adb_command(
        namespace,
        &vec!["shell".to_string(), "getprop".to_string(), "ro.product.cpu.abi".to_string()],
        &logging_send,
        true,
    ).await;
    let abi = match res {
        Ok(ok) => ok,
        Err(e) => bail!("could not determine the abi version for the emulator: {e:#}"),
    };

    // remove whitespaces and newlines
    let abi = abi.trim().to_string();

    // Install frida server if it doesn't exist
    if !Path::new(&format!("/var/lib/testbedos/tools/frida-server-17.2.15-android-{abi}")).exists() {
        tracing::info!("Installing frida server");
        let output = Command::new("sudo")
            .arg("wget")
            .arg(format!("https://github.com/frida/frida/releases/download/17.2.15/frida-server-17.2.15-android-{abi}.xz"))
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
            .arg(format!("/var/lib/testbedos/tools/frida-server-17.2.15-android-{abi}.xz"))
            .output()
            .await
            .context("Failed to extract server")?;
    }

    // Run adb as root, push frida server to emulator and make it executable
    let res = adb_command(namespace, &vec!["root".to_string()], &logging_send, false).await;
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

    adb_command(namespace, &vec!["push".to_string(), format!("/var/lib/testbedos/tools/frida-server-17.2.15-android-{abi}"), "/data/local/tmp".to_string()], &logging_send, false).await?;
    adb_command(namespace, &vec!["shell".to_string(), "chmod".to_string(), "755".to_string(), format!("/data/local/tmp/frida-server-17.2.15-android-{abi}")], &logging_send, false).await?;

    // Added -D to daemonize and -C to ignore crashes, which seems to prevent frida from holding
    // up the terminal so it exits - unclear if this is causing side effects yet
    let res = adb_command(namespace, &vec!["shell".to_string(), format!("/data/local/tmp/frida-server-17.2.15-android-{abi} -D -C")], &logging_send, false).await;
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
    command: &Vec<String>,
    logging_send: &Sender<OrchestrationLogger>,
) -> anyhow::Result<()> {

    let venv_path = format!("/var/lib/testbedos/tools/frida_tools_venv/bin/python");

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
    command: &Vec<String>,
    logging_send: &Sender<OrchestrationLogger>,
    cancel_token_recv: Arc<Mutex<Receiver<()>>>,
) -> anyhow::Result<()> {

    adb_command(namespace, &vec!["start-server".to_string()], &logging_send, false)
        .await
        .context("making sure adb server is running on device")?;

    let venv_path = format!("/var/lib/testbedos/tools/frida_tools_venv/bin/python");

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

    // TODO - how to fix relative paths given to the CLI/GUI and then what the script sees, so
    //  currently a relative path will try to put the output in the Frida-Tools folder

    let mut child = Command::new("sudo")
        .args(&args)
        .current_dir("/var/lib/testbedos/tools/Frida-Tools")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .group_spawn()
        .context("Spawning tls intercept command")?;

    let inner = child.inner();

    let stdout = inner.stdout.take().context("Child did not have stdout")?;
    let stderr = inner.stderr.take().context("Child did not have stderr")?;
    let mut stdout_reader = BufReader::new(stdout).lines();
    let mut stderr_reader = BufReader::new(stderr).lines();

    let pid = child.id().context("getting tls intercept pid")?;
    tracing::info!("Got tls intercept pid {}", pid);

    let mut recv_lock = cancel_token_recv.lock().await;

    logging_send.send(OrchestrationLogger::info("The following is tls-interceptor logs ========".to_string())).await?;

    // loop here on the tokio select! as we will be sending back to the client the logging from
    loop {
        tokio::select! {
            output = child.wait() => {
                // this will run if the command exits by itself
                let status = output?;
                if status.success() {
                    logging_send.send(OrchestrationLogger::info("tls-interceptor exited successfully".to_string())).await?;
                    break;
                } else {
                    bail!("tls-interceptor did not exit successfully");
                }
            }
            cancel = recv_lock.recv() => {
                // this will run if a cancel token is received
                tracing::info!("received cancel token in tls intercept");
                if let Some(_) = cancel {
                    let _ = child.kill().await?;
                    break;
                }
            }
            // the following two branches are for the live logging from the command
            stdout_line = stdout_reader.next_line() => {
                match stdout_line {
                    Ok(Some(line)) => logging_send.send(OrchestrationLogger::info(line)).await?,
                    Ok(None) => {},
                    Err(err) => logging_send.send(OrchestrationLogger::error(err.to_string())).await?,
                }
            }
            stderr_line = stderr_reader.next_line() => {
                match stderr_line {
                    Ok(Some(line)) => logging_send.send(OrchestrationLogger::error(line)).await?,
                    Ok(None) => {},
                    Err(err) => logging_send.send(OrchestrationLogger::error(err.to_string())).await?,
                }
            }
        }
    }

    logging_send.send(OrchestrationLogger::info("End of tls-interceptor logging ========".to_string())).await?;

    Ok(())
}

pub async fn test_privacy(
    namespace: &str,
    command: &Vec<String>,
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
