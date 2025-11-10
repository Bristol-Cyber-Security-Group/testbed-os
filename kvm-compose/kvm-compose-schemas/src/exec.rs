use std::path::PathBuf;
use clap::Parser;
use serde::{Deserialize, Serialize};

/// Entrypoint to run commands against a guest.
#[derive(Parser, Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct ExecCmd {
    #[clap(index = 1)]
    pub guest_name: String,
    #[clap(subcommand)]
    pub command_type: ExecCmdType,
}

impl ExecCmd {
    pub fn name(&self) -> String {
        match self.command_type {
            ExecCmdType::ShellCommand(_) => "Shell Command".to_string(),
            ExecCmdType::Push(_) => "Push file or folder".to_string(),
            ExecCmdType::Pull(_) => "Pull file or folder".to_string(),
            ExecCmdType::Tool(_) => "Tool".to_string(),
        }
    }
}

/// Represents that various types of commands we can run against a guest
#[derive(Parser, Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExecCmdType {
    ShellCommand(ExecCmdShellCommand),
    /// Push a file or folder to a guest
    Push(ExecCmdFileTransfer),
    /// Pull a file or folder from a guest
    Pull(ExecCmdFileTransfer),
    Tool(ExecCmdTool),
}

/// A command that will be run inside the guest's shell, if the guest type permits.
#[derive(Parser, Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct ExecCmdShellCommand {
    #[clap(short, long, default_value="5000", help = "Timeout in milliseconds, defaults to 5s")]
    pub timeout_ms: u64,
    #[clap(trailing_var_arg=true, index = 1)]
    pub command: Vec<String>,
    #[clap(short, long, help = "Suppress all logging during command execution")]
    pub suppress_logging: bool,
}

/// Represents the options for the push and pull file transfer sub-commands
#[derive(Parser, Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct ExecCmdFileTransfer {
    #[clap(short, long, value_parser, help = "Location of file or folder to push on host")]
    pub source_path: PathBuf,
    #[clap(short, long, value_parser, help = "Location to push file or folder to on guest")]
    pub target_path: PathBuf,
}

/// A tool that will be run against the guest. This will be a tool that is included with the testbed
/// such that there is a pre-defined way to execute the tool against a guest.
#[derive(Parser, Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct ExecCmdTool {
    #[clap(subcommand)]
    pub tool: TestbedTools,
}

/// A user defined script that will be run against the guest. The user must provision their own way
/// to interact with the guest.
#[derive(Parser, Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct ExecCmdUserScript {
    #[clap(short, long, help = "Run script from main testbed even if guest is on a remote testbed")]
    pub run_on_main_testbed: bool,
    #[clap(trailing_var_arg=true, index = 1)]
    pub script: Vec<String>,
}

/// A collection of tools that is included with the testbed and knows how to use.
#[derive(Parser, Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TestbedTools {
    /// Run an adb command against an android guest
    ADB(Command),
    FridaSetup,
    /// Install an Android app from an APK file
    InstallApk(Command),
    TestPermissions(Command),
    TestPrivacy(Command),
    TLSIntercept(Command),
}

#[derive(Parser, Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub struct Command {
    #[clap(trailing_var_arg=true, index = 1)]
    pub command: Vec<String>,
}
