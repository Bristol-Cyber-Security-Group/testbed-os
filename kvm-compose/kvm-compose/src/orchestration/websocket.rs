use std::borrow::Cow;
use std::sync::Arc;
use anyhow::{bail, Context, Error};
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio::sync::{mpsc, Mutex};
use tokio::sync::mpsc::{Sender};
use tokio::task::JoinHandle;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};
use tokio_tungstenite::tungstenite::{Message};
use tokio_tungstenite::tungstenite::protocol::CloseFrame;
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
use kvm_compose_schemas::cli_models::Opts;
use kvm_compose_schemas::deployment_models::{Deployment, DeploymentCommand};
use crate::orchestration::api::{OrchestrationInstruction, OrchestrationLogger, OrchestrationLoggerLevel, OrchestrationProtocol, OrchestrationProtocolResponse};
use crate::orchestration::orchestrator::{run_orchestration};

struct WebsocketContainer {
    pub sender: SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>,
    pub receiver: SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>,
}

/// This function completely handles the orchestration command from the client side by sending instructions to the
/// server. We pass the websocket sink and stream to the orchestration function, and depending on the orchestration
/// command, the respective `OrchestrationProtocol`. The orchestration process will use the function
/// `send_orchestration_instruction` that will handle sending the `OrchestrationProtocol`, receiving the acknowledgement
/// and then receiving the `OrchestrationProtocolResponse` for that instruction.
/// If there are any errors in the `OrchestrationProtocolResponse`, then the client will close the websocket connection
/// and the server will handle the database update. The client can then continue and verify the state of the
/// deployment separately.
pub async fn ws_orchestration_client(
    runner_url: String,
    deployment: Deployment,
    command: DeploymentCommand,
    opts: Opts,
) -> anyhow::Result<()> {
    tracing::debug!("starting orchestration websocket");

    let ws_stream = match connect_async(runner_url).await {
        Ok((stream, response)) => {
            tracing::debug!("Server response was {:?}", response);
            stream
        }
        Err(e) => {
            bail!("WebSocket handshake for client failed with {e}!");
        }
    };

    let (sender, receiver) = ws_stream.split();
    // since we move this sender and received into the futures below, we need to wrap these in a thread safe
    // container so that we can clone the container and re-use it later
    let websocket_container = Arc::new(Mutex::new(WebsocketContainer { sender, receiver }));
    // make a copy for the futures
    let websocket_container_clone = websocket_container.clone();

    // when using MPSC channel, the sender will wait until the buffer is read, so it shouldn't
    // matter if the sender works faster than the receiver and therefore the size of the buffer
    // .. experiment shows the buffer does fill faster then the receiver
    let (orchestration_send, mut orchestration_recv) = mpsc::channel(32);

    // start orchestration task
    let run_orchestration_res = tokio::spawn(async move {

        let local_deployment = deployment.clone();
        // let mut orchestration_recv_resub = orchestration_recv.resubscribe();
        let mut orchestration_send_clone = orchestration_send.clone();

        // set off three threads, depending on which ends first there is a different outcome

        let orchestration_cmd_generation_thread = tokio::spawn(async move {
            // TODO - this assumes the messages were OK, so the deployment state should
            //  really come from the message sender
            let res = run_orchestration(
                local_deployment.clone(),
                command.clone(),
                opts,
                &mut orchestration_send_clone,
                // &mut orchestration_recv_resub,
            ).await.context("getting run orchestration result");
            // tell channel that orchestration message sending is done
            // regardless if `run_orchestration` was successful or not to prevent waiting indefinitely
            orchestration_send_clone.send(OrchestrationProtocol { instruction: OrchestrationInstruction::End }).await?;

            res
        });

        let orchestration_message_cmd_receiver_thread = tokio::spawn(async move {
            loop {
                // get message from orchestration channel
                if let Some(protocol) = orchestration_recv.recv().await {
                    // end message is always sent if run_orchestration is successful or not
                    if let OrchestrationInstruction::End = protocol.instruction {
                        // if command generation is matched, then we can end this future
                        break;
                    }
                    // send protocol to server on websocket
                    send_orchestration_instruction(
                        websocket_container_clone.clone(), // this is cloned every loop...
                        protocol,
                    ).await?;
                } else {
                    bail!("exiting socket send loop, message not Ok");
                }
            }
            Ok(())
        });

        let orchestration_interrupt_listener = tokio::spawn(async move {
            let _ = tokio::signal::ctrl_c().await;

            tracing::info!("captured ctrl + C, gracefully stopping command");

            bail!("orchestration was interrupted by user")

        });

        // we should let `orchestration_cmd_generation_thread` finish, unless we are cancelling.
        // but we need to make sure to abort all the futures once we are done.

        let deployment_result = tokio::spawn(future_loop(
            orchestration_cmd_generation_thread,
            orchestration_message_cmd_receiver_thread,
            orchestration_interrupt_listener,
        )).await.context("running parallel tasks to manage command running state")?;
        match deployment_result {
            Ok(_) => {}
            Err(err) => {
                bail!(err);
            },
        }

        // close websocket
        tracing::debug!("closing websocket");
        websocket_container.lock()
                .await
                .sender
                .send(Message::Close(Some(CloseFrame {
            code: CloseCode::Normal,
            reason: Cow::from("End of orchestration"),
        }))).await?;

        Ok(())
    })
        .await
        .context("spawning send receive task for client");
    
    // get result of task creation, then get result of orchestration - send errors to GUI and tell
    // the channel receiver to close
    match run_orchestration_res {
        Ok(orchestration_result) => {
            match orchestration_result {
                Ok(_) => {
                    tracing::info!("orchestration Ok");
                }
                Err(err) => {
                    bail!("Orchestration Failed, error: {err:#}");
                }
            }
        }
        Err(err) => {
            bail!(err);
        }
    }

    tracing::debug!("Orchestration socket closed");

    Ok(())
}

