pub mod load_balancing;
pub mod orchestration_tasks;
pub mod delta;

use crate::components::LogicalTestbed;
use chrono;
use kvm_compose_schemas::kvm_compose_yaml::machines::GuestType;
use kvm_compose_schemas::kvm_compose_yaml::Machine;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::{fmt, fs};
use std::fmt::Formatter;
use tokio::fs::File;
use std::os::linux::fs::MetadataExt;
use std::path::PathBuf;
use anyhow::Context;
use nix::unistd::{Gid, Uid};
use tokio::io::AsyncWriteExt;

use crate::components::network::LogicalNetwork;
use crate::ovn::ovn::OvnNetwork;

// the data structures in this file represent the state, they are generated from the Config and Common
// data structures used to parse the kvm-compose.yaml
// some of the conversion is redundant but needed to keep the concerns of the data structures separate in the
// initial refactor as we unpick the coupling of the original code to only deploy on the current host
// in addition to being a command runner against libvirt and ovs into a distributed system and only
// generate configs ready for another process to run commands based on the various configs generated

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "snake_case")]
pub struct State {
    pub project_name: String,
    pub creation_date: String,
    pub project_working_dir: PathBuf,
    // creation_version: String,
    pub testbed_hosts: StateTestbedHostList,
    pub testbed_guests: StateTestbedGuestList,
    pub testbed_host_shared_config: StateTestbedHostSharedConfig,
    pub testbed_guest_shared_config: StateTestbedGuestSharedConfig,
    pub network: StateNetwork,
    pub state_provisioning: StateProvisioning,
}

impl State {
    pub fn new(logical_testbed: &LogicalTestbed) -> anyhow::Result<Self> {
        let testbed_hosts = Self::fill_host_list(logical_testbed)?;
        // let network = Self::fill_network(&testbed_hosts, logical_testbed)?;
        Ok(Self {
            project_name: logical_testbed.common.project.clone(),
            creation_date: format!("{:?}", chrono::offset::Local::now()),
            project_working_dir: logical_testbed.common.project_working_dir.clone(),
            testbed_hosts: StateTestbedHostList(testbed_hosts),
            testbed_guests: StateTestbedGuestList(Self::fill_guest_list(logical_testbed)?),
            testbed_host_shared_config: StateTestbedHostSharedConfig {
                // // this is the designated SDN bridge that is connected to the libvirt bridge
                // external_bridge: logical_testbed
                //     .common
                //     .config
                //     .network
                //     .external_bridge
                //     .as_ref()
                //     .unwrap()
                //     .clone(),
            },
            testbed_guest_shared_config: StateTestbedGuestSharedConfig {
                ssh_public_key_location: logical_testbed
                    .common
                    .kvm_compose_config
                    .ssh_public_key_location
                    .clone(),
                ssh_private_key_location: logical_testbed
                    .common
                    .kvm_compose_config
                    .ssh_private_key_location
                    .clone(),
                // password_ssh_enabled: common.config.password_ssh_enabled,
            },
            network: Self::fill_network(logical_testbed)?,
            state_provisioning: StateProvisioning {
                guests_provisioned: false
            },
        })
    }

    fn fill_host_list(
        logical_testbed: &LogicalTestbed,
    ) -> anyhow::Result<BTreeMap<String, StateTestbedHost>> {
        let mut testbed_host_map = BTreeMap::new();
        // testbed host allocation and master allocation in ~/.kvm-compose/kvm-compose-config.json
        for (host, config) in logical_testbed
            .common
            .kvm_compose_config
            .testbed_host_ssh_config
            .iter()
        {
            let is_master_host = if config.is_master_host.is_some() {
                // if is_master_host is given then read given otherwise set to false
                config.is_master_host.context("getting is master host bool")?
            } else {
                false
            };
            testbed_host_map.insert(
                host.to_string(),
                StateTestbedHost {
                    // hostname: host.clone(),
                    username: config.user.clone(),
                    ssh_private_key_location: config.identity_file.clone(),
                    ip: config.ip.clone(),
                    testbed_nic: config.testbed_nic.clone(),
                    is_master_host,
                },
            );
        }
        // TODO - sanity check if there was no declared testbed master
        Ok(testbed_host_map)
    }

