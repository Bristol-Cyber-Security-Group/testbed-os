use tokio::sync::mpsc::Sender;
use crate::orchestration::api::OrchestrationLogger;
use crate::orchestration::OrchestrationCommon;
use crate::state::StateTestbedGuest;

pub async fn shell_command(
    command: Vec<&str>,
    guest_data: &StateTestbedGuest,
    guest_name_with_project: &String,
    common: &OrchestrationCommon,
    logging_send: &Sender<OrchestrationLogger>,
) -> anyhow::Result<()> {
    unimplemented!()
}
