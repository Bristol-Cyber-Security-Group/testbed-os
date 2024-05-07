use crate::cli_models::{AnalysisToolsCmd, SnapshotSubCommand, UpCmd};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::fmt::Formatter;
use chrono::{DateTime, Utc};
use crate::exec::ExecCmd;

#[derive(Deserialize, Serialize, Debug, Default, Clone)]
#[serde(rename_all = "snake_case")]
pub struct NewDeployment {
    pub name: String,
    pub project_location: String,
}

impl fmt::Display for NewDeployment {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(&serde_json::to_string_pretty(&self).unwrap())
            .expect("new deployment to json via serde failed");
        Ok(())
    }
}

#[derive(Deserialize, Serialize, Debug, Default, Clone)]
#[serde(rename_all = "snake_case")]
pub struct Deployment {
    pub name: String,
    pub project_location: String,
    pub state: DeploymentState,
    // this stores the last/current uuid for polling for logs
    pub last_action_uuid: Option<String>,
}

impl fmt::Display for Deployment {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(&serde_json::to_string_pretty(&self).unwrap())
            .expect("deployment to json via serde failed");
        Ok(())
    }
}

#[derive(Deserialize, Serialize, Debug, Default, Clone)]
#[serde(rename_all = "snake_case")]
pub struct DeploymentLogs {
    pub logs: HashMap<String, DeploymentLogsData>,
}

#[derive(Deserialize, Serialize, Debug, Default, Clone)]
#[serde(rename_all = "snake_case")]
pub struct DeploymentLogsData {
    pub log_path: String,
    pub error_code: Option<i32>,
    pub end_state: Option<DeploymentState>,
    pub execution_time: DateTime<Utc>,
}

impl fmt::Display for DeploymentLogs {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(&serde_json::to_string_pretty(&self).unwrap())
            .expect("deployment logs to json via serde failed");
        Ok(())
    }
}

/// This enum represents the possible states a deployment can be in due to orchestration or as a
/// results of operations through the server
#[derive(Deserialize, Serialize, Debug, Default, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DeploymentState {
    Up,
    #[default]
    Down,
    // set this state if the orchestration is executing
    Running,
    Failed(DeploymentCommand),
}

/// This enum represents the possible commands that are allowed by orchestration
#[derive(Deserialize, Serialize, Debug, Default, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DeploymentCommand {
    Up {
        up_cmd: UpCmd,
    },
    #[default]
    Down,
    GenerateArtefacts,
    ClearArtefacts,
    // Start,
    // Stop,
    // Pause,
    // Resume,
    Snapshot {
        snapshot_cmd: SnapshotSubCommand,
    },
    TestbedSnapshot {
        snapshot_guests: bool,
    },
    AnalysisTool(AnalysisToolsCmd),
    Exec(ExecCmd),
    ListCloudImages,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "snake_case")]
pub struct DeploymentList {
    pub deployments: HashMap<String, Deployment>,
}

#[derive(Deserialize, Serialize, Debug, Default, Clone)]
#[serde(rename_all = "snake_case")]
pub struct DeploymentAction {
    pub command: DeploymentCommand,
    #[serde(default)]
    pub generate_artefacts: bool,
}
