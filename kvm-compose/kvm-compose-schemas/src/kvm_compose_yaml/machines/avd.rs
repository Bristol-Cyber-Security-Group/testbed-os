use std::collections::HashMap;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use validator::Validate;
use crate::kvm_compose_yaml::machines::ConfigScalingInterface;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ConfigAVDMachine {
    // while this is not the ip of the android device, its the ip of the veth in the namespace
    pub static_ip: Option<String>,
    #[serde(flatten)]
    pub avd_type: AVDGuestOptions,
    // scaling - wont be implemented yet, while we know how to do it the actual user experience
    // in controlling many AVDs is not yet easy
    pub scaling: Option<AndroidScaling>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum AVDGuestOptions {
    Avd {
        android_api_version: u8,
        playstore_enabled: bool,
    },
    ExistingAvd {
        path: PathBuf,
    }
}

// #[derive(Deserialize, Serialize, Debug, Clone)]
// pub enum AndroidAPIVersions {
//     Android28,
// }

#[derive(Deserialize, Serialize, Debug, Validate, Clone)]
pub struct AndroidScaling {
    #[validate(range(min = 1))]
    pub count: u32,
    #[validate(length(min = 1))]
    pub interfaces: HashMap<String, ConfigScalingInterface>,
}
