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
use kvm_compose_schemas::cli_models::Opts;
use kvm_compose_schemas::deployment_models::{Deployment, DeploymentCommand};
use crate::orchestration::api::{OrchestrationInstruction, OrchestrationLogger, OrchestrationLoggerLevel, OrchestrationProtocol, OrchestrationProtocolResponse};
use crate::orchestration::orchestrator::{run_orchestration, CommandResult};


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
) -> anyhow::Result<bool> {
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
    let safe_sender = Arc::new(Mutex::new(sender));
    let safe_receiver = Arc::new(Mutex::new(receiver));
    // make a copy for the futures
    let websocket_container_sender_clone = safe_sender.clone();
    let websocket_container_receiver_clone = safe_receiver.clone();

    let cancellation_future_ws_sender_clone = safe_sender.clone();

    // when using MPSC channel, the sender will wait until the buffer is read, so it shouldn't
    // matter if the sender works faster than the receiver and therefore the size of the buffer
    // .. experiment shows the buffer does fill faster then the receiver
    let (orchestration_send, mut orchestration_recv) = mpsc::channel(32);

    // start orchestration task
    let run_orchestration_res: anyhow::Result<anyhow::Result<bool>> = tokio::spawn(async move {

        let local_deployment = deployment.clone();
        // let mut orchestration_recv_resub = orchestration_recv.resubscribe();
        let mut orchestration_send_clone = orchestration_send.clone();

        // set off three threads, depending on which ends first there is a different outcome

        // this async task will generate the sequence of commands and push to a channel queue
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

        // this async task will loop over the channel queue taking the task to send over the
        // websocket to the server
        let orchestration_message_cmd_receiver_thread = tokio::spawn(async move {

            // loop polling over the command generator loop, once we get to the end (with an End
            // protocol) we will end this loop. for every protocol generated we call
            // `send_orchestration_instruction` to send the protocol to the testbed server and get
            // the acknowledgement of the command then the result.
            loop {
                // get message from orchestration channel
                if let Some(protocol) = orchestration_recv.recv().await {
                    // end message is always sent if run_orchestration is successful or not
                    match protocol.instruction {
                        OrchestrationInstruction::End => {
                            // if command generation end is matched, then we can stop checking for
                            // more commands and continue to wait for the server to finish if the
                            // last sent command is long-running and/or will send more logging

                            // we need to send the End command still
                            send_orchestration_instruction(
                                websocket_container_sender_clone.clone(), // this is cloned every loop...
                                websocket_container_receiver_clone.clone(),
                                protocol,
                            ).await?;

                            break;
                        }
                        _ => {}
                    }
                    // send protocol to server on websocket, this will then wait for the
                    // acknowledgement of receipt, any logging and then the instruction complete
                    // response
                    send_orchestration_instruction(
                        websocket_container_sender_clone.clone(), // this is cloned every loop...
                        websocket_container_receiver_clone.clone(),
                        protocol,
                    ).await?;
                    
                } else {
                    bail!("exiting socket send loop, message not Ok");
                }
            }

            Ok(())
        });

        // this async task will listen for the user's ctrl+c interrupt to abort the command running
        //
        let orchestration_interrupt_listener = tokio::spawn(async move {
            let _ = tokio::signal::ctrl_c().await;

            tracing::info!("captured ctrl + C, gracefully stopping command");

            // create and send the cancel token to the server, before this exits in the future loop
            // which will abort the other tasks
            let cancel = serde_json::to_vec(&OrchestrationProtocol {
                instruction: OrchestrationInstruction::Cancel,
            }).context("serializing cancel instruction")?;

            cancellation_future_ws_sender_clone
                .lock()
                .await
                .send(Message::Binary(cancel.into()))
                .await
                .context("sending serialised OrchestrationProtocol")?;

            bail!("orchestration was interrupted by user")

        });

        // we should let `orchestration_cmd_generation_thread` finish, unless we are cancelling.
        // but we need to make sure to abort all the futures once we are done.

        // this will check each of the three async tasks to see which has finished first
        let clientside_cmd_result = tokio::spawn(future_loop(
            orchestration_cmd_generation_thread,
            orchestration_message_cmd_receiver_thread,
            orchestration_interrupt_listener,
        )).await.context("running parallel tasks to manage command running state")?;
        // this checks whether the client side was successful or not i.e. the command generation
        // and/or sending to the server could have worked or failed, but this is separate to
        // whether the commands actually worked on the server - we need to get this state back from
        // the server next
        let client_success = match clientside_cmd_result {
            // the command didn't error but the command could be success: true or false
            Ok(cmd_res) => cmd_res.command_success,
            Err(err) => {
                tracing::error!("command error: {}", err);
                false
            },
        };

        // TODO - wait for state of completion of the command from server, if nothing comes back
        //  in X amount of time, then we give this result to the user as well

        tracing::debug!("waiting for server final response");
        let server_success = final_server_response(safe_receiver).await?;

        // both must be successful
        Ok(client_success && server_success)
    })
        .await
        .context("spawning send receive task for client");
    
    // get result of task creation, then get result of orchestration - send errors to GUI and tell
    // the channel receiver to close
    let success = match run_orchestration_res {
        Ok(orchestration_result) => {
            match orchestration_result {
                Ok(success) => {
                    tracing::info!("orchestration command finished");
                    success
                }
                Err(err) => {
                    bail!("Orchestration Failed, error: {err:#}");
                }
            }
        }
        Err(err) => {
            bail!(err);
        }
    };

    tracing::debug!("Orchestration socket closed");

    Ok(success)
}

