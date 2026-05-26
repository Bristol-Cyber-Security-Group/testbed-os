use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use anyhow::Context;
use serde::{Deserialize, Serialize};
use kvm_compose_schemas::kvm_compose_yaml::machines::libvirt::{ConfigLibvirtMachine, ConfigScaling, ConfigScalingRun, ConfigScalingSetup, DiskDeviceType, DiskDriverType, LibvirtGuestOptions};
use kvm_compose_schemas::kvm_compose_yaml::machines::libvirt_image_download::OnlineCloudImage;
use validator::Validate;
use crate::state::schema::machines::{StateScalingInterface};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct StateLibvirtMachine {
    pub memory_mb: u32,
    pub cpus: u32,
    pub libvirt_type: StateLibvirtGuestOptions,
    pub username: Option<String>,
    pub password: Option<String>,
    pub hostname: String,
    pub ssh_address: String,
    pub scaling: Option<StateScaling>,
    pub is_clone_of: Option<String>,
    pub tcp_tty_port: Option<u32>,
    pub static_ip: Option<String>,
}

impl TryFrom<ConfigLibvirtMachine> for StateLibvirtMachine {

    type Error = anyhow::Error;

    fn try_from(libvirt: ConfigLibvirtMachine) -> anyhow::Result<Self> {
        Ok(Self {
            memory_mb: libvirt.memory_mb.context("expected memory to be Some")?,
            cpus: libvirt.cpus.context("expected cpus to be Some")?,
            libvirt_type: libvirt.libvirt_type.into(),
            username: libvirt.username.clone(),
            password: libvirt.password.clone(),
            hostname: "".to_string(), // TODO - where does this get filled in and does it work
            ssh_address: "".to_string(), // TODO - where does this get filled in and does it work
            scaling: libvirt.scaling.map(|s| s.into()),
            is_clone_of: libvirt.is_clone_of.clone(),
            tcp_tty_port: None, // TODO - where does this get filled in and does it work
            static_ip: libvirt.static_ip.clone(),
        })
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum StateLibvirtGuestOptions {
    CloudImage {
        name: OnlineCloudImage, // import from yaml schema rather than create a state specific version
        expand_gigabytes: Option<u16>,
        path: Option<PathBuf>,
        run_script: Option<PathBuf>,
        setup_script: Option<PathBuf>,
        setup_script_timeout_s: u16,
        context: Option<PathBuf>,
        #[serde(default)]
        environment: BTreeMap<String, String>,
    },
    ExistingDisk {
        path: PathBuf,
        #[serde(default)]
        driver_type: StateDiskDriverType,
        #[serde(default)]
        device_type: StateDiskDeviceType,
        #[serde(default)]
        readonly: bool,
        /// Specify whether a full copy of the disk is made rather than a linked clone of the
        /// original image. A linked clone will still preserve the original but will be more
        /// conservative with disk space on the host.
        #[serde(default)]
        create_deep_copy: bool,
    },
    IsoGuest {
        path: PathBuf,
        expand_gigabytes: Option<u16>,
        #[serde(default)]
        driver_type: StateDiskDriverType,
        #[serde(default)]
        device_type: StateDiskDeviceType,
        #[serde(default)]
        readonly: bool,
    },
}

impl From<LibvirtGuestOptions> for StateLibvirtGuestOptions {
    fn from(options: LibvirtGuestOptions) -> Self {
        match options {
            LibvirtGuestOptions::CloudImage {
                name,
                expand_gigabytes,
                path,
                run_script,
                setup_script,
                setup_script_timeout_s,
                context,
                environment,
            } => StateLibvirtGuestOptions::CloudImage {
                name,
                expand_gigabytes,
                path,
                run_script,
                setup_script,
                setup_script_timeout_s,
                context,
                environment,
            },
            LibvirtGuestOptions::ExistingDisk {
                path,
                driver_type,
                device_type,
                readonly,
                create_deep_copy ,
            } => StateLibvirtGuestOptions::ExistingDisk {
                path,
                driver_type: driver_type.into(),
                device_type: device_type.into(),
                readonly,
                create_deep_copy,
            },
            LibvirtGuestOptions::IsoGuest {
                path,
                expand_gigabytes,
                driver_type,
                device_type,
                readonly,
            } => StateLibvirtGuestOptions::IsoGuest {
                path,
                expand_gigabytes,
                driver_type: driver_type.into(),
                device_type: device_type.into(),
                readonly,
            },
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum StateDiskDriverType {
    Raw,
    QCow2,
}

impl Default for StateDiskDriverType {
    fn default() -> Self {
        Self::Raw
    }
}

impl From<DiskDriverType> for StateDiskDriverType {
    fn from(value: DiskDriverType) -> Self {
        match value {
            DiskDriverType::Raw => StateDiskDriverType::Raw,
            DiskDriverType::QCow2 => StateDiskDriverType::QCow2,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum StateDiskDeviceType {
    Disk,
    CdRom,
}

impl Default for StateDiskDeviceType {
    fn default() -> Self {
        Self::Disk
    }
}

impl From<DiskDeviceType> for StateDiskDeviceType {
    fn from(value: DiskDeviceType) -> Self {
        match value {
            DiskDeviceType::Disk => StateDiskDeviceType::Disk,
            DiskDeviceType::CdRom => StateDiskDeviceType::CdRom,
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Validate, Clone)]
pub struct StateScaling {
    #[validate(range(min = 1))]
    pub count: u32,
    pub shared_setup: Option<PathBuf>,
    #[validate(length(min = 1))]
    pub interfaces: HashMap<String, StateScalingInterface>,
    pub clone_setup: Option<Vec<StateScalingSetup>>,
    pub clone_run: Option<Vec<StateScalingRun>>,
}

impl From<ConfigScaling> for StateScaling {
    fn from(scaling: ConfigScaling) -> Self {
        Self {
            count: scaling.count,
            shared_setup: scaling.shared_setup,
            interfaces: scaling.interfaces
                .into_iter()
                .map(|(k, v)| (k, v.into()))
                .collect(),
            clone_setup: scaling.clone_setup.map(|vec| {
                vec.iter().map(Into::into).collect()
            }),
            clone_run: scaling.clone_run.map(|vec| {
                vec.iter().map(Into::into).collect()
            }),
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct StateScalingSetup {
    pub script: PathBuf,
    pub clones: Vec<u32>,
}

impl From<&ConfigScalingSetup> for StateScalingSetup {
    fn from(scaling: &ConfigScalingSetup) -> Self {
        Self {
            script: scaling.script.clone(),
            clones: scaling.clones.clone(),
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct StateScalingRun {
    pub script: PathBuf,
    pub clones: Vec<u32>,
}

impl From<&ConfigScalingRun> for StateScalingRun {
    fn from(run: &ConfigScalingRun) -> Self {
        Self {
            script: run.script.clone(),
            clones: run.clones.clone(),
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct StateScalingIp {
    pub clone: u32,
    pub ip: String,
}