use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::Stdio;
use anyhow::{bail, Context};
use async_trait::async_trait;
use tokio::process::Command;
use tokio::io::AsyncReadExt;
use futures_util::stream::{SplitSink, SplitStream};
use nix::unistd::{Gid, Uid};
use serde::{Deserialize, Serialize};
use tokio::net::TcpStream;
use tokio::sync::mpsc::{Sender};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
use tokio_tungstenite::tungstenite::Message;
use kvm_compose_schemas::deployment_models::Deployment;
use kvm_compose_schemas::settings::TestbedClusterConfig;
use crate::components::LogicalTestbed;
use crate::orchestration::api::OrchestrationProtocol;
use crate::orchestration::ssh::SSHClient;
use crate::parse_config;
use crate::state::{State, StateNetwork, StateTestbedGuest, StateTestbedGuestList, StateTestbedGuestSharedConfig, StateTestbedHost};

pub mod ssh;
pub mod orchestrator;
pub mod api;
pub mod websocket;

// type aliases for the split stream for the websocket connection to make function signiatures smaller
pub type WebsocketSender = SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>;
pub type WebsocketReceiver = SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>;

/// This is a minimal set of data required for all implementations of `OrchestrationTask` to be
/// able to fully run orchestration. This includes any testbed connection details.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OrchestrationCommon {
    pub testbed_hosts: BTreeMap<String, StateTestbedHost>,
    pub testbed_guest_shared_config: StateTestbedGuestSharedConfig,
    pub testbed_url: String,
    pub project_name: String,
    pub project_working_dir: PathBuf,
    pub force_provisioning: bool,
    pub force_rerun_scripts: bool,
    pub reapply_acl: bool,
    pub kvm_compose_config: TestbedClusterConfig,
    pub network: StateNetwork,
    pub fs_user: u32, //Uid,
    pub fs_group: u32, //Gid,
}

impl OrchestrationCommon {
    pub fn apply_user_file_perms(&self, path_buf: &PathBuf) -> anyhow::Result<()> {
        nix::unistd::chown(path_buf, Some(Uid::from_raw(self.fs_user)), Some(Gid::from_raw(self.fs_group)))?;
        Ok(())
    }

    pub fn get_master(&self) -> anyhow::Result<String> {
        for (tb_name, tb_config) in &self.testbed_hosts {
            if tb_config.is_master_host {
                return Ok(tb_name.clone())
            }
        }
        bail!("could not find master")
    }
}

impl Default for OrchestrationCommon {
    fn default() -> Self {
        Self {
            testbed_hosts: Default::default(),
            testbed_guest_shared_config: Default::default(),
            testbed_url: "localhost:3355".to_string(),
            project_name: "".to_string(),
            project_working_dir: Default::default(),
            force_provisioning: false,
            force_rerun_scripts: false,
            reapply_acl: false,
            kvm_compose_config: Default::default(),
            network: Default::default(),
            fs_user: Uid::from(0).as_raw(),
            fs_group: Gid::from(0).as_raw(),
        }
    }
}

#[async_trait]
pub trait OrchestrationTask {
    // These two are for creating resources all in one get
    async fn create_action(&self, common: &OrchestrationCommon) -> anyhow::Result<()>;
    async fn destroy_action(&self, common: &OrchestrationCommon) -> anyhow::Result<()>;

    // These two are for the client to request the server to create
    async fn request_create_action(&self, common: &OrchestrationCommon, sender: &mut Sender<OrchestrationProtocol>) -> anyhow::Result<()>;
    async fn request_destroy_action(&self, common: &OrchestrationCommon, sender: &mut Sender<OrchestrationProtocol>) -> anyhow::Result<()>;
}

/// Special trait for guests that need some further preparation before being deployed with the
/// `OrchestrationTask` functions. Due to the schema for guests being based on the yaml schema,
/// we need to also provide the individual guest data in the enum `GuestType` information about
/// itself. This is necessary as each `GuestType` has different requirements for deployment and
/// certain things need to be run in a specific order (see the traits functions) hence this
/// workaround is needed.
#[async_trait]
pub trait OrchestrationGuestTask {
    // TODO - since were not using parallel tasks, we can revert back to just using references and remove usage of .clone()
    async fn setup_image_action(&self, common: OrchestrationCommon, machine_config: StateTestbedGuest) -> anyhow::Result<()>;
    async fn push_image_action(&self, common: OrchestrationCommon, machine_config: StateTestbedGuest) -> anyhow::Result<()>;
    async fn pull_image_action(&self, common: OrchestrationCommon, machine_config: StateTestbedGuest) -> anyhow::Result<()>;
    async fn rebase_image_action(&self, common: OrchestrationCommon, machine_config: StateTestbedGuest, guest_list: StateTestbedGuestList) -> anyhow::Result<()>;
    async fn create_action(&self, common: OrchestrationCommon, machine_config: StateTestbedGuest) -> anyhow::Result<()>;
    async fn setup_action(&self, common: OrchestrationCommon, machine_config: StateTestbedGuest) -> anyhow::Result<()>;
    async fn run_action(&self, common: OrchestrationCommon, machine_config: StateTestbedGuest) -> anyhow::Result<()>;
    async fn destroy_action(&self, common: OrchestrationCommon, machine_config: StateTestbedGuest) -> anyhow::Result<()>;
    async fn is_up(&self, common: OrchestrationCommon, machine_config: StateTestbedGuest) -> anyhow::Result<bool>;
}