pub async fn future_loop<T>(
    orchestration_cmd_generation_thread: JoinHandle<anyhow::Result<Deployment>>,
    orchestration_message_cmd_receiver_thread: JoinHandle<T>,
    orchestration_interrupt_listener: JoinHandle<Result<T, Error>>
) -> anyhow::Result<Deployment> {
    loop {
        if orchestration_interrupt_listener.is_finished() {
            // caught interrupt, abort the others
            orchestration_cmd_generation_thread.abort();
            orchestration_message_cmd_receiver_thread.abort();
            orchestration_interrupt_listener.abort();
            bail!("orchestration interrupted");
        }
        if orchestration_cmd_generation_thread.is_finished() && orchestration_message_cmd_receiver_thread.is_finished() {
            // orchestration is finished and we have finished sending messages to the server,
            // abort the interrupt listener
            tracing::debug!("cmd gen and msg gen finished, aborting interrupt listener");
            orchestration_interrupt_listener.abort();
            tracing::debug!("awaiting on cmd gen");
            let result = orchestration_cmd_generation_thread.await?;
            tracing::debug!("awaiting on msg gen");
            orchestration_message_cmd_receiver_thread.await?;
            tracing::debug!("returning deployment result");
            return result;
        }
        // TODO - what if either orchestration_cmd_generation_thread or orchestration_message_cmd_receiver_thread never finishes?
    }
}

pub async fn send_orchestration_instruction_over_channel(
    sender: &mut Sender<OrchestrationProtocol>,
    orchestration_instruction: OrchestrationInstruction,
) -> anyhow::Result<()> {

    tracing::info!("generated orchestration protocol: {}", orchestration_instruction.name());
    let protocol = OrchestrationProtocol {
        // common: orchestration_common,
        instruction: orchestration_instruction,
    };

    sender.send(protocol).await?;

    Ok(())
}

/// This function is used in the client orchestration to send the orchestration instruction to the server for the
/// server to process. This protocol is split into three parts on the client side: 1) send 2) receive acknowledgement
/// 3) wait for response of outcome of instruction. The outcome may or may not have been successful.
async fn send_orchestration_instruction(
    websocket_container: Arc<Mutex<WebsocketContainer>>,
    orchestration_protocol: OrchestrationProtocol,
) -> anyhow::Result<()> {

    tracing::info!("making request: {}", orchestration_protocol.instruction.name());

    // need to serialise the instruction to binary format
    let serialised_instruction = serde_json::to_vec(&orchestration_protocol)
        .context("serialising OrchestrationProtocol")?;
    websocket_container
        .lock()
        .await
        .sender
        .send(Message::Binary(serialised_instruction))
        .await
        .context("sending serialised OrchestrationProtocol")?;

    // get acknowledgement
    if let Some(response) = websocket_container
        .lock()
        .await
        .receiver
        .next()
        .await {
        let response = response
            .context("getting acknowledgement response")?;
        match response {
            Message::Text(t) => {
                tracing::debug!("Server response: {t}");
            }
            Message::Close(msg) => {
                match msg {
                    None => tracing::error!("received close from server as acknowledgement"),
                    Some(close) => {
                        tracing::error!("received close from server as acknowledgement, reason: {}", close.reason);
                    }
                }
                return Ok(());
            }
            _ => bail!("got unexpected message type for acknowledgement"),
        }
    } else {
        bail!("problem in getting websocket acknowledgement response from server");
    }

    // wait for response
    if let Some(response) = websocket_container
        .lock()
        .await
        .receiver
        .next().await {
        let response = response
            .context("getting instruction outcome response")?;
        match response {
            Message::Text(b) => {

                // the message could be either
                // OrchestrationProtocolResponse or OrchestrationLogger, handle appropriately

                // handle orchestration message from server
                let response_result: Result<OrchestrationProtocolResponse, serde_json::Error> = serde_json::from_str(&b);
                if let Ok(ref response) = response_result {
                    let result_messages = response.get_result_messages()?;
                    if let Some(success) = result_messages.success_message {
                        for msg in success {
                            tracing::info!("{}", msg);
                        }
                    }
                    if let Some(fail) = result_messages.fail_message {
                        // tracing::error!("Instruction completed with the following failures: {}", fail);
                        for msg in fail {
                            tracing::error!("Instruction failed for: {}", msg);
                        }
                    }

                    if !response.is_success()? {
                        bail!("instruction failed");
                    }
                }

                // handle a logging message from server
                let logging_result: Result<OrchestrationLogger, serde_json::Error> = serde_json::from_str(&b);
                if let Ok(OrchestrationLogger::Log { ref message, ref level }) = logging_result {
                    match level {
                        OrchestrationLoggerLevel::Info => tracing::info!("{message}"),
                        OrchestrationLoggerLevel::Error => tracing::error!("{message}"),
                    }
                }

                // in case both messages were corrupt, bail
                if response_result.is_err() && logging_result.is_err() {
                    bail!("message received from client was neither a result or logging message");
                }

            }
            Message::Close(msg) => {
                match msg {
                    None => tracing::error!("received close from server as instruction result"),
                    Some(close) => {
                        tracing::error!("received close from server as instruction result, reason: {}", close.reason)
                    }
                }
                return Ok(());
            }
            _ => bail!("got unexpected message type for acknowledgement"),
        }
    } else {
        bail!("problem in getting websocket instruction outcome response from server");
    }


    Ok(())
}
