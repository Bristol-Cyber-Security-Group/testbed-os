use clap::{Args, Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use crate::exec::ExecCmd;
use nix::unistd::{Gid, Uid};
use crate::kvm_compose_yaml::Config;
use crate::settings::TestbedClusterConfig;
use crate::TESTBED_SETTINGS_FOLDER;

#[derive(Parser)]
#[command(version = "1.0", author = "Bristol Cyber Security Group (BCSG)")]
pub struct Opts {
    #[arg(long, default_value = "kvm-compose.yaml", help = "Configuration file")]
    pub input: String,
    #[arg(long, help = "Defaults to the current folder name")]
    pub project_name: Option<String>,
    #[arg(short, long)]
    pub verbosity: Option<String>,
    #[command(subcommand)]
    pub sub_command: SubCommand,
    #[arg(
    long,
    default_value = "http://localhost:3355/",
    help = "Specify the URL to the testbed server"
    )]
    pub server_connection: String,
}

#[derive(Subcommand, Debug, Deserialize, Serialize)]
pub enum SubCommand {
    #[command(about = "Create all artefacts for virtual devices in the current configuration")]
    GenerateArtefacts,
    #[command(about = "Destroy all artefacts for the current configuration")]
    ClearArtefacts,
    #[command(about = "List supported cloud images")]
    CloudImages,
    // #[command(about = "Generates the desired state from the kvm-compose.yaml file without testbed orchestration")]
    // GenerateState TODO
    #[command(about = "Setup kvm compose config")]
    SetupConfig,
    #[command(about = "Control deployments on the testbed server")]
    Deployment(DeploymentCmd),
    #[command(about = "Deploy the test case")]
    Up(UpCmd),
    #[command(about = "Undeploy the test case")]
    Down,
    #[command(about = "Snapshot guest images (guests must be switched off)")]
    Snapshot(SnapshotCmd),
    #[command(about = "Analysis tools")]
    AnalysisTools(AnalysisToolsCmd),
    #[command(about = "Prepare all artefacts in deployment to be shared and used in another testbed")]
    TestbedSnapshot(TestbedSnapshotCmd),
    #[command(about = "Execute a command against a guest")]
    Exec(ExecCmd),
}

impl SubCommand {
    pub fn name(&self) -> String {
        match &self {
            SubCommand::GenerateArtefacts => "generate artefacts".into(),
            SubCommand::ClearArtefacts => "clear artefacts".into(),
            SubCommand::CloudImages => "cloud images".into(),
            SubCommand::SetupConfig => "setup config".into(),
            SubCommand::Deployment(_) => "deployment".into(),
            SubCommand::Up(_) => "up".into(),
            SubCommand::Down => "down".into(),
            SubCommand::Snapshot(_) => "snapshot".into(),
            SubCommand::AnalysisTools(_) => "analysis tools".into(),
            SubCommand::TestbedSnapshot(_) => "testbed snapshot".into(),
            SubCommand::Exec(_) => "exec".into(),
        }
    }
}

/// Contains information about the testbed at runtime specific to the project being deployed
pub struct Common {
    // pub hypervisor: Arc<RwLock<Box<Connect>>>,
    pub config: Config,
    pub project: String,
    pub no_ask: bool,
    pub project_working_dir: PathBuf,
    pub kvm_compose_config: TestbedClusterConfig,
    pub force_provisioning: bool,
    // record the user and group of the project directory
    pub fs_user: Uid,
    pub fs_group: Gid,
}

impl Common {
    pub async fn storage_location() -> anyhow::Result<PathBuf> {
        let p = PathBuf::from(TESTBED_SETTINGS_FOLDER);
        // p.push(".kvm-compose");
        tokio::fs::create_dir_all(&p).await?;
        Ok(p)
    }

    pub fn prepend_project<T: AsRef<str>>(&self, t: T) -> String {
        format!("{}-{}", self.project, t.as_ref())
    }

    /// Convert the file to the project folder's user:group permissions so that it is not owned
    /// by root, as the server will leave everything as root:root.
    pub fn apply_user_file_perms(&self, path_buf: &PathBuf) -> anyhow::Result<()> {
        nix::unistd::chown(path_buf, Some(self.fs_user), Some(self.fs_group))?;
        Ok(())
    }
}

#[derive(Parser, Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct UpCmd {
    #[clap(long, short, action, help = "Force regenerate guest images")]
    pub provision: bool,
    #[clap(long, short, action, help = "Force rerunning use specified guest setup scripts")]
    pub rerun_scripts: bool,
}

/// Snapshot testbed command to provide the minimal required artefacts for sharing and reproducing
/// a deployment on a different testbed
#[derive(Parser, Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub struct TestbedSnapshotCmd {
    #[clap(long, short, action, help = "Also create snapshots of guests")]
    pub snapshot_guests: bool,
}

/// Snapshot sub command to control snapshots from the CLI
#[derive(Parser, Debug, Deserialize, Serialize)]
pub struct SnapshotCmd {
    #[command(subcommand)]
    pub sub_command: SnapshotSubCommand,
}

