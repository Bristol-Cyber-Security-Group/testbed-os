use serde::{Deserialize, Serialize};

/// Helper model to support the pretty=true query string to return configuration as pretty JSON
#[derive(Serialize, Deserialize)]
pub struct PrettyQueryParams {
    pub pretty: Option<bool>,
}