/// We have three concurrent async tasks running, and we need to be able to handle either if the
/// command running has finished or if the interrupt listener has finished. If command running has
/// finished then we abort the interrupt listener. If the interrupt listener has finished, we abort
/// the command running. The command running consists of the command generator and the command
/// sender.
pub async fn future_loop(
    orchestration_cmd_generation_thread: JoinHandle<Result<CommandResult, Error>>,
    orchestration_message_cmd_receiver_thread: JoinHandle<Result<(), Error>>,
    orchestration_interrupt_listener: JoinHandle<Result<(), Error>>
) -> anyhow::Result<CommandResult> {

    // Here we join together the cmd generation and cmd receiver handles into a single future.
    // This means we can await for both inside this future, and then wait on this single future
    // in the select! macro. So we can test for if both have finished, or the cancel interrupt
    // handler has finished first.

    let both_handlers = tokio::spawn(async {
        let (res, _) = tokio::join!(orchestration_cmd_generation_thread, orchestration_message_cmd_receiver_thread);
        res?
    });
    let both_handlers_abort_handle = both_handlers.abort_handle();
    let orchestration_interrupt_listener_abort_handle = orchestration_interrupt_listener.abort_handle();

    tokio::select! {
        _ = orchestration_interrupt_listener => {
            // caught interrupt, abort the others
            both_handlers_abort_handle.abort();

            bail!("orchestration interrupted");
        }
        result = both_handlers => {
            // orchestration is finished and we have finished sending messages to the server,
            // abort the interrupt listener
            tracing::debug!("cmd gen and msg gen finished, aborting interrupt listener");
            orchestration_interrupt_listener_abort_handle.abort();

            result?
        }
    }

    // TODO - what if either orchestration_cmd_generation_thread or orchestration_message_cmd_receiver_thread never finishes?

    // TODO - what if the CMD generation thread has not finished generating?

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
    websocket_sender: Arc<Mutex<SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>>>,
    websocket_receiver: Arc<Mutex<SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>>>,
    orchestration_protocol: OrchestrationProtocol,
) -> anyhow::Result<()> {

    tracing::info!("making request: {}", orchestration_protocol.instruction.name());

    // need to serialise the instruction to binary format
    let serialised_instruction = serde_json::to_vec(&orchestration_protocol)
        .context("serialising OrchestrationProtocol")?;
    let _ = websocket_sender
        .lock()
        .await
        .send(Message::Binary(serialised_instruction.into()))
        .await
        .context("sending serialised OrchestrationProtocol")?;

    // get acknowledgement
    if let Some(response) = websocket_receiver
        .lock()
        .await
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

    // now that we have the acknowledgement, we will need to wait for the response from the server
    // that the command has completed either successfully or unsuccessfully. however, meanwhile the
    // server may be sending an unknown number of logging messages before the completion message, so
    // we will loop de-serialising the messages until we get completion confirmation

    loop {

        // wait for response
        if let Some(response) = websocket_receiver
            .lock()
            .await
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
                        // regardless of the result, we break out of the loop
                        break;
                    }
                    
                    // if there was no confirmation of completion, it will be a logging message so
                    // we will de-serialise it, present to the user, then continue looping until we
                    // get the completion message

                    // handle a logging message from server
                    let logging_result: Result<OrchestrationLogger, serde_json::Error> = serde_json::from_str(&b);
                    if let Ok(ref log) = logging_result {
                        match log {
                            OrchestrationLogger::Log { message, level } => {
                                match level {
                                    OrchestrationLoggerLevel::Info => tracing::info!("{message}"),
                                    OrchestrationLoggerLevel::Error => tracing::error!("{message}"),
                                }
                            }
                            _ => {} // dont handle `End` here
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
    }


    Ok(())
}

async fn final_server_response(
    safe_receiver: Arc<Mutex<SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>>>
) -> anyhow::Result<bool> {

    let message = safe_receiver
        .lock()
        .await
        .next()
        .await;

    if let Some(Ok(final_response)) = message {
        match final_response {
            Message::Text(b) => {
                let response_result: Result<OrchestrationProtocolResponse, serde_json::Error> = serde_json::from_str(&b);
                if let Ok(ref response) = response_result {
                    let result_messages = response.get_result_messages()?;
                    if let Some(success) = result_messages.success_message {
                        for msg in success {
                            tracing::info!("Server response: {}", msg);
                        }
                        return Ok(true);
                    }
                    if let Some(fail) = result_messages.fail_message {
                        // tracing::error!("Instruction completed with the following failures: {}", fail);
                        for msg in fail {
                            tracing::error!("Server response: {}", msg);
                        }
                        return Ok(false);
                    }
                    // TODO - logic here can be improved, this doesn't correspond to anything
                    Ok(false)
                } else {
                    // we couldn't get the final response
                    tracing::error!("could not deserialise the last response from the server");
                    Ok(false)
                }
            }
            _ => {
                tracing::error!("received unexpected response format from server when waiting for final message");
                Ok(false)
            }
        }
    } else {
        tracing::error!("received unexpected response from server when waiting for final message");
        Ok(false)
    }
}