/// Snapshot sub command to control snapshots from the CLI.
#[derive(Parser, Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SnapshotSubCommand {
    /// Create a snapshot
    #[command(about = "Create a guest snapshot")]
    Create(SnapshotInfo),
    /// Destroy a snapshot
    #[command(about = "Delete a snapshot for a guest or delete all snapshots for the guest, if any")]
    Delete {
        #[arg(short, long, required_unless_present = "all", help = "Guest name")]
        name: Option<String>,
        #[arg(short, long, required_unless_present = "all", help = "Snapshot id")]
        snapshot: Option<String>,
        #[arg(short, long, conflicts_with_all = &["snapshot"], help = "Apply to all snapshots")]
        all: bool,
    },
    /// Get info on a snapshot
    #[command(about = "Get information about a guest and it's snapshots")]
    Info {
        #[arg(short, long, help = "Guest name")]
        name: String,
    },
    /// List all snapshots
    #[command(about = "List guest snapshots")]
    List(GuestSnapshot),
    /// Restore a snapshot for a guest
    #[command(about = "Restore guest from snapshot or all guests from latest snapshot, if any")]
    Restore(SnapshotInfo),
}

impl SnapshotSubCommand {
    pub fn name(&self) -> String {
        match self {
            SnapshotSubCommand::Create(c) => {
                let msg = if c.all {
                    "all guests".to_string()
                } else {
                    format!("name = {} snapshot id = {}", c.name.as_ref().unwrap(), c.snapshot.as_ref().unwrap())
                };
                format!("Create {msg}")
            }
            SnapshotSubCommand::Delete { name, snapshot, all } => {
                let msg = if *all {
                    "all".to_string()
                } else {
                    format!("name = {} snapshot id = {}", name.as_ref().unwrap(), snapshot.as_ref().unwrap())
                };
                format!("Delete {msg}")
            }
            SnapshotSubCommand::Info { name } => {
                format!("Info on {name}")
            }
            SnapshotSubCommand::List(guest_snapshot) => {
                let msg = if guest_snapshot.all {
                    "all".to_string()
                } else {
                    format!("name = {}", guest_snapshot.name.as_ref().unwrap())
                };
                format!("List {}", msg)
            }
            SnapshotSubCommand::Restore(restore) => {
                let msg = if restore.all {
                    "all".to_string()
                } else {
                    format!("name = {} snapshot id = {}", restore.name.as_ref().unwrap(), restore.snapshot.as_ref().unwrap())
                };
                format!("Restore {}", msg)
            }
        }
    }
}

/// This is the name of the guest and the snapshot name for snapshot commands
#[derive(Parser, Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct SnapshotInfo {
    #[arg(short, long, required_unless_present = "all", help = "Guest name")]
    pub name: Option<String>,
    #[arg(short, long, required_unless_present = "all", help = "Snapshot id")]
    pub snapshot: Option<String>,
    #[arg(short, long, conflicts_with_all = &["name", "snapshot"], help = "Apply to all guests")]
    pub all: bool,
}

/// This is the name of the guest for snapshot commands, if --all is given then --name cannot be
/// given. If name --all is not given then --name must be given.
#[derive(Args, Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct GuestSnapshot {
    #[arg(short, long, required_unless_present = "all", help = "Guest name")]
    pub name: Option<String>,
    #[arg(short, long, conflicts_with = "name", help = "Apply to all guests")]
    pub all: bool,
}

/// Deployment sub command to control deployments from the CLI
#[derive(Parser, Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub struct DeploymentCmd {
    #[command(subcommand)]
    pub sub_command: DeploymentSubCommand,
}

/// Deployment sub command to control deployments from the CLI
#[derive(Parser, Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum DeploymentSubCommand {
    /// Create a deployment
    Create(DeploymentName),
    /// Destroy a deployment
    Destroy(DeploymentName),
    /// List all deployments
    List,
    // Update(DeploymentActionSubCommand),
    Info(DeploymentName),
    /// Set the state of a deployment
    ResetState(DeploymentName),
}

/// This is the name of the deployment that is passed to the deployment commands
#[derive(Parser, Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub struct DeploymentName {
    pub name: String,
}

#[derive(Parser, Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct AnalysisToolsCmd {
    #[command(subcommand)]
    pub tool: AnalysisToolsSubCmd,
}

impl AnalysisToolsCmd {
    pub fn name(&self) -> String {
        match self.tool {
            AnalysisToolsSubCmd::TcpDump { .. } => "TCP Dump".to_string(),
        }
    }
}

#[derive(Parser, Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AnalysisToolsSubCmd {
    // #[clap(trailing_var_arg=true)] // TODO this doesnt seem to remove the need for -- in cli args
    TcpDump {
        /// Specify either an OVS port or a guest interface
        port_or_iface: String,
        /// Specify the name of the output file for the capture
        output_file: String,
        // /// Arguments to pass through to tcpdump such as filters, don't pass -w or -i
        // tcpdump_args: Vec<String>,
    },
}
