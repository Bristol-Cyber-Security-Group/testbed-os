use std::ops::{Add, Deref, DerefMut};
use std::process::Command;
use anyhow::{bail, Context, Error};
use rexpect::process::{signal, PtyProcess};
use rexpect::ReadUntil;
use rexpect::session::PtySession;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::{mpsc, Mutex};
use virt::connect::Connect;
use kvm_compose_schemas::kvm_compose_yaml::machines::GuestType;
use crate::orchestration::api::OrchestrationLogger;
use crate::orchestration::OrchestrationCommon;
use crate::state::StateTestbedGuest;

/// Enum to define the different states the PTY could be in when we first try to open it. Virsh
/// could either let us open it or complain that there is already a session open.
#[derive(Debug)]
enum PtyInitialState {
    Ok,
    SessionOpen,
}

// these constants are used to determine what state the PTY is in based on the text that it is
// currently showing
const ACTIVE_SESSION: &str = "Active console session exists for this domain";
const SESSION_READY: &str = "(Ctrl + ])";
const LOGIN_USER: &str = " login:";
const LOGIN_PASSWORD: &str = "Password:";
const SHELL: &str = ":~$";

/// Enum to define the different states the PTY could be in during use, after we have passed the
/// initial check to be able to open the PTY. The PTY could be in a few different states, depending
/// on previous usage i.e. on login prompt or already logged in.
#[derive(Debug)]
enum PtyState {
    LoginUser(String),
    LoginPassword(String),
    ShellOpen(String),
}

pub async fn shell_command(
    command: Vec<&str>,
    guest_data: &StateTestbedGuest,
    guest_name_with_project: &String,
    common: &OrchestrationCommon,
    logging_send: &Sender<OrchestrationLogger>,
) -> anyhow::Result<()> {

    logging_send.send(OrchestrationLogger::info(format!("Logging into guest {} pty", guest_name_with_project))).await?;

    let usr_cmd = command.join(" ");

    let virsh_cmd = format!("virsh console {}", guest_name_with_project);

    // retrieve credentials for the guest
    let libvirt_guest = match &guest_data.guest_type.guest_type {
        GuestType::Libvirt(libvirt) => libvirt,
        _ => unreachable!(),
    };
    let username = if libvirt_guest.username.is_some() {
        libvirt_guest.username.as_ref().unwrap().clone()
    } else {
        bail!("guest has not been supplied a username in definition")
    };
    let password = if libvirt_guest.password.is_some() {
        libvirt_guest.password.as_ref().unwrap().clone()
    } else {
        bail!("guest has not been supplied a password in definition")
    };

    // set up a channel to send logging from the command running
    let (cmd_log_sender, mut cmd_log_receiver) = mpsc::channel(16);

    let tty = tokio::task::spawn_blocking(move || {
        // spawn the pty in the expect session
        // TODO - CLI configurable timeout?
        let mut pty = rexpect::spawn(&virsh_cmd, Some(5_000))
            .context("rexpect error")?;

        // we need to know if virsh will let us open the pty or there is already a connection to it
        let intial_state = determine_initial_pty_state(&mut pty)
            .context("getting initial state of tty")?;
        match intial_state {
            PtyInitialState::SessionOpen => bail!("there was already a tty session open to the guest, cannot continue"),
            _ => {}
        }
        // we have a connection
        cmd_log_sender.blocking_send("Successfully opened PTY session to guest".to_string())?;

        // we send a new line command to dismiss the virsh escape character message
        pty.send_line("")?;

        // now we need to check which state the PTY is in, due to possible previous interaction
        // ... we loop until we have reached the shell open state, allowing us to run the command
        loop {
            let pty_state = determine_pty_state(&mut pty)
                .context("getting state of pty")?;
            match pty_state {
                PtyState::LoginUser(prompt_state) => {
                    // cmd_log_sender.blocking_send(format!("debug: {}", prompt_state))?;
                    if prompt_state.contains("Last login:") {
                        // some shells give you a last login, which causes it to match with the
                        // login check, so if this is the case then check again as the next expect
                        // check will then read the rest of the text and match with shell open
                        cmd_log_sender.blocking_send("Hit last login".to_string())?;
                        continue;
                    }
                    cmd_log_sender.blocking_send("PTY at login prompt, sending username".to_string())?;
                    pty.send_line(&username)?;
                }
                PtyState::LoginPassword(_) => {
                    // cmd_log_sender.blocking_send(format!("debug: {}", prompt_state))?;
                    cmd_log_sender.blocking_send("PTY at login prompt, sending password".to_string())?;
                    pty.send_line(&password)?;
                }
                PtyState::ShellOpen(_) => {
                    // cmd_log_sender.blocking_send(format!("debug: {}", prompt_state))?;
                    cmd_log_sender.blocking_send("PTY logged in with shell open".to_string())?;
                    break;
                }
            }
        }

        // the shell is open, we can finally run the command
        cmd_log_sender.blocking_send("Running command on guest".to_string())?;
        pty.send_line(&usr_cmd)?;

        // collect the command result and send to user once it has finished
        let res = pty.exp_string(SHELL)?;
        cmd_log_sender.blocking_send(format!("Command output:\n{}", res))?;

        cmd_log_sender.blocking_send("Sending close command to PTY".to_string())?;
        pty.send("\x1D")?;
        pty.process.kill(signal::SIGKILL)?;

        Ok::<(), anyhow::Error>(())
    });


    // loop to check if the command run has finished or not, but also check to see if there are log
    // messages to print before exiting - the pty will close itself as it has a timeout
    loop {
        if cmd_log_receiver.is_empty() && tty.is_finished() {
            // no more log messages and the pty has stopped running
            let tty_res = tty.await?;
            match tty_res {
                Ok(_) => logging_send.send(OrchestrationLogger::info("PTY closed OK".to_string())).await?,
                Err(err) => bail!(err),
            }
            break;
        }
        // log messages in the queue
        let msg = cmd_log_receiver.recv().await;
        match msg {
            Some(msg) => logging_send.send(OrchestrationLogger::info(msg.to_string())).await?,
            None => {
                // don't handle if the channel has been closed, we must wait for thread to close
                tracing::debug!("the exec command logging channel was unexpectedly closed");
            }
        }
    }

    logging_send.send(OrchestrationLogger::info(format!("Finished running command in guest {} pty", guest_name_with_project))).await?;

    Ok(())
}

