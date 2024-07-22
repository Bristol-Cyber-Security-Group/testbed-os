use std::borrow::Cow;
use std::sync::Arc;
use anyhow::{bail, Context};
use axum::extract::ws::{CloseFrame, Message, WebSocket};
use futures_util::{SinkExt, StreamExt};
use futures_util::stream::SplitSink;
use tokio::sync::{mpsc, Mutex};
use tokio::task::JoinHandle;
use kvm_compose_lib::orchestration::api::{OrchestrationInstruction, OrchestrationLogger, OrchestrationProtocol, OrchestrationProtocolResponse};
use kvm_compose_lib::orchestration::{OrchestrationCommon};
use kvm_compose_lib::state::orchestration_tasks::get_orchestration_common;
use kvm_compose_lib::state::State;
use kvm_compose_schemas::deployment_models::{DeploymentCommand, DeploymentState};
use crate::AppState;

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
                    let _ = sender.lock().await.send(Message::Text("Receiving instruction OK".to_string()))
                        .await
                        .context("sending acknowledgement")?;
                    // confirm
                    let run_instruction_res = ok
                        .clone()
                        .run_init()
                        .await?;
                    let serialised_response = serde_json::to_string(&run_instruction_res)?;
                    let _ = sender.lock().await.send(Message::Text(serialised_response))
                        .await
                        .context("sending instruction result")?;
                    // return the init protocol for later code to get deployment info
                    ok
                }
                Err(_) => {
                    let _ = sender.lock().await.send(Message::Close(Some(CloseFrame {
                        code: 1011, // this is error
                        reason: Cow::from("Could not deserialise the Init instruction"),
                    }))).await.context("sending close to client websocket")?;
                    bail!("could not deserialise Init instruction");
                }
            }
        }
        _ => {
            let _ = sender.lock().await.send(Message::Close(Some(CloseFrame {
                code: 1011, // this is error
                reason: Cow::from("The client did not begin orchestration with an Init instruction"),
            }))).await.context("sending close to client websocket")?;
            bail!("client did not begin with init instruction");
        }
    };

    tracing::info!("getting deployment info");

    // get the deployment info for later
    let (mut deployment, previous_state, deployment_command) = match init.instruction {
        OrchestrationInstruction::Init { deployment, deployment_command } => {
            let mut deployment = db_config.deployment_config_db
                .read()
                .await
                .get_deployment(deployment.name)
                .await
                .context("getting deployment for orchestration websocket")?;

            let previous_state = deployment.state;
            // set deployment to running
            deployment.state = DeploymentState::Running;
            db_config.deployment_config_db
                .write()
                .await
                .update_deployment(deployment.name.clone(), deployment.clone())
                .await
                .context("updating deployment to running state")?;
            (deployment, previous_state, deployment_command)
        }
        _ => {
            let _ = sender.lock().await.send(Message::Close(Some(CloseFrame {
                code: 1011, // this is error
                reason: Cow::from("The client did not begin orchestration with an Init instruction"),
            }))).await.context("sending close to client websocket")?;
            bail!("client did not begin with init instruction");
        }
    };

    // create copies that are moved into the async closure
    let deployment_command_copy = deployment_command.clone();
    let deployment_copy = deployment.clone();
    let db_config_copy = db_config.clone();

    // in this loop, we wait for instructions until the client sends a close
    let orchestration_task: anyhow::Result<()> = tokio::spawn(async move {

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


        loop {

            // make copies (new ref as it's an Arc) in the loop so that we can move them
            let loop_state = state.clone();
            let loop_common = common.clone();
            let loop_sender = sender.clone();
            let loop_sender_cancel = sender.clone();

            // get message from server
            let raw_msg = receiver.next().await;
            // make sure it is ok and not empty
            if let Some(Ok(msg)) = raw_msg {

                // now we want to execute a command based on this message, but we might also receive
                // a cancellation token ... the receiver must be free to listen to this command ...
                // we also do not expect any other message over the socket from the client while a
                // command is running

                // if this errors, it is either due to orchestration failure or non cancellation
                // token was received
                let close_connection_bool_result = tokio::select! {
                    // process client instruction and return whether we continue
                    close = process_client_instruction(msg, loop_sender, loop_state, loop_common) => close,
                    // process potential cancelation token
                    Some(Ok(maybe_cancel_token)) = receiver.next() => process_potential_cancel_token(maybe_cancel_token, loop_sender_cancel.clone()).await
                };

                let close_connection_bool = close_connection_bool_result
                    .context("determining if the orchestration loop should continue")?;

                // got a close result, exit loop
                if close_connection_bool {
                    tracing::info!("close connection true in orchestration websocket");

                    // normal close
                    // connection might already be closed by client so don't handle error with ?
                    let _ = loop_sender_cancel.lock().await.send(Message::Close(Some(CloseFrame {
                        code: 1000,
                        reason: Cow::from("Last command received, connection closed"),
                    }))).await.context("sending close to client websocket"); // TODO - need ? here?

                    break;
                }

            } else {
                // message from client was not Ok
                let _ = loop_sender_cancel.lock().await.send(Message::Close(Some(CloseFrame {
                    code: 1011,
                    reason: Cow::from("The server could not process the last message, connection closed"),
                }))).await.context("sending close to client websocket")?;

                break;
            }
        }
        tracing::info!("end of orchestration connection");

        Ok(())

    }).await.context("could not join on job task")?;

    // TODO - is there a chance of a race condition between this running and the client checking for the outcome?
    //  it would be in "running" state if the client checks before the server gets a chance

    // determine the outcome of the job
    match orchestration_task {
        Ok(_) => {
            // get deployment config and set to success for whatever the command was
            match deployment_command {
                DeploymentCommand::Up { .. } => {
                    deployment.state = DeploymentState::Up;
                    db_config.deployment_config_db
                        .write()
                        .await
                        .update_deployment(deployment.name.clone(), deployment)
                        .await
                        .context("updating deployment to up state")?;
                }
                DeploymentCommand::Down => {
                    deployment.state = DeploymentState::Down;
                    db_config.deployment_config_db
                        .write()
                        .await
                        .update_deployment(deployment.name.clone(), deployment)
                        .await
                        .context("updating deployment to down state")?;
                }
                DeploymentCommand::ClearArtefacts => {
                    // should be down due to clear artefacts implementation
                    deployment.state = DeploymentState::Down;
                    db_config.deployment_config_db
                        .write()
                        .await
                        .update_deployment(deployment.name.clone(), deployment)
                        .await
                        .context("updating deployment to down state")?;
                }
                _ => {
                    // set to previous state
                    deployment.state = previous_state;
                    db_config.deployment_config_db
                        .write()
                        .await
                        .update_deployment(deployment.name.clone(), deployment)
                        .await
                        .context("updating deployment to previous state")?;
                }
            }
        }
        Err(err) => {
            tracing::error!("error in orchestration websocket: {err:#}");
            // set state to failed with the deployment command attempted
            deployment.state = DeploymentState::Failed(deployment_command);
            db_config.deployment_config_db
                .write()
                .await
                .update_deployment(deployment.name.clone(), deployment)
                .await
                .context("updating deployment to failed state")?;
        }
    }

    Ok(())
}

