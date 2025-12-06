use std::path::PathBuf;
use clap::Parser;
use tokio::task::JoinHandle;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::Layer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use packet_capture::{TCPDumpConsumer, TestbedPacketCapture};

/// This CLI tool allows you to capture packets on the testbed OVS bridge.
/// The bridge is fixed to `br-int` which is the OVN integration bridge that is used to provide the
/// networking in the testbed.
///
/// You must supply the interface, which will be the same as the port on the integration bridge for
/// the given guest you would like to capture on.
/// If you check with `ovs-vsctl show` you can check which `Port` you want to capture on.
/// This will then create a mirror port on a dummy interface to prevent interrupting any traffic.
///
/// You can also supply filters using the `tcpdump` Berkeley Packet Filter (BPF) syntax.
/// For example, after all the arguments in the command, you can specify any filter arguments.
/// `sudo testbedos-tcpdump -i vm-ovn00 dst host 10.0.0.21`
/// This will capture on the interface `vm-ovn00` and use the BPF filter of `dst host 10.0.0.21`,
/// which will filter packets with destination to ip 10.0.0.21 only.
#[derive(Parser, Debug)]
struct CliArgs {

    /// The interface or port to capture from
    #[clap(short, long)]
    interface: String,

    /// Optional arguments to be used with `tcpdump`
    #[arg(trailing_var_arg = true, allow_hyphen_values = true, index = 1)]
    dump_args: Vec<String>,

    // /// Optional name for mirror port, otherwise one will automatically be made
    // mirror_to: Option<String>,

    /// Enable SPAN to mirror all traffic on the bridge
    #[clap(short, long)]
    span: bool,

    #[clap(short, long)]
    output_file: PathBuf,
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
    run_loop(cli_args).await?;

    Ok(())
}

/// In the run loop, we will run the pcap listener in a loop while at the same time waiting for a
/// stop instruction to gracefully end the packet capture.
async fn run_loop(
    args: CliArgs,
) -> anyhow::Result<()> {
    // channel that will be used to send and listen for the stop instruction
    let (stop_tx, stop_rx) = tokio::sync::oneshot::channel::<()>();

    // create the packet capture
    let tb_packet_capture = TestbedPacketCapture {};

    // convert cli args to tcpdump args
    let config = packet_capture::TCPDumpConfig::new(
        args.interface,
        // args.mirror_to,
        args.span,
        args.dump_args,
        TCPDumpConsumer::File(args.output_file),
    )?;

    // packet capture future start, this wraps the blocking thread call in `packet_capture`
    let packet_capture_handle: JoinHandle<anyhow::Result<()>> = tokio::spawn(async move {
        tb_packet_capture.capture(config, stop_rx).await?;
        Ok(())
    });

    // start future to listen to ctrl+c
    let stop_handle: JoinHandle<anyhow::Result<()>> = tokio::spawn(async move {
        let _ = tokio::signal::ctrl_c().await;
        tracing::info!("captured ctrl + C, gracefully stopping command");

        tracing::info!("sending cancel token");
        stop_tx.send(())
            .map_err(|_| anyhow::anyhow!("failed to send stop instruction to channel"))?;
        Ok(())
    });

    // wait for packet capture to finish, which will either be from the cancel token triggering a
    // tear down, or the packet capture has errored
    packet_capture_handle.await??;
    // properly drop the cancel listener to clear resources
    std::mem::drop(stop_handle);

    Ok(())
}
