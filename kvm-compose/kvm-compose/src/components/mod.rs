pub mod guest;
pub mod helpers;
pub mod logical_load_balancing;
pub mod network;
pub mod network_interfaces;

use crate::components::guest::get_guest_from_config;
use crate::components::logical_load_balancing::load_balance;
use kvm_compose_schemas::cli_models::Common;
use anyhow::{bail, Context};
use kvm_compose_schemas::kvm_compose_yaml::network_old::NetworkInterface;
use kvm_compose_schemas::kvm_compose_yaml::{Config, Machine, MachineNetwork};
use std::any::Any;
use std::collections::BTreeMap;
use std::path::PathBuf;
use async_trait::async_trait;
use tokio::sync::mpsc::{Sender};
use kvm_compose_schemas::kvm_compose_yaml::network::NetworkBackend;
use kvm_compose_schemas::settings::TestbedClusterConfig;
use crate::components::network::{LogicalNetwork};
use crate::orchestration::api::{OrchestrationInstruction, OrchestrationProtocol};
use crate::orchestration::websocket::{send_orchestration_instruction_over_channel};
use crate::ovn::components::MacAddress;

// The purpose of the Logical testbed struct that holds all of the components is to provide
// a generic interface for artefact generation and creating state as there are various types
// and implementations for each component. The idea is to abstract the detail from the yaml
// definition, which is specific i.e. a machine is cloud init, to provide a generic interface
// that can be used by load balancing. Once load balancing occurs, then we can start to customise
// these logical components so that they can be converted into their state representation for
// the state.json.
// This means in this file, there is some composition using traits to give that flexibility so
// that in the respective component files in this folder i.e. guest or network, will be the
// implementation and is where you will need to edit or add new components.
// The contents of this file should remain relatively untouched once this abstraction is complete.

// Note: This abstraction for now will just be a wrapper around the "Config" struct and related
// structs it contains.

/// This struct contains information about the specific test case that testbed components may need
/// to use when they are finally specialised. Information such as the location of the project will
/// exist here such that guests can work out what their disk image paths should be.
pub struct SpecialisationContext {
    // we cant pass the common struct here due to lifetime issues and we shouldn't clone it since
    // it contains a libvirt ssh connection. So we need to specify exactly what we need duplicated
    // from the common struct.
    // TODO - we dont have the libvirt connection anymore, can we consolidate this and just pass around common?
    pub project_name: String,
    pub ssh_config: TestbedClusterConfig,
    pub project_folder_path: PathBuf,
    pub master_host: String,
    // send the deserialised yaml config
    pub config: Config,
    pub guest_ip_mapping: Option<BTreeMap<String, Option<String>>>,
    pub guest_mac_mapping: Option<BTreeMap<String, MacAddress>>,
}

/// This trait is used to describe how a testbed component will have it's artefacts created,
/// so that the artefact generation can run the generate and destroy methods on all components
/// once it has worked out all the underlying infrastructure and load balancing.
/// This is a trait as the underlying implementation for generating the artefact will be different
/// for the different machine types i.e. libvirt, docker, AVD etc and the different network
/// components i.e. OVS bridge, tunnel, libvirt network etc.
/// This also allows for the ease of addition for new future components.
#[async_trait]
pub trait TestbedComponent {

    /// Set the testbed host the component is assigned to, in addition to any other adjustments
    /// needed for the component to work there such as image disk locations if its a guest
    fn set_testbed_host(&mut self, in_host: String);

    fn get_testbed_host(&self) -> &Option<String>;

    fn get_static_ip(&self) -> Option<String>;

    /// Add implementation for the testbed component if the component needs to specialise their
    /// configuration once logical load balancing has occurred. This assumed that all the
    /// information this component needs has been set into it's struct. For example, a guest
    /// once it has been allocated a testbed host, it can work out the specific path for it's
    /// disk images etc.
    fn specialise(&mut self, context: &SpecialisationContext) -> anyhow::Result<()>;

    /// Implement Any for all testbed components so that we can downcast at a later stage to access
    /// any of the fields in the structs.
    fn as_any(&self) -> &dyn Any;
}

// Traits for testbed components

/// This trait is used to describe how a testbed guest will be converted from it's yaml
/// representation into a logical testbed component.
pub trait TestbedGuest {
    /// Convert from the yaml representation in ConfigMachine
    fn from_config_machine(in_config_machine: &Machine, unique_id: u32) -> Self
    where
        Self: Sized;

