use std::sync::Arc;
use anyhow::{bail, Context};
use axum::extract::ws::{CloseFrame, Message, Utf8Bytes, WebSocket};
use futures_util::{SinkExt, StreamExt};
use futures_util::stream::SplitSink;
use tokio::sync::{mpsc, Mutex};
use tokio::sync::mpsc::{Receiver};
use tokio::task::JoinHandle;
use kvm_compose_lib::orchestration::api::{OrchestrationInstruction, OrchestrationLogger, OrchestrationProtocol, OrchestrationProtocolResponse};
use kvm_compose_lib::orchestration::{OrchestrationCommon};
use kvm_compose_lib::state::orchestration_tasks::get_orchestration_common;
use kvm_compose_lib::state::schema::State;
use kvm_compose_schemas::deployment_models::{Deployment, DeploymentCommand};
use crate::AppState;
use crate::state_evaluation::models::{total_deployment_state, DeploymentStatus};

/// This function completely handles the orchestration command requested by the client. This is the websocket
/// implementation. Depending on the result of the orchestration or any runtime errors, the result is updated here
/// before the websocket is closed.
/// The main process for this server side of the websocket it to listen for `OrchestrationProtocol` that comes from the
/// client. The `OrchestrationProtocol` contains implementation for the different "instructions" possible that the
/// client can request, For every `OrchestrationProtocol`, the server will respond that the protocol was received and
/// deserialised OK. Then will respond again after with the outcome of that instruction using the
/// `OrchestrationProtocolResponse` type that is returned by `OrchestrationProtocol`.
pub async fn handle_orchestration_socket(
    socket: WebSocket,
    db_config: Arc<AppState>,
) {

    let result = tokio::spawn(run(
        socket,
        db_config,
    )).await;

    // make sure thread is Ok
    let result = match result {
        Ok(ok) => ok,
        Err(err) => {
            tracing::error!("orchestration socket crashed, error: {err:#}");
            return;
        }
    };

    match result {
        Ok(_) => {
            tracing::info!("end of orchestration websocket, closing socket");
        }
        Err(err) => {
            tracing::error!("orchestration failed");
            err.chain().for_each(|cause| tracing::error!("because: {}", cause));
        }
    }

}

