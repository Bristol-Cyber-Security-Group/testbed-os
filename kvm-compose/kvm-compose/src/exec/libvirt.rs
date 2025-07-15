use std::ops::{Add};
use std::time::Duration;
use anyhow::{bail, Context, Error};
use rexpect::process::{signal};
use rexpect::ReadUntil;
use rexpect::session::PtySession;
use tokio::sync::mpsc::{Sender};
use tokio::sync::{mpsc};
use console::strip_ansi_codes;
use tokio::task::JoinHandle;
use tokio::time;
use kvm_compose_schemas::exec::ExecCmdFileTransfer;
use kvm_compose_schemas::kvm_compose_yaml::machines::GuestType;
use crate::exec::file_transfer::*;
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
    LoginPassword,
    SudoPassword,
    ShellOpen(String),
}

pub async fn shell_command(
    command: Vec<&str>,
    timeout: u64,
    guest_data: &StateTestbedGuest,
    guest_name_with_project: &String,
    _common: &OrchestrationCommon,
    logging_send: &Sender<OrchestrationLogger>,
    suppress_logging: bool,
) -> anyhow::Result<(String, i32)> {

    if !suppress_logging {
        logging_send.send(OrchestrationLogger::info(format!("Logging into guest {} pty", guest_name_with_project))).await?;
    }
    
    // TODO - here we should determine 
    //  1) what OS the guest is (this can be done with virt-inspector from guestfs-tools)
    //  2) if the guest has qemu-guest-agent, otherwise fall back on serial console port with credentials

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

    // the final line of a shell will usually have the username@hostname, so we will use this to
    // 'expect' at the end of a command - this will not be then included in the output
    let shell_user_host_string = format!("{}@{}{}", &username, guest_name_with_project, SHELL);

    // set up a channel to send logging from the command running
    let (cmd_log_sender, mut cmd_log_receiver) = mpsc::channel(16);

    // get the blocking thread that runs rexpect in a blocking context
    let tty = begin_pty_thread(
        virsh_cmd,
        timeout,
        username,
        password,
        shell_user_host_string,
        usr_cmd,
        cmd_log_sender.clone(),
        true, // we will run the command the first time
    );

    let command_exit_code;
    let command_output;

    // loop to check if the command run has finished or not, but also check to see if there are log
    // messages to print before exiting - the pty will close itself as it has a timeout
    loop {
        // we have to use a tokio select! here because there is a race condition between the tty
        // thread finishing and reporting is_finished()==true and the log receiver waiting for the
        // next message
        tokio::select! {
            msg = cmd_log_receiver.recv() => {
                match msg {
                    Some(msg) => {
                        if !suppress_logging {
                            logging_send.send(OrchestrationLogger::info(msg.to_string())).await?
                        }
                    },
                    None => {
                        // don't handle if the channel has been closed, we must wait for thread to close
                        tracing::debug!("the exec command logging channel was unexpectedly closed");
                    }
                }
            }
            _ = time::sleep(Duration::from_millis(100)) => {
                // this will execute every 100 if the above branch doesn't return
                if cmd_log_receiver.is_empty() && tty.is_finished() {
                    // no more log messages and the pty has stopped running
                    let tty_res = tty.await?;
                    match tty_res {
                        Ok((output, exit_code)) => {
                            if !suppress_logging {
                                logging_send.send(OrchestrationLogger::info("PTY closed OK".to_string())).await?;
                            }
                            command_output = output;
                            command_exit_code = exit_code;
                        },
                        Err(err) => {
                            // the
                            if err.to_string().contains("Timeout Error") {
                                // in case there was a timeout error, we want to re-open the tty and
                                // check again to see if the command was still running

                                // TODO how to get the last output string if it bailed?
                            }
                            bail!(err);
                        }
                    }
                    break;
                }
            }
        }
    }

    // parse the outputs
    let ansi_strip_command_output = strip_ansi_codes(&command_output).to_string();
    let ansi_strip_command_exit_code = if let Some(exit_code) = command_exit_code {
        strip_ansi_codes(&exit_code).to_string()
    } else {
        if !suppress_logging {
            logging_send.send(OrchestrationLogger::error(format!("Could not get exit code, got {command_exit_code:?} instead. Setting to -1"))).await?;
        }
        "-1".to_string()
    };
    // TODO - stripping carriage returns like this is likely to cause a weird edge case, how to avoid?
    //  i.e. can we prevent this ANSI code problem earlier up the chain?
    let ansi_strip_command_output = ansi_strip_command_output.replace("\r", "");
    let ansi_strip_command_exit_code = ansi_strip_command_exit_code.replace("\r", "");

    // make sure exit code was a number
    let maybe_int_exit_code = ansi_strip_command_exit_code.parse::<i32>();
    let parsed_exit_code = match maybe_int_exit_code {
        Ok(ok) => ok,
        Err(_) => bail!("the command did not return an exit code: {:?}", maybe_int_exit_code),
    };

    // the command output will also have the first line as the command input, as a side effect of
    // using expect - we need to remove it as we did for the exit code
    let mut command_output_lines = ansi_strip_command_output.lines();
    command_output_lines.next();
    let mut final_command_output = command_output_lines
        .map(|line| format!("{}\n", line))
        .collect::<String>();
    // remove a trailing \n if there is one
    if final_command_output.ends_with("\n") {
        final_command_output = final_command_output[0..final_command_output.len() - 1].to_string();
    }

    // log the output depending on if the command worked or not
    if !suppress_logging {
        let cmd_output_string = format!("Command output:\n{}", final_command_output);
        let finish_command_string = format!("Finished running command in guest {} pty, with exit code {}", guest_name_with_project, parsed_exit_code);
        if parsed_exit_code != 0 {
            logging_send.send(OrchestrationLogger::error(cmd_output_string)).await?;
            logging_send.send(OrchestrationLogger::error(finish_command_string)).await?;
        } else {
            logging_send.send(OrchestrationLogger::info(cmd_output_string)).await?;
            logging_send.send(OrchestrationLogger::info(finish_command_string)).await?;
        }
    }

    Ok((final_command_output, parsed_exit_code))
}

