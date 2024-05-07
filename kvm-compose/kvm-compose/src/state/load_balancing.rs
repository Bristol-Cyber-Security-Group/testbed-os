use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum BridgeConnection {
    Ovs {
        name: String, // TODO - redundant?, can remove for ovs only
        source_br: String,
        target_br: String,
        source_veth: String,
        target_veth: String,
        ip: Option<String>,
        testbed_host: String,
    },
    Tunnel {
        source_br: String,
        target_br: String,
        source_remote_ip: Option<String>,
        target_remote_ip: Option<String>,
        key: Option<String>,
        source_br_ip: Option<String>,
        target_br_ip: Option<String>,
        testbed_host_source: String,
        testbed_host_target: String,
    },
}

#[derive(Debug, Clone)]
pub struct LoadBalancedTopology {
    // host_bridge_map: BTreeMap<String, String>,
    pub bridge_host_map: BTreeMap<String, String>,
    pub guest_host_map: BTreeMap<String, String>,
}
