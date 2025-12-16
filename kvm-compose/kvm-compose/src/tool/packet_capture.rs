use packet_capture::{TCPDumpConsumer, TestbedPacketCapture};
use std::sync::Arc;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use kvm_compose_schemas::cli_models::{ToolCmd, ToolSubCmd};
use crate::orchestration::api::{OrchestrationLogger, OrchestrationLoggerLevel};

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
            ).await?
        }
    };

    logging_send.send(OrchestrationLogger::Log {
        message: "Starting packet capture, there is no visual logging of packets here, will wait for user to stop command through a cancel".to_string(),
        level: OrchestrationLoggerLevel::Info,
    }).await?;

    // packet capture future start, this wraps the blocking thread call in `packet_capture`
    let packet_capture_handle: JoinHandle<anyhow::Result<()>> = tokio::spawn(async move {
        tb_packet_capture.capture(config, stop_rx).await?;
        Ok(())
    });

    // immediately start a future that is listening for a testbed cancel token
    let stop_handle: JoinHandle<anyhow::Result<()>> = tokio::spawn(async move {
        // set up waiting for the cancel token from the testbed client ...
        // if we receive a token here
        let _ = cancel_token_recv.lock().await.recv().await;
        // then we send a token to the packet capture internals
        stop_tx.send(())
            .map_err(|_| anyhow::anyhow!("failed to send stop instruction to channel"))?;
        Ok(())
    });

    // wait for the packet capture to finish, which will either be from the cancel token triggering
    // a tear down, or the packet capture has errored
    packet_capture_handle.await??;
    // properly drop the cancel listener to clear resources
    std::mem::drop(stop_handle);

    tracing::info!("finished packet capture");

    Ok(())
}
