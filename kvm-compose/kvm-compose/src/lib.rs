use std::os::linux::fs::MetadataExt;
use crate::components::LogicalTestbed;

use anyhow::{Context};
use kvm_compose_schemas::cli_models::{Common};
use kvm_compose_schemas::kvm_compose_yaml::machines::GuestType;
use kvm_compose_schemas::kvm_compose_yaml::Config;
use components::helpers::clones::generate_clone_guests;
use std::path::PathBuf;
use std::string::String;
use nix::unistd::{Gid, Uid};
use kvm_compose_schemas::settings::TestbedClusterConfig;
pub mod assets;
pub mod components;

pub mod server_client;
pub mod state;
pub mod orchestration;
pub mod snapshot;
pub mod exec;
pub mod ovn;
pub mod analysis_tools;

fn format_prj_name(s: &str) -> String {
    // replace awkward characters with "-"
    let new_proj_name = s.replace(&['(', ')', ',', '\"', '.', ';', ':', '\'', ' '][..], "-");
    new_proj_name
}

pub fn get_project_name(project_name: Option<String>) -> anyhow::Result<String> {
    let project_name = match project_name {
        None => {
            let path = std::env::current_dir()?;
            let path_str = path.iter().last().context("getting project name")?
                .to_str().context("converting project name to string")?.to_owned();
            // remove problematic chars
            format_prj_name(&path_str)
        }
        Some(x) => format_prj_name(&x),
    };
    Ok(project_name)
}

/// This assigns a unique port for the serial TTY access. This is only applicable to libvirt guests.
pub fn assign_tcp_tty_ports(config: &mut Config) -> anyhow::Result<()> {
    // TODO - condition if a libvirt guest
    let mut tcp_port = 4555;
    if config.machines.is_some() {
        for machine in config
            .machines
            .as_mut()
            .context("Getting machines from yaml config")?
            .iter_mut()
        {
            match &mut machine.guest_type {
                GuestType::Libvirt(libvirt_guest) => {
                    libvirt_guest.tcp_tty_port = Some(tcp_port);
                    tcp_port += 1;
                }
                GuestType::Docker(_) => {}
                GuestType::Android(_) => {}
            }
        }
    }
    Ok(())
}

/// This is the main bit of code to create the logical testbed. It will expand the yaml file input
/// and prepare all the information in memory, ready for creating artefacts. Note the artefacts are
/// created/invoked elsewhere, this is solely to parse the yaml in combination with the testbed
/// kvm-compose-config.json so that once this logical testbed is ready in memory, anything can be
/// built/inferred from this.
pub async fn parse_config(
    path: String,
    project_name: Option<String>,
    no_ask: bool,
    current_dir: PathBuf,
    force_provisioning: bool,
) -> anyhow::Result<LogicalTestbed> {

    tracing::trace!("current dir = {:?}", current_dir);

    // project default to currect folder - set the path related data
    let project_name = get_project_name(project_name)?;
    tracing::trace!("Project name: {}", project_name);

    // Project input file will be set to either kvm-compose.yaml as default or a new value will
    // have been supplied - Use and load that file. That will trigger parsing of yaml to return a
    // Config struct
    let mut config = Config::load_from_file(path).await?;
    // expand any machines with scaling parameters to include clones in the machine list
    generate_clone_guests(&mut config)?;
    // assign tty ports for the guest
    assign_tcp_tty_ports(&mut config)?;
    // end yaml parsing

    // permissions
    let user_group = get_project_folder_user_group().await?;

    // store dynamic data about the test case to be used later to customise testbed components
    let common = Common {
        config,
        project: project_name,
        no_ask,
        project_working_dir: current_dir,
        kvm_compose_config: TestbedClusterConfig::read().await?,
        force_provisioning,
        fs_user: user_group.0,
        fs_group: user_group.1,
    };

    // now create logical assets for the testbed, that are not yet assigned to any physical infra
    let mut logical_testbed = LogicalTestbed::new(common);
    logical_testbed.process_config()?;

    Ok(logical_testbed)
}

pub async fn get_project_folder_user_group() -> anyhow::Result<(Uid, Gid)> {
    let metadata = tokio::fs::metadata(".").await?;
    let uid = metadata.st_uid();
    let gid = metadata.st_gid();
    let uid = Uid::from(uid);
    let gid = Gid::from(gid);
    Ok((uid, gid))
}