use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Router {
    pub ports: Option<Vec<RouterPort>>,
    pub static_routes: Option<Vec<StaticRoutes>>,
    pub nat: Option<Vec<NAT>>,
    pub dhcp: Option<Vec<Dhcp>>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct RouterPort {
    pub name: String,
    pub mac: String,
    pub gateway_ip: String,
    pub switch: String,
    pub set_gateway_chassis: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct StaticRoutes {
    pub prefix: String,
    pub nexthop: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct NAT {
    pub nat_type: NatType, // TODO make this enum
    pub external_ip: String,
    pub logical_ip: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum NatType {
    DnatAndSnat,
    Snat,
    // Dnat,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub struct Dhcp {
    pub switch: String,
    pub exclude_ips: ExcludeIps,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub struct ExcludeIps {
    pub from: String,
    pub to: String,
}
