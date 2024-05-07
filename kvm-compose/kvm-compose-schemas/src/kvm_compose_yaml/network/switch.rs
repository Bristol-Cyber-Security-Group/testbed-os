use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Switch {
    pub subnet: String,
    pub ports: Option<Vec<SwitchPort>>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct SwitchPort {
    pub name: String,
    pub chassis: Option<String>, // optionally bind port to a host
    // pub ip: Option<IpAddr>,
    // pub mac: Option<String>,
    #[serde(flatten)]
    pub port_type: SwitchPortType,
    // pub options: SwitchPortOptions,
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct SwitchPortOptions {
    pub network_name: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum SwitchPortType {
    Internal {
        ip: Option<String>,
        mac: Option<String>,
        network_name: Option<String>,
    },
    Router {
        mac: Option<String>,
        router_port: String,
    },
    Localnet {
        network_name: String,
    },
}