/// Depending on whether the virsh console is currently in use or not, will determine whether we can
/// use it for command running. If we see the active console message, we cannot continue.
fn determine_initial_pty_state(
    pty: &mut PtySession,
) -> anyhow::Result<PtyInitialState> {
    let (_, end) = pty.exp_any(vec![
        ReadUntil::String("(Ctrl + ])".to_string()),
        ReadUntil::String("Active console session exists for this domain".to_string()),
    ])?;
    let end_str = end.as_str();
    match end_str {
        ACTIVE_SESSION => Ok(PtyInitialState::SessionOpen),
        SESSION_READY => Ok(PtyInitialState::Ok),
        _ => bail!("the initial tty state could not be determined"),
    }
}

/// Depending on the initial state of the pty, we need to run different commands. This will either
/// mean that we are either in the login prompt for the tty or already logged in due to a previous
/// command on the guest.
fn determine_pty_state(
    pty: &mut PtySession
) -> anyhow::Result<PtyState> {
    // start: text before the 'until' match, end: is the matched string
    let (start, end) = pty.exp_any(vec![
        ReadUntil::String(LOGIN_USER.to_string()),
        ReadUntil::String(LOGIN_PASSWORD.to_string()),
        ReadUntil::String(SHELL.to_string()),
    ])?;
    let end_str = end.as_str();
    match end_str {
        LOGIN_USER => Ok(PtyState::LoginUser(start.add(&end))),
        LOGIN_PASSWORD => Ok(PtyState::LoginPassword(start.add(&end))),
        SHELL => Ok(PtyState::ShellOpen(start.add(&end))),
        _ => bail!("the tty state could not be determined"),
    }
}
