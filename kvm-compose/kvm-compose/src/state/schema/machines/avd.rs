use std::collections::HashMap;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use validator::Validate;
use kvm_compose_schemas::kvm_compose_yaml::machines::avd::{AVDGuestOptions, AndroidScaling, ConfigAVDMachine};
use kvm_compose_schemas::kvm_compose_yaml::machines::ConfigScalingInterface;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct StateAVDMachine {
    // while this is not the ip of the android device, its the ip of the veth in the namespace
    pub static_ip: Option<String>,
    #[serde(flatten)]
    pub avd_type: StateAVDGuestOptions,
    // scaling - wont be implemented yet, while we know how to do it the actual user experience
    // in controlling many AVDs is not yet easy
    pub scaling: Option<StateAndroidScaling>,
}

impl From<ConfigAVDMachine> for StateAVDMachine {
    fn from(config: ConfigAVDMachine) -> Self {
        Self {
            static_ip: config.static_ip,
            avd_type: config.avd_type.into(),
            scaling: config.scaling.map(|s| s.into()),
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum StateAVDGuestOptions {
    Avd {
        android_api_version: u8,
        playstore_enabled: bool,
        setup_script: Option<PathBuf>,
        run_script: Option<PathBuf>,
    },
    ExistingAvd {
        path: PathBuf,
        run_script: Option<PathBuf>,
    }
}

impl From<AVDGuestOptions> for StateAVDGuestOptions {
    fn from(options: AVDGuestOptions) -> Self {
        match options {
            AVDGuestOptions::Avd {
                android_api_version,
                playstore_enabled,
                setup_script,
                run_script,
            } => StateAVDGuestOptions::Avd {
                android_api_version,
                playstore_enabled,
                setup_script,
                run_script,
            },
            AVDGuestOptions::ExistingAvd {
                path,
                run_script,
            } => StateAVDGuestOptions::ExistingAvd {
                path,
                run_script,
            }
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Validate, Clone)]
pub struct StateAndroidScaling {
    #[validate(range(min = 1))]
    pub count: u32,
    #[validate(length(min = 1))]
    pub interfaces: HashMap<String, ConfigScalingInterface>,
}

impl From<AndroidScaling> for StateAndroidScaling {
    fn from(scaling: AndroidScaling) -> Self {
        Self {
            count: scaling.count,
            interfaces: scaling.interfaces
                .into_iter()
                .map(|(k, v)| (k, v.into()))
                .collect(),
        }
    }
}
