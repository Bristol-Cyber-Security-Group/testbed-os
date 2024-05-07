use std::net::SocketAddr;
use std::sync::Arc;
use axum::extract::{ConnectInfo, State, WebSocketUpgrade};
use axum_extra::{TypedHeader};
use axum::response::IntoResponse;
use axum_extra::headers::UserAgent;
use crate::{AppError, AppState};
use crate::gui::websocket::handle_gui_orchestration_socket;
use crate::orchestration::websocket::handle_orchestration_socket;

pub async fn orchestration_websocket_handler(
    State(db_config): State<Arc<AppState>>,
    ws: WebSocketUpgrade,
    user_agent: Option<TypedHeader<UserAgent>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> Result<impl IntoResponse, AppError> {
    let user_agent = if let Some(TypedHeader(user_agent)) = user_agent {
        user_agent.to_string()
    } else {
        // TODO - differentiate from CLI and web GUI?
        String::from("Unknown browser")
    };
    tracing::info!("`{user_agent}` at {addr} connected.");

    // error handling is done in the handle socket function, depending on the outcome the database is updated and the
    // client will check the outcome separately, this function result should only be related to the graceful close of
    // the socket
    Ok(ws.on_upgrade(move |socket| {
        handle_orchestration_socket(socket, db_config.clone())
    }))
}


// pub async fn preflight_setup(
//     State(db_config): State<Arc<AppState>>,
//     Json(resource): Json<OrchestrationProtocol>,
// ) -> Result<impl IntoResponse, AppError> {
//     // TODO could return an http code 422 on error
//     let resource = test_for_setup_command(resource).await?;
//     let deployment_name = &resource.common.project_name;
//     tracing::info!("received request to setup deployment {deployment_name}");
//
//     // TODO check_if_testbed_hosts_up
//     // TODO create_remote_project_folders
//
//     Ok(())
// }

// pub async fn teardown()


/// This handler wraps the CLI code to make it available to the GUI. This means the process the CLI goes through in
/// reading the yaml then preparing the state file, is possible through the GUI via the server. This means the CLI code
/// is re-used, and we don't have to re-implement the orchestration websocket protocol in javascript. This does mean
/// that the server is essentially opening up a websocket to itself.
pub async fn gui_orchestration_websocket_handler(
    State(db_config): State<Arc<AppState>>,
    ws: WebSocketUpgrade,
    user_agent: Option<TypedHeader<UserAgent>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> Result<impl IntoResponse, AppError> {

    tracing::info!("webclient connected, {user_agent:?}, {addr}");

    Ok(ws.on_upgrade(move |socket| {
        handle_gui_orchestration_socket(socket, db_config.clone())
    }))

}

// /// This handler wraps the CLI code specifically for the exec commands, to make it available to the GUI. Due to the
// /// exec commands via the CLI directly manipulating files and needing root permissions, we need to run this via the
// /// server. This is a more involved version of `gui_orchestration_websocket_handler` as there will be more of the CLI
// /// re-implemented here, as the GUI cannot run any of the exec commands directly due to filesystem and root access.
// pub async fn gui_exec_websocket_handler(
//     State(db_config): State<Arc<AppState>>,
//     ws: WebSocketUpgrade,
//     user_agent: Option<TypedHeader<headers::UserAgent>>,
//     ConnectInfo(addr): ConnectInfo<SocketAddr>,
// ) -> Result<impl IntoResponse, AppError> {
//     unimplemented!()
// }
