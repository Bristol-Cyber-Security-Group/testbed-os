use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::kvm_compose_yaml::network::acl::ACL;
use crate::kvm_compose_yaml::network::router::Router;
use crate::kvm_compose_yaml::network::switch::Switch;

pub mod switch;
pub mod router;
pub mod acl;

// TODO - semantic validation of inputs when converting into "state"

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct OvnNetworkSchema {
    pub switches: Option<HashMap<String, Switch>>,
    pub routers: Option<HashMap<String, Router>>,
    pub acl: Option<ACL>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct OvsNetwork {

}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum NetworkBackend {
    Ovn(OvnNetworkSchema),
    Ovs(OvsNetwork),
}
