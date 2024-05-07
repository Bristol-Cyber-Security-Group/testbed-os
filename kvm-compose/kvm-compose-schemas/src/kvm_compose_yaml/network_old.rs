use serde::{Deserialize, Serialize};

// #[derive(Deserialize, Serialize, Debug, Clone)]
// pub struct Network {
//     pub bridges: Option<Vec<ConfigBridge>>,
//     #[serde(default)]
//     pub bridge_connections: BTreeMap<String, String>,
//     pub external_bridge: Option<String>,
//     // TODO - allow setting the subnet for the testbed
//
//     // a list of network interfaces that should be connected to the testbed
//     pub network_interfaces: Option<Vec<NetworkInterface>>,
// }

// #[derive(Deserialize, Serialize, Debug, Validate, Clone)]
// pub struct ConfigBridge {
//     #[validate(length(min = 1), custom = "constrain_bridge_name_len")]
//     pub name: String,
//     pub controller: Option<String>,
//     pub protocol: Option<String>,
//     // TODO - allow assigning an IP to the bridge
// }

// fn constrain_bridge_name_len(bridge_name: &str) -> Result<(), ValidationError> {
//     // this is a stop gap before addressing state validation issues #22 #41
//     return if bridge_name.len() > 5 {
//         Err(ValidationError {
//             code: Cow::from("bridge names cannot be longer than 5 characters"),
//             message: None,
//             params: HashMap::new(),
//         })
//     } else {
//         Ok(())
//     };
// }

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct NetworkInterface {
    // name of the interface on system
    pub nic_name: String,
    // // desired static ip address
    // pub ip_address: Option<String>,
    // which testbed bridge to assign to the interface
    pub testbed_bridge: String,
    // which testbed host is the interface on
    pub testbed_host: String,
}
