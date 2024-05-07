use serde::{Deserialize, Serialize};
use crate::cli_models::SubCommand;

/// This model is used to be able to de-serialise the command coming from the GUI to initiate some
/// orchestration or exec command on the server. Re-using existing code from the CLI schema, but not
/// all the sub commands will be available to the GUI in the code that processes this model
#[derive(Debug, Deserialize, Serialize)]
pub struct GUICommand {
    pub project_name: String,
    pub sub_command: SubCommand
}

impl GUICommand {
    pub fn to_string_json(&self) -> String {
        get_json_or_err(self)
    }
}

/// This model is used to define the response back to the web GUI. There will be a set of pre-determined
/// commands that are embedded inside the text/binary message. This is not a substitute to the websocket
/// message types.
#[derive(Debug, Deserialize, Serialize)]
pub struct GUIResponse {
    pub init_msg: bool,
    pub message: String,
    pub was_error: bool,
    // This tells the GUI to request action from the user to respond
    // pub interactive_instruction:
}

impl GUIResponse {
    pub fn to_string_json(&self) -> String {
        get_json_or_err(self)
    }
}

/// Helper generic to serialise the GUI structs into JSON. We capture any serialisation errors as a string to be sent to
/// the caller.
fn get_json_or_err<T: Serialize>(data: &T) -> String {
    serde_json::to_string(data).unwrap_or_else(|err| format!("{err:#}"))
}

/// This model is used in orchestration optionally when triggered via the GUI. Messages from the
/// orchestration futures that are usually sent to the CLI will be sent over a channel to be
/// forwarded to the GUI.
#[derive(Clone)]
pub struct IPCGUIMessage {
    pub message: String,
    pub end_of_messages: bool,
    pub was_error: bool,
}

/// This model represents the deployment create from the GUI page, as a form.
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct GUICreateDeploymentJson {
    pub yaml: String,
    pub project_name: String,
}