/// Helper method to run sub commands since there is a lot of boilerplate. Also due to the need to
/// separate the initial command from the arguments. See callers as they fix the `allow_fail` opt.
/// This is the private implementation, see public wrappers `run_subprocess_command`
/// and `run_subprocess_command_allow_fail`
/// There is a `starting_command` as the sub process command needs you to specify the first command
/// explicitly, then you can arguments as a collection.
async fn _run_subprocess_command(
    starting_command: &str, // almost always sudo
    command_string: Vec<&str>,
    allow_fail: bool,
    in_background: bool,
    working_dir: Option<String>,
) -> anyhow::Result<String> {
    // TODO - pretty print this so it doesnt look like a vector of strings
    tracing::debug!("running command: {:?}", command_string);
    if !in_background {

        // apply working_dir if is Some
        let sub_process = if working_dir.is_some() {
            Command::new(starting_command)
                .args(&command_string)
                .current_dir(working_dir.unwrap())
                .output()
                .await?
        } else {
            Command::new(starting_command)
                .args(&command_string)
                .output()
                .await?
        };

        // if the command failed and not allowing fail
        if !sub_process.status.success() && !allow_fail {
            let std_err = std::str::from_utf8(&sub_process.stderr)?;
            // tracing::error!("command running failed for command ({:?}), error: {:#}", &command_string, std_err);
            bail!("{}", std_err);
        }
        // command failed but allowed to fail, log the reason
        if !sub_process.status.success() && allow_fail {
            let std_err = std::str::from_utf8(&sub_process.stderr)?;
            let err_string = format!("command running failed but allowed to fail for command ({:?}), error: {:#}", &command_string, std_err);
            tracing::warn!("{}", err_string.trim());
        }
        // return the result as a string in case it is needed
        let std_out = std::str::from_utf8(&sub_process.stdout)?.to_string();
        Ok(std_out)
    } else {
        tracing::debug!("running command in background");
        // ideally we hold onto the handle for the emulator as it will become a zombie process once
        // it exits but "has not been reaped" by the parent process

        // we need to pipe everything to null or it wont work
        let sub_process = Command::new(starting_command)
            .args(&command_string)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        // check if the spawned background command didn't fail
        if sub_process.stderr.is_some() {
            // since reading the stderr to a mutable string blocks the thread, we need to just give
            // it a fixed length buffer so that it stops reading and wont block. unclear if this
            // will fail if the output from the emulator is smaller than 100 bytes, so can tweak
            // this value as the expected error is for example as 100 bytes
            //
            // ERROR   | Unknown AVD name [avd-phone], use -list-avds to see valid list.\nERROR   | HOME is defined
            //
            // we just want to see "Unknown AVD name [avd-phone]"
            let mut buf = [0; 200];
            sub_process.stderr.context("getting stderr in run subprocess cmd")?.read_exact(&mut buf).await?;
            let parsed_err_string = std::str::from_utf8(&buf[..])?;
            // we may want to capture different errors as we encounter them
            tracing::debug!("stderr from AVD emulator background execution: {:?}", parsed_err_string);
            if parsed_err_string.contains("Unknown AVD name") {
                bail!("Partial error captured from AVD background process: {parsed_err_string:?}");
            }
            if parsed_err_string.contains("ERROR |") {
                tracing::error!("{parsed_err_string}");
            }
        }
        if sub_process.stdout.is_some() {
            let mut buf = [0; 200];
            sub_process.stdout.context("getting stdout in run subprocess cmd")?.read_exact(&mut buf).await?;
            let parsed_err_string = std::str::from_utf8(&buf[..])?;
            tracing::debug!("stdout from AVD emulator background execution: {:?}", parsed_err_string);
            if parsed_err_string.contains("ERROR |") {
                tracing::error!("{parsed_err_string}");
            }
        }

        // let res = sub_process.wait_with_output().await?;
        // tracing::error!("{:#}", std::str::from_utf8(&res.stderr)?);
        // tracing::error!("{:#}", std::str::from_utf8(&res.stdout)?);
        Ok("executed command in background".to_string())
    }
}

/// Public wrapper for `_run_subprocess_command` with hardcoded allow fail setting.
pub async fn run_subprocess_command_allow_fail(
    starting_command: &str,
    command_string: Vec<&str>,
    in_background: bool,
    working_dir: Option<String>,
) -> anyhow::Result<String> {
    _run_subprocess_command(starting_command, command_string, true, in_background, working_dir).await
}

