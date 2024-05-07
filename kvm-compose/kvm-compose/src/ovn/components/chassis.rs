use crate::ovn::components::ovs::{OvsBridge, OvsSystem};

/// This represents a chassis in OVN
pub struct OvnChassis {
    pub name: String,
    pub ovs_system: OvsSystem,
    /// The bridge where guests will attack to
    pub integration_bridge: OvsBridge,
    /// The bridge that is used to connect the logical network with a network on the host
    pub provider_bridge: Option<OvsBridge>,
}

// TODO - need to check what exists in the DB before running any create etc

// TODO - need to cross reference the kvm-compose-config json to see what needs to be added/removed
//  if we can communicate with the other chassis
