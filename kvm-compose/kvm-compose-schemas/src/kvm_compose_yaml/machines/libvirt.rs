use crate::kvm_compose_yaml::machines::libvirt_image_download::OnlineCloudImage;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use validator::Validate;
use crate::kvm_compose_yaml::machines::ConfigScalingInterface;

/// Contains the shared config for all libvirt guest types
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ConfigLibvirtMachine {
    pub memory_mb: Option<u32>,
    pub cpus: Option<u32>,
    pub libvirt_type: LibvirtGuestOptions,
    pub username: Option<String>,
    pub password: Option<String>,
    #[serde(skip_deserializing)]
    pub hostname: String,
    #[serde(skip_deserializing)]
    pub ssh_address: String,
    // #[serde(default)]
    // pub extended_graphics_support: bool,
    pub scaling: Option<ConfigScaling>,
    // #[serde(skip_deserializing)]
    pub is_clone_of: Option<String>,
    #[serde(skip_deserializing)]
    pub tcp_tty_port: Option<u32>,
    pub static_ip: Option<String>,
}

/// This is a further specialisation for libvirt guests, any options that are specific to the guest
/// type are placed here that are not applicable to the other guest types
#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum LibvirtGuestOptions {
    CloudImage {
        name: OnlineCloudImage,
        expand_gigabytes: Option<u16>,
        // #[serde(skip_deserializing)]
        path: Option<PathBuf>,
        run_script: Option<PathBuf>,
        setup_script: Option<PathBuf>,
        // TODO validate
        // #[validate(custom = "validate_context")]
        context: Option<PathBuf>,
        #[serde(default)]
        environment: BTreeMap<String, String>,
    },
    ExistingDisk {
        path: PathBuf,
        #[serde(default)]
        driver_type: DiskDriverType,
        #[serde(default)]
        device_type: DiskDeviceType,
        #[serde(default)]
        readonly: bool,
    },
    IsoGuest {
        path: PathBuf,
        expand_gigabytes: Option<u16>,
        #[serde(default)]
        driver_type: DiskDriverType,
        #[serde(default)]
        device_type: DiskDeviceType,
        #[serde(default)]
        readonly: bool,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum DiskDriverType {
    Raw,
    QCow2,
}

impl Default for DiskDriverType {
    fn default() -> Self {
        Self::Raw
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum DiskDeviceType {
    Disk,
    CdRom,
}

impl Default for DiskDeviceType {
    fn default() -> Self {
        Self::Disk
    }
}

#[derive(Deserialize, Serialize, Debug, Validate, Clone)]
pub struct ConfigScaling {
    #[validate(range(min = 1))]
    pub count: u32,
    pub shared_setup: Option<PathBuf>,
    #[validate(length(min = 1))]
    pub interfaces: HashMap<String, ConfigScalingInterface>,
    pub clone_setup: Option<Vec<ConfigScalingSetup>>,
    pub clone_run: Option<Vec<ConfigScalingRun>>,
    // pub clone_static_ip: Option<Vec<ConfigScalingIp>>
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ConfigScalingSetup {
    pub script: PathBuf,
    pub clones: Vec<u32>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ConfigScalingRun {
    pub script: PathBuf,
    pub clones: Vec<u32>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ConfigScalingIp {
    pub clone: u32,
    pub ip: String,
}