    fn get_guest_name(&self) -> &String;

    /// Return the guest's interface on the network
    fn get_network(&self) -> anyhow::Result<Vec<MachineNetwork>>;

    /// Get the guest machine definition as specified in the yaml file but could be more up to date
    /// as the logical testbed setup could have further specialised or edited the definition.
    fn get_machine_definition(&self) -> Machine;

    /// Get a mutable reference to the guest machine definition as specified in the yaml file but
    /// could be more up to date as the logical testbed setup could have further specialised or
    /// edited the definition.
    fn get_machine_definition_mut(&mut self) -> &mut Machine;

    /// get unique guest ID
    fn get_guest_id(&self) -> u32;

    /// If the guest is using an image or an iso as a reference for its own image, this should return Some with the
    /// path to this reference image
    fn get_reference_image(&self) -> anyhow::Result<Option<String>>;
}

pub trait TestbedNetworkInterface {
    fn from_config(in_nic: &NetworkInterface, in_id: u16) -> Self
        where
            Self: Sized;

    fn get_interface(&self) -> &String;

    fn get_id(&self) -> u16;
}

// Trait combinations

/// Trait to combine ``TestbedGuest`` and ``TestbedComponent`` to allow calling both traits
/// methods in the ``LogicalTestbed`` logic.
pub trait TestbedGuestComponent: TestbedGuest + TestbedComponent {}
impl<T: TestbedGuest + TestbedComponent > TestbedGuestComponent for T {}

/// Trait to combine ``TestbedNetworkInterface`` and ``TestbedComponent`` to allow calling both traits
/// methods in the ``LogicalTestbed`` logic.
pub trait TestbedNetworkInterfaceComponent: TestbedNetworkInterface + TestbedComponent {}
impl<T: TestbedNetworkInterface + TestbedComponent > TestbedNetworkInterfaceComponent for T {}

// make a type def to make this shorter in the signatures
type LogicalGuests = Vec<Box<dyn TestbedGuestComponent + Sync + Send>>;

// Logical testbed implementation

/// This struct contains the logical form of all the testbed components, generic to their
/// underlying implementation by specifying the trait they implement.
pub struct LogicalTestbed {
    // these are the main components of the testbed
    pub logical_guests: LogicalGuests,
    // // this stores the external network interfaces
    // pub logical_network_interfaces: Option<Vec<Box<dyn TestbedNetworkInterfaceComponent + Sync + Send>>>,

    pub network: Option<LogicalNetwork>,

    // this stores the guest to ip mapping, if static
    pub guest_ip_mapping: Option<BTreeMap<String, Option<String>>>,
    // this stored the guest to mac mapping, necessary for OVN
    pub guest_mac_mapping: Option<BTreeMap<String, MacAddress>>,

    // store a copy of Common
    pub common: Common,
}

impl LogicalTestbed {
    /// Constructor for LogicalTestbed
    pub fn new(in_common: Common) -> Self {
        Self {
            logical_guests: vec![],
            network: None,
            guest_ip_mapping: None,
            guest_mac_mapping: None,
            common: in_common,
        }
    }