fn begin_pty_thread(
    virsh_cmd: String,
    timeout: u64,
    username: String,
    password: String,
    shell_user_host_string: String,
    usr_cmd: String,
    cmd_log_sender: Sender<String>,
    run_command: bool,
) -> JoinHandle<Result<(String, Option<String>), Error>> {
    // run the rexpect code in a blocking thread (to prevent locking up the server), while sending
    // logging results in a channel
    // also return the possible exit code to be parsed at the end to be returned to the caller and
    // via logging to the user
    tokio::task::spawn_blocking(move || {

        // get and start a PTY in a usable state
        let mut pty = start_pty(&virsh_cmd, timeout, &cmd_log_sender)?;
        // get the PTY into an open shell state
        pty_state_loop(&mut pty, &username, &password, &shell_user_host_string, &cmd_log_sender)?;

        if run_command {
            // the shell is open, we can finally run the command
            cmd_log_sender.blocking_send(format!("Running command ({usr_cmd}) on guest"))?;
            pty.send_line(&usr_cmd)?;
        }

        // check if there was a password prompt, otherwise get result
        let res = pty_state_loop(&mut pty, &username, &password, &shell_user_host_string, &cmd_log_sender)?;

        // grab the command exit code, but prepend a space to not save it in the history
        cmd_log_sender.blocking_send("Getting command exit code".to_string())?;
        pty.send_line(" echo $?")?;
        let exit_code_res = pty.exp_string(&shell_user_host_string)?;
        // the exit code will include the new terminal line below, so we need to trim that
        let mut exit_code_lines = exit_code_res.lines();
        exit_code_lines.next();
        let exit_code = exit_code_lines.next();

        cmd_log_sender.blocking_send("Sending close command to PTY".to_string())?;
        pty.send("\x1D")?;
        pty.process.kill(signal::SIGKILL)?;

        // return both the output of the command and the exit code
        // need to push the exit code if okay as a string rather than &str
        let exit_code_string = if exit_code.is_some() {
            Some(exit_code.unwrap().to_string())
        } else {
            None
        };
        Ok::<(String, Option<String>), Error>((res, exit_code_string))
    })
}

