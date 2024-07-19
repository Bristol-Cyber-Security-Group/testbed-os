use std::path::PathBuf;
use anyhow::{bail, Context};
use tokio::process::Command;
use kvm_compose_schemas::kvm_compose_yaml::machines::GuestType;
use kvm_compose_schemas::kvm_compose_yaml::machines::libvirt::LibvirtGuestOptions;
use kvm_compose_schemas::cli_models::Common;
use crate::orchestration::OrchestrationCommon;
use crate::state::{StateTestbedGuest, StateTestbedHost};

/// This is a struct that contains the implementation and management of SSH for the testbed.
/// Although there is the sophisticated SSH crate `russh`, we just want to run remote commands.
/// There is also another create that wraps `russh` in a simple API but it doesn't support ssh
/// tunnelling yet.
/// All we need is to run a remote command, push and pull some files so we will just use a sub
/// process command for now. In the future we can look at using something more sophisticated if
/// necessary.
pub struct SSHClient {}

impl SSHClient {

    /// Run a command on a remote testbed host
    pub async fn run_remote_testbed_command(
        common: &OrchestrationCommon,
        testbed_host: &String,
        remote_cmd: Vec<&str>,
        allow_fail: bool,
        in_background: bool,
    ) -> anyhow::Result<String> {
        let testbed_host_ssh_config = _get_conn_testbed_host(common, testbed_host).await?;
        let output = _run_remote_command(
            &testbed_host_ssh_config.username,
            &testbed_host_ssh_config.ip,
            &testbed_host_ssh_config.ssh_private_key_location,
            remote_cmd,
            allow_fail,
            in_background,
        ).await?;
        Ok(output)
    }

    /// Run a command on a guest that is on the master testbed host
    pub async fn run_guest_command(
        common: &OrchestrationCommon,
        remote_cmd: Vec<&str>,
        machine_config: &StateTestbedGuest,
        in_background: bool,
    ) -> anyhow::Result<String> {
        let guest_name = format!("{}-{}", &common.project_name, &machine_config.guest_type.name);
        // running a guest command depends on the type of guest and the available connection
        match &machine_config.guest_type.guest_type {
            GuestType::Libvirt(libvirt) => {
                // there may be different connection mechanisms available for non cloud init guests
                match libvirt.libvirt_type {
                    LibvirtGuestOptions::CloudImage { .. } => {
                        // we have a testbed ssh public key registered in the guest
                        let key = &common.testbed_guest_shared_config.ssh_private_key_location;
                        let output = _run_remote_command(
                            libvirt.username.as_ref().context("getting testbed username in run guest command")?,
                            &guest_name,
                            key,
                            remote_cmd,
                            false,
                            in_background,
                        ).await?;
                        Ok(output)
                    }
                    LibvirtGuestOptions::ExistingDisk { .. } => unimplemented!(),
                    LibvirtGuestOptions::IsoGuest { .. } => unimplemented!(),
                }
            }
            GuestType::Docker(_) => unimplemented!(),
            GuestType::Android(_) => unimplemented!(),
        }
    }

    /// Run a command on a guest that is on a remote testbed host
    pub async fn run_remote_guest_command(_common: &Common, _testbed_host: &String, _testbed_guest:&String, _remote_cmd: Vec<&str>) {
        todo!()
    }

    /// Push a file to a remote testbed host
    pub async fn push_file_to_remote_testbed(
        common: &OrchestrationCommon,
        testbed_host: String,
        local_src: String,
        remote_dst: String,
        allow_fail: bool,
    ) -> anyhow::Result<()> {
        tracing::info!("pushing {} to remote testbed {}", local_src, testbed_host);

        let testbed_host_ssh_config = _get_conn_testbed_host(common, &testbed_host).await?;

        // check if the file exists on the remote
        // get file name to check for it as remote_dst is a folder
        let file_name = PathBuf::from(&local_src).file_name().context("getting file name in pure file to remote testbed")?.to_os_string();
        let remote_file = format!("{remote_dst}/{file_name:?}"); // TODO - change :? to normal print
        let file_check = _run_remote_command(
            &testbed_host_ssh_config.username,
            &testbed_host_ssh_config.ip,
            &testbed_host_ssh_config.ssh_private_key_location,
            vec!["ls", remote_file.as_str()],
            allow_fail,
            false,
        ).await;
        match file_check {
            Ok(_) => {
                // does exist, depends if we want to overwrite
                if !common.force_provisioning {
                    // dont overwrite
                    tracing::info!("file {} already exists on remote testbed {}, won't overwrite", &remote_file, &testbed_host);
                    return Ok(());
                } else {
                    tracing::info!("file {} already exists on remote testbed {} but force provisioning is true, overwriting", &remote_file, &testbed_host);
                }
            }
            Err(_) => {
                // doesn't exist, continue
            }
        }

        // make sure remote folder exists
        let remote_dst_path = PathBuf::from(remote_dst.clone());
        let remote_dst_folder = remote_dst_path.to_str().context("getting remote destination string in push file to remote testbed")?;
        _run_remote_command(
            &testbed_host_ssh_config.username,
            &testbed_host_ssh_config.ip,
            &testbed_host_ssh_config.ssh_private_key_location,
            vec!["mkdir", "-p", remote_dst_folder],
            allow_fail,
            false,
        ).await?;

        tracing::info!("pushing {} to {} at {}", local_src, remote_dst, testbed_host);

        // push file
        _push_file_to_remote(
            &testbed_host_ssh_config.username,
            &testbed_host_ssh_config.ip,
            &testbed_host_ssh_config.ssh_private_key_location,
            local_src,
            remote_dst.clone(),
            allow_fail,
        ).await?;


        Ok(())
    }
    /// Push a file to a guest that is on the master testbed host
    pub async fn push_file_to_guest(
        common: &OrchestrationCommon,
        local_src: &String,
        remote_dst: &String,
        username: &String,
        hostname: &String,
    ) -> anyhow::Result<()> {
        // TODO - when guest has been given a private key that isn't the default testbed key
        _push_file_to_remote(
            username,
            hostname,
            &common.testbed_guest_shared_config.ssh_private_key_location,
            local_src.clone(),
            remote_dst.clone(),
            false,
        ).await?;

        Ok(())
    }