/// Public wrapper for `_run_subprocess_command` with hardcoded don't allow fail setting.
pub async fn run_subprocess_command(
    starting_command: &str,
    command_string: Vec<&str>,
    in_background: bool,
    working_dir: Option<String>,
) -> anyhow::Result<String> {
    _run_subprocess_command(starting_command, command_string, false, in_background, working_dir).await
}

/// Checks if the testbed host input is the master host
pub fn is_master(
    common: &OrchestrationCommon,
    testbed_host: &String,
) -> bool {
    for (host, config) in &common.testbed_hosts {
        if host.eq(testbed_host) {
            // match
            if config.is_master_host {
                return true;
            }
        }
    }
    false
}

/// Wrapper to work out if a command needs to be run locally or on a remote testbed host
pub async fn run_testbed_orchestration_command(
    common: &OrchestrationCommon,
    testbed_host: &String,
    starting_command: &str,
    arguments: Vec<&str>,
    in_background: bool,
    working_dir: Option<String>,
) -> anyhow::Result<String> {
    _run_testbed_orchestration_command(common, testbed_host, starting_command, arguments, false, in_background, working_dir).await
}

/// Wrapper to work out if a command needs to be run locally or on a remote testbed host, allow fail
pub async fn run_testbed_orchestration_command_allow_fail(
    common: &OrchestrationCommon,
    testbed_host: &String,
    starting_command: &str,
    arguments: Vec<&str>,
    in_background: bool,
    working_dir: Option<String>,
) -> anyhow::Result<String> {
    _run_testbed_orchestration_command(common, testbed_host, starting_command, arguments, true, in_background, working_dir).await
}

async fn _run_testbed_orchestration_command(
    common: &OrchestrationCommon,
    testbed_host: &String,
    starting_command: &str,
    arguments: Vec<&str>,
    allow_fail: bool,
    in_background: bool,
    working_dir: Option<String>,
) -> anyhow::Result<String> {
    if is_master(common, testbed_host) {
        // running on master host
        if allow_fail {
            let output = run_subprocess_command_allow_fail(
                starting_command,
                arguments,
                in_background,
                working_dir).await?;
            return Ok(output);
        } else {
            let output = run_subprocess_command(
                starting_command,
                arguments,
                in_background,
                working_dir).await?;
            return Ok(output);
        }
    } else {
        // running on remote host
        // merge starting command and command
        let mut a = vec![starting_command];
        let mut b = arguments;
        a.append(&mut b);
        let output = SSHClient::run_remote_testbed_command(
            common,
            testbed_host,
            a,
            allow_fail,
            in_background,
        ).await?;
        return Ok(output);
    }

}

pub async fn read_previous_state(
    project_location: PathBuf,
    project_name: &String,
) -> anyhow::Result<State> {
    let mut state_path = project_location.clone();
    state_path.push(format!("{}-state.json", project_name));
    // tracing::info!("reading existing state at {state_path:?}");
    // get state from file, should destroy only what is up
    let text = tokio::fs::read_to_string(&state_path).await?;
    let old_state: State = serde_json::from_str(&text)?;
    Ok(old_state)
}

pub async fn create_logical_testbed(
    yaml_path: &String,
    deployment: &Deployment,
    project_location: &PathBuf,
    force_provisioning: bool,
) -> anyhow::Result<LogicalTestbed> {
    let logical_testbed = parse_config(
        yaml_path.clone(),
        Some(deployment.name.clone()),
        true,
        project_location.clone(),
        force_provisioning,
    ).await.context("Failed to parse the yaml config and create a logical testbed")?;
    Ok(logical_testbed)
}

pub async fn create_remote_project_folders(
    common: &OrchestrationCommon,
) -> anyhow::Result<()> {
    tracing::info!("creating remote project folders");

    for (host_name, host_config) in &common.testbed_hosts {
        if !host_config.is_master_host {
            // not master, create testbed projects folder
            let folder = format!(
                "/home/{}/testbed-projects/{}/artefacts/",
                host_config.username,
                common.project_name,
            );
            run_testbed_orchestration_command(
                common,
                host_name,
                "mkdir",
                vec!["-p", &folder],
                false,
                None).await?;
            // TODO - set user:group on remote folders
        }
    }

    Ok(())
}

pub async fn destroy_remote_project_folders(
    common: &OrchestrationCommon,
) -> anyhow::Result<()> {
    tracing::info!("destroying remote project folders");

    for (host_name, host_config) in &common.testbed_hosts {
        if !host_config.is_master_host {
            // not master, create testbed projects folder
            let folder = format!(
                "/home/{}/testbed-projects/{}/",
                host_config.username,
                common.project_name,
            );
            run_testbed_orchestration_command(
                common,
                host_name,
                "sudo",
                vec!["rm", "-rf", &folder],
                false,
                None).await?;
        }
    }

    Ok(())
}