/// Run the websocket code here so that we can use error handling as the websocket handler cannot return a result
async fn run(
    socket: WebSocket,
    db_config: Arc<AppState>,
) -> anyhow::Result<()> {
    tracing::info!("starting orchestration websocket");

    let (sender, mut receiver) = socket.split();
    let sender = Arc::new(Mutex::new(sender));

    tracing::info!("get Init instruction");
    // make sure the client starts with the Init protocol
    let Some(Ok(init)) = receiver.next().await
        else {
            bail!("did not get a message from client");
        };
    let init = match init {
        Message::Binary(b) => {
            let instruction = get_init_protocol(&b).await;
            match instruction {
                Ok(ok) => {
                    // acknowledge
                    let _ = sender.lock().await.send(Message::Text("Receiving instruction OK".into()))
                        .await
                        .context("sending acknowledgement")?;
                    // confirm
                    let run_instruction_res = ok
                        .clone()
                        .run_init()
                        .await?;
                    let serialised_response = serde_json::to_string(&run_instruction_res)?;
                    let _ = sender.lock().await.send(Message::Text(serialised_response.into()))
                        .await
                        .context("sending instruction result")?;
                    // return the init protocol for later code to get deployment info
                    ok
                }
                Err(_) => {
                    let _ = sender.lock().await.send(Message::Close(Some(CloseFrame {
                        code: 1011, // this is error
                        reason: Utf8Bytes::from("Could not deserialise the Init instruction"),
                    }))).await.context("sending close to client websocket")?;
                    bail!("could not deserialise Init instruction");
                }
            }
        }
        _ => {
            let _ = sender.lock().await.send(Message::Close(Some(CloseFrame {
                code: 1011, // this is error
                reason: Utf8Bytes::from("The client did not begin orchestration with an Init instruction"),
            }))).await.context("sending close to client websocket")?;
            bail!("client did not begin with init instruction");
        }
    };

    tracing::info!("getting deployment info");

    // get the deployment info for later
    let (mut deployment, deployment_command) = match init.instruction {
        OrchestrationInstruction::Init { deployment, deployment_command } => {
            let mut deployment = db_config.deployment_config_db
                .read()
                .await
                .get_deployment(deployment.name)
                .await
                .context("getting deployment for orchestration websocket")?;

            (deployment, deployment_command)
        }
        _ => {
            let _ = sender.lock().await.send(Message::Close(Some(CloseFrame {
                code: 1011, // this is error
                reason: Utf8Bytes::from("The client did not begin orchestration with an Init instruction"),
            }))).await.context("sending close to client websocket")?;
            bail!("client did not begin with init instruction");
        }
    };

    // set deployment to running if running destructive commands
    let mut _guard;
    match deployment_command {
        DeploymentCommand::Up { up_cmd: _ } | DeploymentCommand::Down |
        DeploymentCommand::GenerateArtefacts | DeploymentCommand::ClearArtefacts |
        DeploymentCommand::Snapshot { snapshot_cmd: _ } |
        DeploymentCommand::TestbedSnapshot { snapshot_guests: _ } => {
            // these commands are destructive
            _guard = match db_config.try_lock_deployment(deployment.name.clone()) {
                Some(g) => g,
                None => bail!("The deployment {} has a run lock on it", deployment.name)
            };
        }
        _ => {}
    }

    // before we start the run, we will get a lock on the deployment, if we cannot then there is
    // something else running and we will error out here
    // let _guard = match db_config.try_lock_deployment(deployment.name) {
    //     None => {}
    //     Some(_) => {}
    // };


    // create copies that are moved into the async closure
    let deployment_command_copy = deployment_command.clone();
    let deployment_copy = deployment.clone();
    let db_config_copy = db_config.clone();
    let sender_clone = sender.clone();

    // in this loop, we wait for instructions until the client sends a close
    let orchestration_task: anyhow::Result<(anyhow::Result<bool>, bool)> = tokio::spawn(async move {

        tracing::info!("getting project state");
        // get some of the deployment specific data to be used later
        let state = db_config_copy.deployment_config_db
            .read()
            .await
            .get_state(deployment_copy.name)
            .await.context("getting state from provider")?;

        let state = Arc::new(state);
        let (force_provision, force_rerun_scripts, reapply_acl) = match &deployment_command_copy {
            DeploymentCommand::Up { up_cmd } => {
                (up_cmd.provision, up_cmd.rerun_scripts, up_cmd.reapply_acl)
            }
            _ => (false, false, false)
        };
        tracing::info!("getting testbed config");
        let kvm_compose_config = db_config_copy.config_db
            .read()
            .await
            .get_cluster_config()
            .await
            .context("getting testbed cluster config for orchestration job")?;
        let common = get_orchestration_common(
            &state,
            force_provision,
            force_rerun_scripts,
            reapply_acl,
            kvm_compose_config
        ).await?;

        tracing::info!("starting the orchestration listen loop");

        // we record whether an instruction was cancelled, to be handled later
        let mut cancelled = false;

        // channel for the client listener loop to place the instructions for processing
        let (instruction_send_channel, mut instruction_recv_channel) = mpsc::channel(32);
        // channel to control cancellation, to be read at the point of where the commands are being run
        // ... this is mostly important for implementing custom cancel behaviours, beyond stop
        // accepting any further instructions. since most of the testbed instructions are short
        // we can accept when we cancel, the current command will finish
        let (cancel_send_channel, cancel_recv_channel) = mpsc::channel(32);
        let safe_cancel_recv_channel = Arc::new(Mutex::new(cancel_recv_channel));

        // server listener loop, handling the instructions sent from the client, which then checks
        // to run an instruction, cancel or close the connection
        let server_listener_handler_sender = sender_clone.clone();
        let server_listener_handler: JoinHandle<anyhow::Result<bool>> = tokio::spawn(async move {
            loop {

                // get message from server
                let raw_msg = receiver.next().await;
                if let Some(Ok(msg)) = raw_msg {

                    tracing::info!("got message {:?}", msg);

                    // get the instruction from the message, or handle a close web socket message
                    match process_message(msg).await.context("getting instruction from client message")? {
                        Some(instruction) => {
                            // we got an instruction

                            // send the instruction to the other thread or process cancel
                            match instruction.instruction {
                                OrchestrationInstruction::Cancel => {
                                    cancel_send_channel.send(()).await.context("sending cancel signal")?;
                                    // set cancel bool to true to change outcome of run
                                    cancelled = true;
                                    // exit this loop as we will not be receiving any more messages from client
                                    break;
                                }
                                OrchestrationInstruction::End => {
                                    // TODO - the receiver might close itself if there is an error
                                    //  so we might need to ignore a failed End
                                    tracing::info!("End instruction received, breaking out of instruction listener loop");
                                    let _ = instruction_send_channel
                                        .send(Some(instruction))
                                        .await
                                        .context("sending end instruction");
                                    break;
                                }
                                _ => {
                                    tracing::debug!("sending instruction {instruction:?}");
                                    let instruction_res = instruction_send_channel
                                        .send(Some(instruction))
                                        .await
                                        .context("sending instruction");
                                    tracing::debug!("send instruction result {instruction_res:?}");
                                    instruction_res?;
                                }
                            }
                        }
                        None => {
                            // we got a websocket close message
                            tracing::info!("close connection true in orchestration websocket");

                            // tell the instruction channel to close up
                            // the instruction channel might already be closed, since we are in cleanup
                            // of the command running, we can allow this to fail if already closed
                            // TODO - is there an edge case this will bite us?
                            let _ = instruction_send_channel.send(None).await.context("sending close signal to instruction_send_channel");

                            break;
                        }
                    }

                } else {
                    tracing::error!("Could not process the clients instruction, ending command running");
                    let _ = server_listener_handler_sender.lock().await.send(Message::Close(Some(CloseFrame {
                        code: 1011,
                        reason: Utf8Bytes::from("The server could not process the last message, connection closed"),
                    }))).await.context("sending close to client websocket")?;

                    break;
                }

            }
            Ok(cancelled)
        });

        // server instruction handler loop, processing the instructions parsed by the server listener loop
        let server_instruction_handler_sender = sender_clone.clone();
        let server_instruction_handler: JoinHandle<anyhow::Result<_>> = tokio::spawn(async move {

            // loop through all the messages sent to the channel, placed by the future working with
            // the websocket to the client ... this will acknowledge the instruction back to the
            // client and then run the command. Since the channel will only have one instruction
            // at a time due to the protocol from the client-server only processing one instruction
            // at a time, then when the cancel token is placed in the queue it will be immediately
            // consumed, so that will trigger the cancel immediately
            loop {
                tokio::select! {
                    Some(instruction) = instruction_recv_channel.recv() => {
                        match instruction {
                            Some(instruction) => {
                                // send acknowledgement back to client
                                let _ = server_instruction_handler_sender.lock().await.send(Message::Text("Receiving instruction OK".into()))
                                    .await
                                    .context("sending acknowledgement")?;

                                tracing::debug!("received instruction from server listener loop, instruction: {instruction:?}");
                                let run_instruction_res = get_instruction_result(
                                    instruction,
                                    &state,
                                    &common,
                                    server_instruction_handler_sender.clone(),
                                    safe_cancel_recv_channel.clone(),
                                ).await.context("getting result for instruction execution and ws sender")?;
                                tracing::debug!("finished running instruction, next sending client the result of {run_instruction_res:?}");

                                // send to client the result
                                let serialised_response = serde_json::to_string(&run_instruction_res)?;
                                let _ = server_instruction_handler_sender.lock().await.send(Message::Text(serialised_response.into()))
                                    .await
                                    .context("sending instruction result")?;

                                if !run_instruction_res.is_success()? {
                                    tracing::error!("the instruction result was an error, bailing");
                                    bail!("there was a failed orchestration instruction, {run_instruction_res:?}");
                                }

                            }
                            None => {
                                // we got a message, but it was None, meaning there is nothing left to
                                // send from instruction_recv_channel
                                break;
                            }
                        }
                    }
                    else => {
                        // bail!("there was a problem in getting the message from the instruction_recv_channel");
                        // there is no more channel to poll, exit gracefully
                        break;
                    }
                }
            }

            Ok(())
        });

        // instruction runner may have bailed due to error, so we can kill the listener as the
        // client will also have bailed


        // await on both so that we don't continue before both have finished
        let (cancelled, instruction_res) = tokio::try_join!(server_listener_handler, server_instruction_handler)?;

        tracing::info!("end of command running at handler, cancel status: {cancelled:?}");
        tracing::debug!("instruction_res: {instruction_res:?}");

        let command_result = match instruction_res {
            Ok(_) => true,
            Err(_) => false,
        };

        Ok((cancelled, command_result))

    }).await.context("could not join on job task")?;

    // TODO - is there a chance of a race condition between this running and the client checking for the outcome?
    //  it would be in "running" state if the client checks before the server gets a chance

    tracing::info!("end of command running, now determining if it was successful");

    let (was_cancelled, command_result) = match orchestration_task {
        Ok((ref job_result, cmd_res)) => {
            match job_result {
                Ok(cancelled) => (*cancelled, cmd_res),
                Err(_) => (false, cmd_res),
            }
        }
        Err(_) => (false, false),
    };
    tracing::info!("the command running resulted in cancel status of: {was_cancelled}");
    let cancelled = if was_cancelled {
        " and was cancelled".to_string()
    } else {
        "".to_string()
    };


    // we define success here if there were no errors, this means a successful cancel is also a
    // success, despite cancelling technically meaning the command was not successful, since the
    // deployment state is defined on whether the resources are running or not

    let deployment_state = match total_deployment_state(db_config, deployment.name).await {
        Ok(status_json) => {
            match status_json {
                DeploymentStatus::Up => "up".to_string(),
                DeploymentStatus::Partial { .. } => "partial".to_string(),
                DeploymentStatus::Down { .. } => "down".to_string(),
                DeploymentStatus::Running => "running".to_string(),
            }
        }
        Err(_) => {
            "there was an error getting deployment state".to_string()
        }
    };

    let final_response = if was_cancelled {
        if command_result {
            OrchestrationProtocolResponse::Generic {
                is_success: true,
                message: format!("The command was successful{cancelled} and deployment is {deployment_state}"),
            }
        } else {
            OrchestrationProtocolResponse::Generic {
                is_success: false,
                message: format!("The command failed{cancelled} and deployment is {deployment_state}"),
            }
        }
    } else {
        let msg = if command_result {
            ""
        } else {
            "not "
        };
        OrchestrationProtocolResponse::Generic {
            is_success: command_result,
            message: format!("The command was {msg}successful and deployment is {deployment_state}"),
        }
    };
    let serialised_response = serde_json::to_string(&final_response)?;
    tracing::info!("non-destructive command running end - sending final message to client with result of {serialised_response:?}");

    // finally send to the client the result
    let _ = sender.lock().await.send(Message::Text(serialised_response.into()))
        .await
        .context("sending instruction result")?;

    Ok(())
}