    /// Push a file to a guest that is on a remote testbed host
    pub async fn push_file_to_remote_guest(_common: &Common, _testbed_host: &String, _testbed_guest:&String, _remote_cmd: Vec<&str>) {
        todo!()
    }

    /// Pull a file from a remote testbed host
    pub async fn pull_file_from_remote_testbed(
        common: &OrchestrationCommon,
        testbed_host: &String,
        local_dst: String,
        remote_src: String,
        allow_fail: bool,
    ) -> anyhow::Result<()> {
        let testbed_host_ssh_config = _get_conn_testbed_host(common, testbed_host).await?;
        _pull_file_from_remote(
            &testbed_host_ssh_config.username,
            &testbed_host_ssh_config.ip,
            &testbed_host_ssh_config.ssh_private_key_location,
            local_dst,
            remote_src,
            allow_fail,
        ).await?;
        Ok(())
    }

    /// Pull a file from a guest on the master testbed host
    pub async fn pull_file_from_guest(_common: &Common, _testbed_guest: &String, _remote_cmd: Vec<&str>) {
        todo!()
    }

    /// Pull a file from a guest on a remote testbed host
    pub async fn pull_file_from_remote_guest(_common: &Common, _testbed_host: &String, _testbed_guest:&String, _remote_cmd: Vec<&str>) {
        todo!()
    }

}

/// Private implementation for running a remote command, see public API in `SSHClient`
async fn _run_remote_command(
    testbed_username: &String,
    testbed_hostname: &String,
    testbed_ssh_key_location: &String,
    command_string: Vec<&str>,
    allow_fail: bool,
    in_background: bool,
) -> anyhow::Result<String> {
    let ssh_address = format!("{testbed_username}@{testbed_hostname}");
    let ssh_opts = _get_ssh_opts();
    let ssh = vec!["-i", testbed_ssh_key_location, &ssh_address];
    let total_command = format!("{:?} {:?} {:?}", ssh, ssh_opts, command_string);
    tracing::debug!("running remote command: ssh {}", total_command);
    if !in_background {
        let sub_process = Command::new("ssh")
            // .stdin(Stdio::null())
            // .stdout(Stdio::null())
            // .stderr(Stdio::null())
            .args(ssh)
            .args(ssh_opts)
            .args(command_string)
            .output()
            .await?;

        // tracing::info!("{:?}", std::str::from_utf8(&sub_process.stdout)?);
        // tracing::info!("{:?}", std::str::from_utf8(&sub_process.stderr)?);
        // tracing::info!("{:?}", &sub_process.status.code());
        // tracing::info!("{:?}", &sub_process.status.success());

        // TODO - consider returning an enum with different results rather than error/ok, this means
        //  the calling function can then decide what to do with type of error as it is more granular
        //  i.e. OK/Error/AllowedError
        //  this could improve the logging

        // if the command failed and not allowing fail
        if !sub_process.status.success() && !allow_fail {
            let std_err = std::str::from_utf8(&sub_process.stderr)?;
            // if !suppress_errors {
            //     tracing::error!("remote command running failed for ({}): {:#}", total_command, std_err);
            // }
            bail!("{}", std_err);
        }
        // command failed but allowed to fail, log the reason
        if !sub_process.status.success() && allow_fail {
            let std_err = std::str::from_utf8(&sub_process.stderr)?;
            let warn_str = format!("remote command running failed but allowed to fail for ({}): error: {:#}", total_command, std_err);
            tracing::warn!("{}", warn_str.trim());
        }
        // return the result as a string in case it is needed
        let std_out = std::str::from_utf8(&sub_process.stdout)?.to_string();
        Ok(std_out)
    } else {
        let _sub_process = Command::new("ssh")
            // .stdin(Stdio::null())
            // .stdout(Stdio::null())
            // .stderr(Stdio::null())
            .args(ssh)
            .args(ssh_opts)
            .args(command_string)
            .spawn()?;
        Ok("executed command in background".to_string())
    }
}