/// Get the result of the instruction, but also listen for messages during the execution of the
/// instruction to also pass to the client such as output of commands that were executed or data
/// that the user needs to see in either the CLI or GUI.
async fn get_instruction_result(
    instruction: OrchestrationProtocol,
    state: &State,
    common: &OrchestrationCommon,
    ws_sender: Arc<Mutex<SplitSink<WebSocket, Message>>>
) -> anyhow::Result<(OrchestrationProtocolResponse, Arc<Mutex<SplitSink<WebSocket, Message>>>)> {
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

                let _ = ws_sender.lock().await.send(Message::Text(serialised_response))
                    .await
                    .context("sending instruction logging message")?;
            }
        }
        Ok(ws_sender)
    });

    // await for the instruction, which will send an end token at the end of the function:
    // OrchestrationInstruction::run(
    // so that the logging task will close itself, rather than needing us to cancel it
    let instruction_result = instruction.run(&state, &common, &logging_send)
        .await
        .context("getting instruction result")?;
    let ws_sender = logging_task
        .await
        .context("joining on command logging task")?
        .context("getting back websocket sender from logging task")?;


    Ok((instruction_result, ws_sender))
}

async fn get_init_protocol(
    b: &[u8],
) -> anyhow::Result<OrchestrationProtocol> {
    let instruction: OrchestrationProtocol = serde_json::from_slice(b)?;
    Ok(instruction)
}