/// Get the result of the instruction, but also listen for messages during the execution of the
/// instruction to also pass to the client such as output of commands that were executed or data
/// that the user needs to see in either the CLI or GUI.
async fn get_instruction_result(
    instruction: OrchestrationProtocol,
    state: &State,
    common: &OrchestrationCommon,
    ws_sender: Arc<Mutex<SplitSink<WebSocket, Message>>>,
    cancel_token_recv: Arc<Mutex<Receiver<()>>>,
) -> anyhow::Result<OrchestrationProtocolResponse> {
    // set up channel
    let (logging_send, mut logging_recv) = mpsc::channel(32);

    // we need to return the websocket sender `ws_sender`, since we can only have one with no clones
    // so we need to return it back to the caller of this function
    let logging_task: JoinHandle<anyhow::Result<Arc<Mutex<SplitSink<WebSocket, Message>>>>> = tokio::spawn(async move{
        loop {
            // message received from instruction run, send to user
            if let Some(protocol) = logging_recv.recv().await {
                let serialised_response = serde_json::to_string(&protocol)?;

                // if the logging gets an `End`, then return, we don't need to send to the user
                // a log message saying end
                match protocol {
                    OrchestrationLogger::End => {
                        // end the logging send loop, and return the websocket sender
                        break;
                    }
                    _ => {}
                }

                let _ = ws_sender.lock().await.send(Message::Text(serialised_response.into()))
                    .await
                    .context("sending instruction logging message")?;
            }
        }
        Ok(ws_sender)
    });

    // await for the instruction, which will send an end token at the end of the function:
    // OrchestrationInstruction::run(
    // so that the logging task will close itself, rather than needing us to cancel it
    let instruction_result = instruction.run(&state, &common, &logging_send, cancel_token_recv)
        .await
        .context("getting instruction result")?;
    let _ = logging_task
        .await
        .context("joining on command logging task")?
        .context("getting back websocket sender from logging task")?;

    Ok(instruction_result)
}

async fn get_init_protocol(
    b: &[u8],
) -> anyhow::Result<OrchestrationProtocol> {
    let instruction: OrchestrationProtocol = serde_json::from_slice(b)?;
    Ok(instruction)
}


/// Get the `OrchestrationProtocol` from the message. If the message was a websocket close then we
/// return a `None` but this was still a successful message. If we receive anything else, then that
/// is an error.
async fn process_message(
    msg: Message,
) -> anyhow::Result<Option<OrchestrationProtocol>> {
    match msg {
        Message::Binary(b) => {
            // deserialise
            let instruction: OrchestrationProtocol = serde_json::from_slice(&b)?;
            tracing::info!("orchestration got instruction: {:?}", &instruction.instruction);

            Ok(Some(instruction))
        }
        Message::Close(_) => {
            Ok(None)
        }
        _ => bail!("unsupported message type"),
    }
}