async fn _push_file_to_remote(
    testbed_username: &String,
    testbed_hostname: &String,
    testbed_ssh_key_location: &String,
    local_src: String,
    remote_dst: String,
    allow_fail: bool,
) -> anyhow::Result<()> {

    let rsync_opts = vec!["-av", "-e"];
    let ssh_conn = format!("ssh -i {} -o BatchMode=yes -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/nul", testbed_ssh_key_location);

    // let ssh_conn = vec!["'", "ssh", "-i", &testbed_ssh_key_location, "'"];
    let remote_string = format!("{testbed_username}@{testbed_hostname}:{remote_dst}");
    let total_command = format!("{:?} {:?} {:?} {:?}", rsync_opts, ssh_conn, local_src, remote_string);
    tracing::debug!("pushing file to remote: rsync {total_command}");

    let sub_process = Command::new("rsync")
        .args(rsync_opts)
        .arg(ssh_conn)
        .arg(local_src)
        .arg(remote_string)
        .output()
        .await?;

    // tracing::info!("{:?}", std::str::from_utf8(&sub_process.stdout)?);
    // tracing::info!("{:?}", std::str::from_utf8(&sub_process.stderr)?);

    // if the command failed and not allowing fail
    if !sub_process.status.success() && !allow_fail {
        let std_err = std::str::from_utf8(&sub_process.stderr)?;
        // tracing::error!("remote push command running failed for ({}): {:#}", total_command, std_err);
        bail!("{}", std_err);
    }
    // command failed but allowed to fail, log the reason
    if !sub_process.status.success() && allow_fail {
        let std_err = std::str::from_utf8(&sub_process.stderr)?;
        let warn_str = format!("remote command running failed but allowed to fail for ({}): error: {:#}", total_command, std_err);
        tracing::warn!("{}", warn_str.trim());
    }

    Ok(())
}

async fn _pull_file_from_remote(
    testbed_username: &String,
    testbed_hostname: &String,
    testbed_ssh_key_location: &String,
    local_dst: String,
    remote_src: String,
    allow_fail: bool,
) -> anyhow::Result<()> {
    let rsync_opts = vec!["-av", "-e"];
    let ssh_conn = format!("ssh -i {} -o BatchMode=yes -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/nul", testbed_ssh_key_location);
    let remote_string = format!("{testbed_username}@{testbed_hostname}:{remote_src}");
    let total_command = format!("{:?} {:?} {:?} {:?}", rsync_opts, ssh_conn, remote_string, local_dst);
    tracing::debug!("pulling file from remote: rsync {total_command}");

    let sub_process = Command::new("rsync")
        .args(rsync_opts)
        .arg(ssh_conn)
        .arg(remote_string)
        .arg(local_dst)
        .output()
        .await?;

    // if the command failed and not allowing fail
    if !sub_process.status.success() && !allow_fail {
        let std_err = std::str::from_utf8(&sub_process.stderr)?;
        // tracing::error!("remote push command running failed for ({}): {:#}", total_command, std_err);
        bail!("{}", std_err);
    }
    // command failed but allowed to fail, log the reason
    if !sub_process.status.success() && allow_fail {
        let std_err = std::str::from_utf8(&sub_process.stderr)?;
        let warn_str = format!("remote command running failed but allowed to fail for ({}): error: {:#}", total_command, std_err);
        tracing::warn!("{}", warn_str.trim());
    }

    Ok(())
}

/// Private implementation for getting the connection details for a testbed host
async fn _get_conn_testbed_host<'a>(
    common: &'a OrchestrationCommon,
    testbed_host: &'a String,
) -> anyhow::Result<StateTestbedHost> {
    // look in common and find the connection details for this host
    let testbed_host_ssh_config = common.testbed_hosts.get(testbed_host)
        .context(format!("getting testbed host ssh config for {testbed_host}"))?.clone();
    // config was found
    Ok(testbed_host_ssh_config)
}

fn _get_ssh_opts<'a>() -> Vec<&'a str> {
    // TODO - this is not ideal to ignore the host key, when we have testbed joining scripts we can
    //  then add hostkeys before the user uses the testbed - while we are developing still this is fine..
    vec!["-o", "BatchMode=yes", "-o", "StrictHostKeyChecking=no", "-o", "UserKnownHostsFile=/dev/null"]
}
