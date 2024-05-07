pub mod avd;
pub mod docker;
pub mod libvirt;
pub mod libvirt_image_download;

use crate::kvm_compose_yaml::machines::avd::ConfigAVDMachine;
use crate::kvm_compose_yaml::machines::docker::ConfigDockerMachine;
use crate::kvm_compose_yaml::machines::libvirt::ConfigLibvirtMachine;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum GuestType {
    Libvirt(ConfigLibvirtMachine),
    Docker(ConfigDockerMachine),
    Android(ConfigAVDMachine),
}

impl GuestType {
    pub fn name(&self) -> String {
        match self {
            GuestType::Libvirt(_) => "Libvirt".into(),
            GuestType::Docker(_) => "Docker".into(),
            GuestType::Android(_) => "Android".into(),
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ConfigScalingInterface {
    pub clones: Vec<u32>,
    pub gateway: Option<String>,
    pub ip_type: ConfigScalingIpType,
    pub mac_range: ConfigScalingMacRange,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum ConfigScalingIpType {
    IpRange(ConfigScalingIpRange),
    Dynamic,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ConfigScalingIpRange {
    pub from: String,
    pub to: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ConfigScalingMacRange {
    pub from: String,
    pub to: String,
}
