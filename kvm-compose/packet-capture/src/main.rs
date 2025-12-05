use clap::Parser;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::Layer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use packet_capture::{TestbedPacketCapture};

#[derive(Parser, Debug)]
struct CliArgs {

    /// The interface or port to capture from
    #[clap(short, long)]
    interface: String,

    // TODO - recreate the ovs-tcpdump api

    // db-sock

    // dump-cmd

    // mirror-to

    // span

}

#[tokio::main]
async fn main() -> anyhow::Result<()> {

    // set up logging for the CLI
    let stdout_log = tracing_subscriber::fmt::layer();
    tracing_subscriber::registry()
        .with(stdout_log.with_filter(LevelFilter::INFO))
        .init();

    // parse the CLI options and then run the packet capture with the given arguments
    let cli_args = CliArgs::parse();
    run_loop(cli_args.interface).await?;

    Ok(())
}

/// In the run loop, we will run the pcap listener in a loop while at the same time waiting for a
/// stop instruction to gracefully end the packet capture.
pub async fn run_loop(
    interface: String,
) -> anyhow::Result<()> {
    // channel that will be used to send and listen for the stop instruction
    let (stop_tx, stop_rx) = tokio::sync::oneshot::channel::<()>();

    // create the packet capture
    let tb_packet_capture = TestbedPacketCapture {};

    // packet capture future start, this wraps the blocking thread call in `packet_capture`
    let packet_capture_handle = tokio::spawn(async move {
        tb_packet_capture.capture(interface, stop_rx).await
    });

    // start future to listen to ctrl+c
    let stop_handle = tokio::spawn(async move {
        let _ = tokio::signal::ctrl_c().await;
        tracing::info!("captured ctrl + C, gracefully stopping command");
    });

    // wait for either packet capture to error or for ctrl+c to interrupt to exit gracefully
    tokio::select! {
        _ = stop_handle => {
            // ctrl+c triggered, send stop token to packet capture
            tracing::info!("stopping command");
            stop_tx.send(())
                .map_err(|_| anyhow::anyhow!("failed to send stop instruction to channel"))?;
        }
        result = packet_capture_handle => {
            anyhow::bail!("packet_capture_handle exited with {:?}", result);
        }
    }

    Ok(())
}
