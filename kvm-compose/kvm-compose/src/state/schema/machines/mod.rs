pub mod libvirt;
pub mod docker;
pub mod avd;

use serde::{Deserialize, Serialize};
use kvm_compose_schemas::kvm_compose_yaml::machines::{ConfigScalingInterface, ConfigScalingIpRange, ConfigScalingIpType, ConfigScalingMacRange};
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct StateScalingInterface {
    pub clones: Vec<u32>,
    pub gateway: Option<String>,
    pub ip_type: StateScalingIpType,
    pub mac_range: StateScalingMacRange,
}

impl From<ConfigScalingInterface> for StateScalingInterface {
    fn from(interface: ConfigScalingInterface) -> Self {
        Self {
            clones: interface.clones.clone(),
            gateway: interface.gateway.clone(),
            ip_type: interface.ip_type.into(),
            mac_range: interface.mac_range.into(),
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum StateScalingIpType {
    IpRange(StateScalingIpRange),
    Dynamic,
}

impl From<ConfigScalingIpType> for StateScalingIpType {
    fn from(ip_type: ConfigScalingIpType) -> Self {
        match ip_type {
            ConfigScalingIpType::IpRange(ip) => Self::IpRange(ip.into()),
            ConfigScalingIpType::Dynamic => Self::Dynamic,
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct StateScalingIpRange {
    pub from: String,
    pub to: String,
}

impl From<ConfigScalingIpRange> for StateScalingIpRange {
    fn from(ip_range: ConfigScalingIpRange) -> Self {
        Self {
            from: ip_range.from,
            to: ip_range.to,
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct StateScalingMacRange {
    pub from: String,
    pub to: String,
}

impl From<ConfigScalingMacRange> for StateScalingMacRange {
    fn from(mac_range: ConfigScalingMacRange) -> Self {
        Self {
            from: mac_range.from,
            to: mac_range.to,
        }
    }
}