async fn process_client_instruction(
    msg: Message,
    loop_sender: Arc<Mutex<SplitSink<WebSocket, Message>>>,
    loop_state: Arc<State>,
    loop_common: OrchestrationCommon
) -> anyhow::Result<bool> {
    match msg {
        Message::Binary(b) => {
            // deserialise
            let instruction: OrchestrationProtocol = serde_json::from_slice(&b)?;
            tracing::info!("orchestration got instruction: {:?}", &instruction.instruction);

            let _ = loop_sender.lock().await.send(Message::Text("Receiving instruction OK".to_string()))
                .await
                .context("sending acknowledgement")?;

            // run the instruction, if the mpsc channel returns a message from the instruction
            // while it is running, process that and then resume waiting for the instruction to run
            // ... this will also handle sending any important log messages during the
            // execution of the instruction to the client, that aren't the final state of
            // the instruction result, as seen below `serialised_response`
            let (run_instruction_res, loop_sender) = get_instruction_result(
                instruction,
                &loop_state,
                &loop_common,
                loop_sender.clone(),
            ).await.context("getting result for instruction execution and ws sender")?;

            // send to client the result
            let serialised_response = serde_json::to_string(&run_instruction_res)?;
            let _ = loop_sender.lock().await.send(Message::Text(serialised_response))
                .await
                .context("sending instruction result")?;

            if !run_instruction_res.is_success()? {
                bail!("there was a failed orchestration instruction, {run_instruction_res:?}");
            }

            // dont close
            Ok(false)

        }
        Message::Close(_) => {
            return Ok(true);
        }
        _ => Ok(true), // TODO - unexpected message?
    }
}

async fn process_potential_cancel_token(
    maybe_cancel_token: Message,
    loop_sender_cancel: Arc<Mutex<SplitSink<WebSocket, Message>>>,
) -> anyhow::Result<bool> {

    match maybe_cancel_token {
        Message::Binary(b) => {

            let instruction: OrchestrationProtocol = serde_json::from_slice(&b)?;
            tracing::info!("orchestration got instruction in cancellation listener: {:?}", &instruction.instruction);

            match instruction.instruction {
                OrchestrationInstruction::Cancel => {}
                _ => {
                    // was not cancellation token, for now we will throw an error and kill the
                    // command running as the following commands will be out of order for the
                    // orchestration protocol
                    // there is a strict order of commands from the client to the server
                    bail!("got a non cancellation token in the cancellation listener, killing orchestration")
                }
            }

            // as this is processing the cancel request during an instruction, rather than being
            // processed between, we should send the client back the same response
            let serialised_response = serde_json::to_string(&OrchestrationProtocolResponse::Generic {
                is_success: false,
                message: "Cancel request".to_string(),
            })?;
            let _ = loop_sender_cancel.lock().await.send(Message::Text(serialised_response))
                .await
                .context("sending instruction result")?;

            tracing::info!("client has sent a cancellation token, closing connection");
            let _ = loop_sender_cancel.lock().await.send(Message::Close(Some(CloseFrame {
                code: 1000,
                reason: Cow::from("The client sent a cancellation token, connection closed"),
            }))).await.context("sending close to client websocket")?;
            Ok(true)
        }
        Message::Close(_) => {
            return Ok(true);
        }
        _ => Ok(true), // TODO - unexpected message?
    }
}