    /// Process the parsed kvm-compose.yaml file `config` to work out the specific provisioning
    /// requirements for each guest, and any necessary networking provisioning that is dependant
    /// on the testbed itself i.e. the testbed host configuration and how many testbed hosts.
    /// The resulting `LogicalTestbed` will be used to create a `State` which is used in the
    /// orchestration process.
    pub fn process_config(&mut self) -> anyhow::Result<()> {
        // add all guests in yaml schema into logical guests list, this already includes clones
        self.parse_guests()?;

        // load balance guests between hosts based on LB algorithm, output a mapping
        // this will also assign the testbed host to the logical guest config
        tracing::info!("load balancing guests");
        let load_balance_topology = load_balance(self)?;

        // testbed network configuration based on the backend chosen
        match &self.common.config.network {
            NetworkBackend::Ovn(ovn_network) => {
                // create OVN components from network
                self.network = Some(LogicalNetwork::new_ovn(
                    ovn_network,
                    &load_balance_topology,
                    &self.common.kvm_compose_config.testbed_host_ssh_config,
                    &self.logical_guests,
                    &self.common.project
                )?);
            }
            NetworkBackend::Ovs(ovs_network) => {
                // TODO - create a mapping of OVS bridge to host
                //  directly create the state representation here
                self.network = Some(LogicalNetwork::new_ovs(ovs_network));

                // TODO - work out the connections between OVS bridges to create either local on the
                //  same host or tunnel connections between hosts

                // TODO - assign guest ips where necessary

                unimplemented!()
            }
        }

        // // assign guests the mac address based on the port/interface
        // assign_guests_mac(
        //     &mut self.logical_guests,
        //     &self.network.as_ref().unwrap(),
        //     &self.common.project,
        // )?;

        // // assign guest ips, this will depend on the port definitions for the guest
        // //  if android or docker, need to check they're not dynamic
        // tracing::info!("validating guest ip addresses");
        // assign_and_validate_guest_ip(
        //     &mut self.logical_guests,
        //     self.network.as_ref().context("getting network from config")?,
        //     &self.common.project,
        // )?;

        // TODO - validate logical testbed

        // TODO - specialise the guests based on the underlying testbed environment to work out
        //  filesystem paths, generate xmls etc - based on the network backend
        // TODO - context needs to be generic to network backend, can we send in the network config
        //        into the guest
        let context = self.create_specialisation_context()?;
        for guest in self.logical_guests.iter_mut() {
            guest.specialise(&context)?;
        }

        Ok(())
    }

    fn parse_guests(&mut self) -> anyhow::Result<()> {
        if self.common.config.machines.is_some() {
            for (guest_id_counter, guest) in self
                .common
                .config
                .machines
                .clone()
                .context("Getting machines from yaml config")?
                .iter()
                .enumerate()
            {
                let logical_guest = get_guest_from_config(guest, guest_id_counter as u32)
                    .context("Getting guest definition from config as a testbed component")?;
                self.logical_guests.push(logical_guest);
            }
        }
        Ok(())
    }

    /// Get the master testbed host from the configuration
    pub fn get_master_testbed_host(&self) -> anyhow::Result<&String> {
        for (name, config) in self.common.kvm_compose_config.testbed_host_ssh_config.iter() {
            if let Some(is_master) = config.is_master_host {
                if is_master {
                    return Ok(name);
                }
            }
        }
        bail!("there was no master host set in kvm-compose-config.json")
    }

    /// Request to the server to generate artefacts
    pub async fn request_generate_artefacts(
        &self,
        sender: &mut Sender<OrchestrationProtocol>,
        // receiver: &mut Receiver<OrchestrationProtocol>,
    ) -> anyhow::Result<()> {
        send_orchestration_instruction_over_channel(
            sender,
            // receiver,
            OrchestrationInstruction::GenerateArtefacts {
                project_path: self.common.project_working_dir.to_str()
                    .context("getting project path")?.to_string(),
                uid: self.common.fs_user.as_raw(),
                gid: self.common.fs_group.as_raw(),
            },
        ).await.context("sending Generate Artefacts request to server")?;
        Ok(())
    }

    pub fn create_specialisation_context(&self) -> anyhow::Result<SpecialisationContext> {
        Ok(SpecialisationContext {
            project_name: self.common.project.clone(),
            project_folder_path: self.common.project_working_dir.clone(),
            ssh_config: self.common.kvm_compose_config.clone(),
            master_host: self
                .get_master_testbed_host()
                .context("Getting master testbed host")?
                .clone(),
            config: self.common.config.clone(),
            guest_ip_mapping: self.guest_ip_mapping.clone(),
            guest_mac_mapping: self.guest_mac_mapping.clone(),
        })
    }
}

/// This function will generate the interface name based on the project name and the unique guest
/// id due to linux interface name limitations, the interface name must be max 15 characters. We
/// will truncate the project name in case it is a long project name. We will give the project name
/// 7 characters, the interface id 1 character, and the id 4 characters and start with 'vm-',
/// meaning we can support 9999 guests which should be enough for the foreseeable future..
pub fn get_guest_interface_name(project_name: &str, unique_id: u32, idx: usize) -> String {
    let truncated_project_name = if project_name.len() > 7 {
        let mut temp = project_name.to_owned();
        temp.truncate(7);
        temp
    } else {
        project_name.to_owned()
    };
    format!("vm-{truncated_project_name}{unique_id}{idx}")
}