/// Start a pty session
fn start_pty(
    virsh_cmd: &String,
    timeout: u64,
    cmd_log_sender: &Sender<String>,
) -> anyhow::Result<PtySession> {
    // spawn the pty in the expect session
    let mut pty = rexpect::spawn(&virsh_cmd, Some(timeout))
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

    Ok(pty)
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

/// Loop through the possible known states for the terminal to get past any credential prompts, so 
/// that we can eventually reach the ready state to send commands. 
fn pty_state_loop(
    mut pty: &mut PtySession,
    username: &String,
    password: &String,
    after_command: &String,
    cmd_log_sender: &Sender<String>,
) -> anyhow::Result<String> {
    // now we need to check which state the PTY is in, due to possible previous interaction
    // ... we loop until we have reached the shell open state, allowing us to run the command
    loop {
        let pty_state = determine_pty_state(&mut pty, username, after_command)
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
            PtyState::LoginPassword => {
                // cmd_log_sender.blocking_send(format!("debug: {}", prompt_state))?;
                cmd_log_sender.blocking_send("PTY at login prompt, sending password".to_string())?;
                pty.send_line(&password)?;
            }
            PtyState::ShellOpen(output) => {
                // cmd_log_sender.blocking_send(format!("debug: {}", prompt_state))?;
                cmd_log_sender.blocking_send("PTY shell prompt ready for input".to_string())?;
                return Ok(output);
            }
            PtyState::SudoPassword => {
                // TODO - sudo password
                // currently don't support taking a sudo password from the user, for now send
                // the password for the current user in case that will work, otherwise send the
                // user password until the attempt fails
                cmd_log_sender.blocking_send("PTY requesting a password".to_string())?;
                pty.send_line(&password)?;
            }
        }
    }
}

/// Depending on the initial state of the pty, we need to run different commands. This will either
/// mean that we are either in the login prompt for the tty or already logged in due to a previous
/// command on the guest.
fn determine_pty_state(
    pty: &mut PtySession,
    username: &String,
    after_command: &String,
) -> anyhow::Result<PtyState> {
    // set up the password test prompt
    let pass_prompt = format!("password for {username}:");
    // start: text before the 'until' match, end: is the matched string
    let (start, end) = pty.exp_any(vec![
        ReadUntil::String(LOGIN_USER.to_string()),
        ReadUntil::String(LOGIN_PASSWORD.to_string()),
        // ReadUntil::String(SHELL.to_string()),
        ReadUntil::String(after_command.clone()),
        ReadUntil::String(pass_prompt.clone()),
    ])?;

    let end_str = end.as_str();
    // separate match for strings only known at runtime
    if end_str.eq(pass_prompt.as_str()) {
        return Ok(PtyState::SudoPassword);
    }
    if end_str.eq(after_command) {
        return Ok(PtyState::ShellOpen(start));
    }
    match end_str {
        LOGIN_USER => Ok(PtyState::LoginUser(start.add(&end))),
        LOGIN_PASSWORD => Ok(PtyState::LoginPassword),
        // SHELL => Ok(PtyState::ShellOpen(start)),
        _ => bail!("the tty state could not be determined"),
    }
}

/// This command will work out how to push a file or folder into a guest
pub async fn push(
    transfer: &ExecCmdFileTransfer,
    guest_data: &StateTestbedGuest,
    guest_name_with_project: &String,
    common: &OrchestrationCommon,
    logging_send: &Sender<OrchestrationLogger>,
) -> anyhow::Result<()> {

    // create the temporary iso with the file or folder, then proceed to mount the iso as a CD ROM,
    // move the file into the guest, then unmount the CD ROM, then delete the temporary iso
    let temp_iso = prepare_file_transfer_push(transfer, common, logging_send).await?;
    attach_cdrom_to_guest(guest_name_with_project, &temp_iso, logging_send).await?;
    mount_cdrom_in_guest(guest_name_with_project, guest_data, common, logging_send).await?;
    move_pushed_file(transfer, guest_name_with_project, guest_data, common, logging_send).await?;
    unmount_and_detach_cdrom_from_guest(guest_name_with_project, guest_data, common, logging_send).await?;

    // delete the temp iso
    temp_iso.close()?;

    Ok(())
}

/// This command will work out how to pull a file or folder from a guest
pub async fn pull(
    transfer: &ExecCmdFileTransfer,
    _guest_data: &StateTestbedGuest,
    _guest_name_with_project: &String,
    _common: &OrchestrationCommon,
    logging_send: &Sender<OrchestrationLogger>,
) -> anyhow::Result<()> {
    logging_send.send(OrchestrationLogger::info(format!("debug: {transfer:?}"))).await?;
    bail!("pulling files from guests is currently not supported")
}