    fn fill_guest_list(
        logical_testbed: &LogicalTestbed,
    ) -> anyhow::Result<BTreeMap<String, StateTestbedGuest>> {
        let mut testbed_guest_map: BTreeMap<String, StateTestbedGuest> = BTreeMap::new();
        for guest in logical_testbed.logical_guests.iter() {
            let guest_type = guest.get_machine_definition();
            // if a libvirt guest, check if it is a golden image (backing image for linked clones)
            let is_golden_image = match &guest_type.guest_type {
                GuestType::Libvirt(libvirt_guest) => {
                    libvirt_guest.scaling.is_some()
                }
                GuestType::Docker(_) => false,
                GuestType::Android(_) => false,
            };

            testbed_guest_map.insert(
                guest.get_guest_name().clone(),
                StateTestbedGuest {
                    guest_type,
                    testbed_host: guest.get_testbed_host().clone(), // filled in during load balancing
                    is_golden_image,
                    guest_id: guest.get_guest_id(),
                    extra_info: StateTestbedGuestExtraInfo {
                        reference_image: guest.get_reference_image()?,
                    },
                },
            );
        }

        Ok(testbed_guest_map)
    }

    fn fill_network(logical_testbed: &LogicalTestbed) -> anyhow::Result<StateNetwork> {
        match &logical_testbed.network.as_ref().unwrap() {
            LogicalNetwork::Ovn(ovn_network) => {
                // OVN network doesnt need much extra config here so we just pass through what exists
                // in the logical testbed as it is the same schema used in the state
                Ok(StateNetwork::Ovn(ovn_network.clone()))
            }
            LogicalNetwork::Ovs(_) => {
                // since OVS requires some extra information, we need to do further work here
                unimplemented!()
            }
        }
    }

    pub async fn write(&self, project_name: &String, project_path: &PathBuf) -> anyhow::Result<()> {
        let path_str = &project_path.clone()
            .to_string_lossy()
            .to_string();
        let file_name = format!("{}/{}-state.json", &path_str, &project_name);
        let mut output = File::create(&file_name).await?;
        output.write_all(format!("{self}").as_bytes()).await?;
        // set owner:group to the same as parent folder
        let metadata = fs::metadata(path_str)?;
        let uid = metadata.st_uid();
        let gid = metadata.st_gid();
        nix::unistd::chown(&PathBuf::from(file_name), Some(Uid::from(uid)), Some(Gid::from(gid)))?;
        Ok(())
    }
}

impl fmt::Display for State {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(&serde_json::to_string_pretty(&self)
            .expect("parsing state string"))
            .expect("state to json via serde failed");
        Ok(())
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub struct StateTestbedHost {
    // pub hostname: String, // TODO hostname here is redundant when using hashmap of StateTestbedHost
    // TODO - add the ssh address i.e. user@projectname-hostname -> ubuntu@testbed-host2
    //  guests also cannot clash with this name
    pub username: String,
    pub ssh_private_key_location: String,
    pub ip: String,
    // resource_metadata: ,
    pub testbed_nic: String,
    pub is_master_host: bool,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub struct StateTestbedHostList(pub BTreeMap<String, StateTestbedHost>);

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub struct StateTestbedGuest {
    /// `guest_type` is a straight copy of the yaml file definition, wrapped by this "State" struct
    #[serde(flatten)]
    pub guest_type: Machine,
    pub testbed_host: Option<String>,
    pub is_golden_image: bool,
    /// this is a unique identifier for the guest in the state
    pub guest_id: u32,
    pub extra_info: StateTestbedGuestExtraInfo,
}

/// This contains extra information on the guest that is not captured by the yaml, but is computed from a combination of
/// the yaml and the testbed environment, making it unique to a testbed
#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub struct StateTestbedGuestExtraInfo {
    /// This is used when a guest is based off another resource i.e. an image for existing disk, or an iso for iso guest
    pub reference_image: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub struct StateTestbedGuestList(pub BTreeMap<String, StateTestbedGuest>);

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub struct StateTestbedHostSharedConfig {
    // external_bridge: String,
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
#[serde(rename_all = "snake_case")]
pub struct StateTestbedGuestSharedConfig {
    pub ssh_public_key_location: String,
    pub ssh_private_key_location: String,
    // password_ssh_enabled: bool,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum StateNetwork {
    Ovn(OvnNetwork),
    Ovs(StateOvsNetwork),
}

impl Default for StateNetwork {
    fn default() -> Self {
        Self::Ovn(OvnNetwork::default())
    }
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "snake_case")]
#[derive(Clone)]
pub struct StateOvsNetwork {
    // libvirt_network_name: String,
    // livbirt_network_bridge_name: String,
    // libvirt_network_xml: String,
    // veth_peer_left: String,
    // veth_peer_right: String,
    // pub libvirt_network_subnet: String,
    // pub bridges: Vec<StateTestbedInterface>,
    // logical_bridge_connections: LogicalBridgeConnections,
    // pub physical_bridge_connections: PhysicalBridgeConnections,
    // // TODO place the list of network interfaces from config, after being processed into state
    // // network_interfaces: Vec<NetworkInterface>,
}

/// Struct to keep tract of provisioning changes and actions. Used to make sure deployments are not
/// overwritten.
#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "snake_case")]
pub struct StateProvisioning {
    pub guests_provisioned: bool,
}