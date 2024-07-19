use kvm_compose_schemas::TESTBED_SETTINGS_FOLDER;
use anyhow::{bail, Context};
use clap::Parser;
use serde::{Deserialize, Serialize};
use tokio::fs::File;
use std::fmt;
use std::fmt::Formatter;
use tokio::io::AsyncWriteExt;
use kvm_compose_schemas::settings::SshConfig;

pub mod manage;
pub mod ovn;
pub mod handlers;

/// CLI argument parsing for the testbed server
#[derive(Parser, Debug)]
pub struct TestbedServerArgs {
    #[command(subcommand)]
    pub mode: Option<ServerModeCmd>,
}

/// Options to launch the server
#[derive(Parser, Debug, Clone, Deserialize, Serialize)]
pub enum ServerModeCmd {
    /// Run server in master mode
    Master,
    /// Run server in client mode
    Client(ClientMode),
    /// Wizard for creating the kvm-compose-config file for master mode
    CreateConfig,
}


impl fmt::Display for ServerModeCmd {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(&serde_json::to_string_pretty(&self).unwrap())
            .expect("ServerModeCmd to json via serde failed");
        Ok(())
    }
}

/// Client mode options
#[derive(Parser, Debug, Clone, Deserialize, Serialize)]
pub struct ClientMode {
    // TODO - make this take an IpAddr then add the http:// or even tcp://
    /// The ip address of the master testbed host
    #[clap(long, short)]
    pub master_ip: String,

    /// The network interface to be used to connect to other testbed servers
    #[clap(long, short)]
    pub testbed_interface: String,

}

/// This function will take the CLI arguments to determine the running mode for the testbed server.
/// By default, the server will look for the mode.json in the config folder, otherwise the user can
/// specify the mode manually. By specifying the mode, the default mode will be updated with the
/// settings the user gives to be re-used later. The expectation is that the user is not manually
/// running the server so the systemd process will just look for the mode file and start from there.
/// As there is also the expectation that testbed servers will not keep changing their modes
/// frequently.
pub async fn parse_cli_args(

) -> anyhow::Result<ServerModeCmd> {
    let args = TestbedServerArgs::parse();

    // depending on the start mode, update the file with StartMode
    //  if no start mode given, then just load the pre-existing start mode file
    let mode = if args.mode.is_some() {
        // update the mode config file depending on the mode
        match args.mode.as_ref().unwrap() {
            ServerModeCmd::Master => {
                create_mode_config(ServerModeCmd::Master).await?;
            }
            ServerModeCmd::Client(client) => {
                validate_client_mode_arguments(client)?;
                create_mode_config(ServerModeCmd::Client(client.clone())).await?;
            }
            ServerModeCmd::CreateConfig => {
                // don't save create config as a state
            }
        };
        args.mode.unwrap()
    } else {
        // just load the config file with the mode if it exists
        let path = format!("{TESTBED_SETTINGS_FOLDER}config/mode.json");
        let text = tokio::fs::read_to_string(&path).await?;
        let mode_config: ServerModeCmd = serde_json::from_str(&text)
            .context(format!("Could not load {path}, does not exist. Please run with create-config argument to configure the testbed server."))?;
        // check if config is in create config mode, should not be
        if let ServerModeCmd::CreateConfig = mode_config {
            bail!("server start mode cannot be set to CreateConfig, please update the mode.json")
        }
        mode_config
    };

    Ok(mode)
}

/// Sets the config file to the mode the server should autostart with
async fn create_mode_config(
    mode: ServerModeCmd,
) -> anyhow::Result<()> {
    let mut config = File::create(format!("{TESTBED_SETTINGS_FOLDER}config/mode.json")).await?;
    config.write_all(format!("{mode}").as_bytes()).await?;
    Ok(())
}

/// This function will ask the user for configuration options and then save this in the settings
/// so that in further runs, the user can just start the server in a specific mode.
pub fn create_config_wizard(

) {
    // this needs to run as sudo

    // TODO - do this once the kvmComposeConfig has been cut down once we have testbed cluster

    // TODO - ask the user if they want to default the server to start as master/client and update
    //  the systemd file
    todo!()
}

fn validate_client_mode_arguments(
    _client_mode: &ClientMode,
) -> anyhow::Result<()> {
    // TODO

    Ok(())
}

/// This enum represents the different operations possible on the testbed cluster
#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum ClusterOperation {
    Init,
    Join(SshConfig),
    Leave(SshConfig),
}

