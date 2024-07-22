use std::collections::HashMap;
use std::fmt;
use std::fmt::Formatter;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ACL {
    /// Place a low priority deny all traffic policy, with the expectation that the user will place
    /// rules to selectively allow traffic. Default is false to allow all traffic by default.
    #[serde(default)]
    pub apply_deny_all: bool,
    pub switches: HashMap<String, Vec<ACLRule>>,
    // pub port_group:
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ACLRule {
    pub direction: ACLDirection,
    /// Must be number between 0 and 32,767
    pub priority: i16,
    #[serde(rename = "match")]
    pub _match: String,
    pub action: ACLAction,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum ACLDirection {
    #[serde(rename = "to-lport")]
    ToLport,
    #[serde(rename = "from-lport")]
    FromLport,
}

impl fmt::Display for ACLDirection {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let text = match self {
            ACLDirection::ToLport => "to-lport".to_string(),
            ACLDirection::FromLport => "from-lport".to_string(),
        };
        f.write_str(&text)
            .expect("Pretty printing ACLDirection failed");
        Ok(())
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum ACLAction {
    #[serde(rename = "allow-related")]
    AllowRelated,
    #[serde(rename = "allow-stateless")]
    AllowStateless,
    #[serde(rename = "allow")]
    Allow,
    #[serde(rename = "drop")]
    Drop,
    #[serde(rename = "pass")]
    Pass,
    #[serde(rename = "reject")]
    Reject,
}

impl fmt::Display for ACLAction {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let text = match self {
            ACLAction::AllowRelated => "allow-related".to_string(),
            ACLAction::AllowStateless => "allow-stateless".to_string(),
            ACLAction::Allow => "allow".to_string(),
            ACLAction::Drop => "drop".to_string(),
            ACLAction::Pass => "pass".to_string(),
            ACLAction::Reject => "reject".to_string(),
        };
        f.write_str(&text)
            .expect("Pretty printing ACLAction failed");
        Ok(())
    }
}
