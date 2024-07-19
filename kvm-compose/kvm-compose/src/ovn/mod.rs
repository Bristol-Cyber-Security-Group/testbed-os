use std::future::Future;
use tokio::process::Command;
use anyhow::bail;
use async_trait::async_trait;
use thiserror::Error;
use crate::orchestration::OrchestrationCommon;

pub mod components;
pub mod ovn_state;
pub mod configuration;
pub mod ovn_serde;

/// This enum represents the different kinds of results in adding or removing different OVN logical
/// components to OvnNetwork.
#[derive(Error, PartialOrd, PartialEq, Debug)]
pub enum LogicalOperationResult {
    // Successful,
    #[error("Component {name} already exists")]
    AlreadyExists {
        name: String,
    },
    #[error("Component {name} does not exist")]
    DoesNotExist {
        name: String,
    },
    #[error("Component parent {parent} for {name} does not exist")]
    ParentDoesNotExist {
        name: String,
        parent: String,
    },
    #[error("Component {name} has children still defined")]
    HasChildren {
        name: String,
    },
    #[error("{msg}")]
    Error {
        msg: String,
    }
}

/// This trait allows each OVN component to have it's own command prepared and sent into the
/// closure as the argument, which will be the command running function. However, this also means
/// that in unit testing, we can instead send in a function that just passes through the command
/// string so that we can test equality rather than execute the command.
/// Note the generic `F` which shows the closure returned shares the same return type as the
/// function, while it won't be the closure being returned, the internal function used will return
/// a return with the output string - this might be useful as it contains the stdout/stderr
#[async_trait]
pub trait OvnCommand {
    async fn create_command<F>(
        &self, f: impl Fn(Vec<String>, (Option<String>, OrchestrationCommon)) -> F + Send + Sync,
        config: (Option<String>, OrchestrationCommon)
    ) -> anyhow::Result<String>
        where
            F: Future<Output = anyhow::Result<String>> + Send;
    async fn destroy_command<F>(
        &self, f: impl Fn(Vec<String>, (Option<String>, OrchestrationCommon)) -> F + Send + Sync,
        config: (Option<String>, OrchestrationCommon)
    ) -> anyhow::Result<String>
        where
            F: Future<Output = anyhow::Result<String>> + Send;
}

/// Used in the examples code
pub async fn run_cmd(
    cmd: Vec<String>,
    allow_fail: bool,
) -> anyhow::Result<String> {
    let args: Vec<_> = cmd.iter()
        .map(|s| s.as_str())
        .collect();
    println!("{:?}", cmd);
    let command = Command::new("sudo")
        .args(args)
        .output()
        .await?;
    if !command.status.success() && !allow_fail {
        bail!("{:?}", String::from_utf8(command.stderr)?);
    }
    Ok(String::from_utf8(command.stdout)?)
}

/// used in tests instead of `ovn_run_cmd`, just pass through the command
pub async fn test_ovn_run_cmd(
    cmd: Vec<String>,
    _: (Option<String>, OrchestrationCommon),
) -> anyhow::Result<String> {
    Ok(cmd.join(" "))
}

#[cfg(test)]
mod tests {

}