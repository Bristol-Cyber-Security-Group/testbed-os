use packet_capture::{TCPDumpConsumer, TestbedPacketCapture};
use std::sync::Arc;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use kvm_compose_schemas::cli_models::{ToolCmd, ToolSubCmd};
use crate::orchestration::api::{OrchestrationLogger};

/// Run the packet capture. This function needs to work out which testbed host this capture needs to
/// run on. This function also needs to work out if the
pub async fn packet_capture(
    tool_cmd: &ToolCmd,
    cancel_token_recv: Arc<Mutex<Receiver<()>>>,
    logging_send: &Sender<OrchestrationLogger>,
) -> anyhow::Result<()> {

    // channel that will be used to send and listen for the stop instruction
    let (stop_tx, stop_rx) = tokio::sync::oneshot::channel::<()>();
    // create the packet capture
    let tb_packet_capture = TestbedPacketCapture {};

    // get args from tools_cmd
    let config = match &tool_cmd.tool {
        ToolSubCmd::TcpDump { interface, span, dump_args, file_output } => {
            packet_capture::TCPDumpConfig::new(
                interface.clone(),
                span.clone(),
                dump_args.clone(),
                TCPDumpConsumer::File(file_output.clone()),
            )?
        }
        _ => unreachable!(),
    };

    // packet capture future start, this wraps the blocking thread call in `packet_capture`
    let packet_capture_handle: JoinHandle<anyhow::Result<()>> = tokio::spawn(async move {
        tb_packet_capture.capture(config, stop_rx).await?;
        Ok(())
    });

    let mut cancel_lock = cancel_token_recv.lock().await;

    // wait on either the cancel token coming in, or the packet capture exits itself
    tokio::select! {
        _ = cancel_lock.recv() => {
            stop_tx.send(())
            .map_err(|_| anyhow::anyhow!("failed to send stop instruction to channel"))?;
        }
        result = packet_capture_handle => {
            result??;
        }
    }

    tracing::info!("finished packet capture");

    Ok(())
}
