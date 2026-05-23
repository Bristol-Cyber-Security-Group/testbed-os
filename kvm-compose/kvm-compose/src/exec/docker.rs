use anyhow::bail;
use tokio::sync::mpsc::Sender;
use crate::orchestration::api::OrchestrationLogger;
use crate::orchestration::{run_subprocess_command, OrchestrationCommon};
use crate::state::schema::guest::StateTestbedGuest;

pub async fn shell_command(
    command: Vec<&str>,
    _guest_data: &StateTestbedGuest,
    guest_name_with_project: &String,
    _common: &OrchestrationCommon,
    logging_send: &Sender<OrchestrationLogger>,
) -> anyhow::Result<(String, i32)> {

    // join the user command to the docker exec command
    let mut docker_cmd = vec!["docker", "exec", guest_name_with_project, "/bin/sh", "-c"];
    let cmd = command.join(" ");
    docker_cmd.push(&cmd);

    // run the command
    let cmd_res = run_subprocess_command(
        "sudo",
        docker_cmd,
        false,
        None,
    ).await;
    
    match cmd_res {
        Ok(ok) => {
            logging_send.send(OrchestrationLogger::info(format!("Command output:\n{ok}"))).await?;
            return Ok((ok, 0)); // TODO - get actual return code
        }
        Err(err) => {
            bail!(err);
        }
    }
}
