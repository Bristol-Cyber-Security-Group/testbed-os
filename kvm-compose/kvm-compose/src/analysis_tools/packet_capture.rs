use std::time::Duration;
use kvm_compose_schemas::cli_models::AnalysisToolsCmd;

/// Run the packet capture. This function needs to work out which testbed host this capture needs to
/// run on. This function also needs to work out if the
pub async fn packet_capture(
    at: &AnalysisToolsCmd
) -> anyhow::Result<()> {
    //
    tracing::info!("tcpdump args: {at:?}");

    tokio::time::sleep(Duration::from_secs_f32(2.0)).await;

    // TODO intercept the -i argument

    // TODO - if br-int, disallow due to ambiguity and un-usefulness
    // TODO - if br-ex, assume is on master host
    Ok(())
}
