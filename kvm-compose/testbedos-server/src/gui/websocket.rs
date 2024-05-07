use std::borrow::Cow;
use std::sync::Arc;
use anyhow::{bail, Context, Error};
use axum::extract::ws::{CloseFrame, Message, WebSocket};
use futures_util::{SinkExt, StreamExt};
use futures_util::stream::{SplitSink};
use tokio::sync::{mpsc};
use tokio::sync::mpsc::Receiver;
use kvm_compose_lib::orchestration::api::{OrchestrationInstruction, OrchestrationProtocol};
use kvm_compose_lib::orchestration::orchestrator::run_orchestration;
use kvm_compose_lib::server_client::client::get_deployment_action;
use kvm_compose_schemas::cli_models::{Opts};
use kvm_compose_schemas::gui_models::{GUICommand, GUIResponse};
use crate::AppState;


pub async fn handle_gui_orchestration_socket(
    socket: WebSocket,
    db_config: Arc<AppState>,
) {
    match run(socket, db_config).await {
        Ok(_) => {
            tracing::info!("end of gui orchestration websocket, closing socket");
        }
        Err(err) => {
            tracing::error!("web orchestration failed, closing socket");
            err.chain().for_each(|cause| tracing::error!("because: {}", cause));
        }
    }
}

async fn run(
    socket: WebSocket,
    db_config: Arc<AppState>,
) -> anyhow::Result<()> {
    let (mut sender, mut receiver) = socket.split();
    let Some(Ok(init)) = receiver.next().await else {bail!("did not get a message from client")};
    let init = match init {
        Message::Text(t) => {
            tracing::info!("message from web client: {t}");
            get_init_protocol(t)
        }
        _ => {
            tracing::info!("incorrect message from web client");
            // not correct init msg
            let _ = sender.send(Message::Close(Some(CloseFrame {
                code: 1011, // this is error
                reason: Cow::from("The web client did not begin command with an Init instruction"),
            }))).await.context("sending close to client websocket")?;
            bail!("web client did not begin with init instruction");
        }
    }.context("parsing command was not successful");

    // acknowledge to the web client if the init request was successful or not
    let init_response = create_init_request_response(&init);
    sender.send(Message::Text(init_response.to_string_json())).await?;
    if init_response.was_error == true {
        // request was invalid, don't continue and close websocket
        return Ok(())
    }

    // // DEBUGGING
    // // use these two lines if you want to test just the command being validated but skipping the command running
    // sender.send(Message::Text(create_gui_response("debug: command accepted by server, closing connection".to_string(), false))).await?;
    // return Ok(());

    // we now know init is OK
    let gui_command = init?;


    let project_name = gui_command.project_name.clone();

    // get deployment, or tell web client it doesn't exist and close connection
    let deployment = match db_config.deployment_config_db
        .read()
        .await
        .get_deployment(project_name.clone())
        .await
    {
        Ok(ok) => ok,
        Err(_) => {
            let _ = sender.send(Message::Close(Some(CloseFrame {
                code: 1011, // this is error
                reason: Cow::from(format!("Error: The project named '{}' does not exist", project_name.clone())),
            }))).await.context("sending close to client websocket")?;
            bail!("web client did not begin with init instruction");
        }
    };

    // need to recreate the Opts struct
    let opts = Opts {
        input: "kvm-compose.yaml".to_string(), // assume from GUI always this
        project_name: Some(gui_command.project_name.clone()),
        verbosity: Some("Info".to_string()),
        no_ask: true,
        sub_command: gui_command.sub_command,
        server_connection: "http://localhost:3355/".to_string(),
    };

    // here we run the orchestration command generation, so that we can send these commands to the
    // GUI, which will then send back to the server on the orchestration websocket like the CLI
    // would do ...
    // so orchestration_cmd_generation_thread generates the commands via the mpsc channel
    // orchestration_message_cmd_receiver_thread gets the commands as they come from the mpsc channel
    //  and then relays them to the GUI ...
    // the GUI will separately send these commands on a different websocket

    // create orchestration message channel
    let (orchestration_send, orchestration_recv) = mpsc::channel(32);
    let mut orchestration_send_clone = orchestration_send.clone();

    let orchestration_cmd_generation_thread = tokio::spawn(async move {
        // TODO - this assumes the messages were OK, so the deployment state should
        //  really come from the message sender
        // TODO - we dont use the deployment result here
        run_orchestration(
            deployment.clone(),
            get_deployment_action(&opts).context("get deployment action")?,
            opts,
            &mut orchestration_send_clone,
            // &mut orchestration_recv_resub,
        ).await?;
        // tell channel that orchestration message sending is done
        // regardless if `run_orchestration` was successful or not to prevent waiting indefinitely
        orchestration_send_clone.send(OrchestrationProtocol { instruction: OrchestrationInstruction::End }).await?;
        tracing::info!("end of command generation thread");

        Ok(())
    });


    // let container = websocket_container.clone();
    let orchestration_message_cmd_receiver_thread = tokio::spawn(async move {
        orchestration_message_handler(
            orchestration_recv,
            &mut sender,
        ).await?;
        Ok(())

    });


    let res: anyhow::Result<()> = orchestration_message_cmd_receiver_thread.await?;
    match res {
        Ok(_) => {}
        Err(err) => {
            bail!(err);
        },
    }

    let res: anyhow::Result<()> = orchestration_cmd_generation_thread.await?;
    match res {
        Ok(_) => {}
        Err(err) => {
            bail!(err);
        },
    }

    Ok(())
}

/// Parse the init command from the GUI with the intended command to be executed for orchestration.
/// This will be the various commands like up/down etc.
fn get_init_protocol(
    t: String,
) -> anyhow::Result<GUICommand> {
    let instruction: GUICommand = serde_json::from_str(&t)?;
    Ok(instruction)
}

/// Create a response to the GUI for the init request to start orchestration. If the command was
/// correct, then we can continue.
fn create_init_request_response(
    init_state: &Result<GUICommand, Error>,
) -> GUIResponse {
    let init_success_response = match init_state {
        Ok(cmd) => {
            let json = cmd.to_string_json();
            GUIResponse {
                init_msg: true,
                message: json,
                was_error: false,
            }
        },
        Err(err) => GUIResponse {
            init_msg: true,
            message: err.to_string(),
            was_error: true,
        },
    };
    init_success_response
}

async fn orchestration_message_handler(
    mut orchestration_recv: Receiver<OrchestrationProtocol>,
    sender: &mut SplitSink<WebSocket, Message>
) -> anyhow::Result<()> {
    // check orchestration message
    loop {
        if let Some(protocol) = orchestration_recv.recv().await {
            // end message is always sent if run_orchestration is successful or not
            match protocol.instruction {
                OrchestrationInstruction::End => {
                    // if command generation is matched, then we can end this future
                    tracing::info!("end of command receiver thread");
                    return Ok(());
                }
                _ => {}
            }

            sender.send(Message::Text(protocol.to_string()?)).await?;

        } else {
            bail!("exiting socket send loop, message not Ok");
        }
    }
}
