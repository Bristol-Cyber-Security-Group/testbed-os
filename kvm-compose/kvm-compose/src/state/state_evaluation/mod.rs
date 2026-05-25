mod guests;
mod network;

use async_trait::async_trait;
use tokio::process::Command;

/// This enum is to be used to record the state of any components
#[derive(PartialEq)]
pub enum StateComponentStatus {
    Up,
    Down(String),
    DoesNotExist,
}

/// Trait to be implemented for all components that require state checking
#[async_trait]
pub trait EvaluateState {
    async fn check_exists(&self, project_name: String) -> anyhow::Result<StateComponentStatus> {
        let check_command = self.get_check_command(project_name).await;

        let output = Command::new("sudo")
            .arg("sh")
            .arg("-c")
            .arg(&check_command)
            .output()
            .await?;

        let status_string = String::from_utf8_lossy(&output.stdout).trim().to_string();

        let result = match status_string.as_str() {
            "does_not_exist" => StateComponentStatus::DoesNotExist,
            "running" => StateComponentStatus::Up, // "running" is used by the services we call
            // assume anything that isn't "running" or not exist to mean it is down for some reason
            down => StateComponentStatus::Down(down.to_string()),

        };

        Ok(result)
    }
    async fn get_check_command(&self, project_name: String) -> String;
}
