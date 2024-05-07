use crate::components::{SpecialisationContext, TestbedComponent, TestbedGuest, TestbedGuestComponent};
use anyhow::{bail, Context};
use kvm_compose_schemas::kvm_compose_yaml::machines::libvirt::LibvirtGuestOptions;
use kvm_compose_schemas::kvm_compose_yaml::machines::GuestType;
use kvm_compose_schemas::kvm_compose_yaml::{Machine, MachineNetwork};
use std::any::Any;
use std::path::{PathBuf};
use async_trait::async_trait;
use rand::distributions::Alphanumeric;
use rand::Rng;
use crate::components;
use crate::ovn::components::MacAddress;

// This file describes all the different possible guests the kvm-compose yaml file supports.
// To add a new guest type, create a struct that implements both TestbedGuest and TestbedComponent.

/// The various guest types will correlate to `MachineType` in the Config code.
/// This function will return the correct type but as a trait so that it can be added to
/// the ``LogicalTestbed`` struct.
pub fn get_guest_from_config(
    in_config_machine: &Machine,
    unique_id: u32
) -> anyhow::Result<Box<dyn TestbedGuestComponent + Sync + Send>> {
    return match &in_config_machine.guest_type {
        GuestType::Libvirt(libvirt_guest) => {
            // differentiate from the different libvirt guest types
            // TODO are these redundant now since the guest type is differentiated in "specialise"?
            match libvirt_guest.libvirt_type {
                LibvirtGuestOptions::CloudImage { .. } => {
                    Ok(Box::new(LibvirtGuest::from_config_machine(in_config_machine, unique_id)))
                }
                LibvirtGuestOptions::ExistingDisk { .. } => {
                    Ok(Box::new(LibvirtGuest::from_config_machine(in_config_machine, unique_id)))
                }
                LibvirtGuestOptions::IsoGuest { .. } => {
                    Ok(Box::new(LibvirtGuest::from_config_machine(in_config_machine, unique_id)))
                }
            }
        }
        GuestType::Docker(_) => {
            Ok(Box::new(DockerGuest::from_config_machine(in_config_machine, unique_id)))
        }
        GuestType::Android(_) => {
            Ok(Box::new(AndroidGuest::from_config_machine(in_config_machine, unique_id)))
        }
    }
}

/// Helper to get the guest network, is a generic as there are more than one type of guest that
/// implements trait `TestbedGuest`. This is needed since `MachineNetwork` is defined as optional
/// due to definitions that have scaling. Scaling guests do not have a network section as they
/// represent the base image and are provisioned differently, only the clones of these guests have
/// networks.
fn get_guest_network<G: TestbedGuest>(guest: &G) -> anyhow::Result<MachineNetwork> {
    if let Some(net) = guest.get_machine_definition().network {
        Ok(net)
    } else {
        bail!("could not get guest network definition")
    }
}

/// This struct represents all testbed guests that will utilise libvirt and cloud-init for
/// virtualisation
pub struct LibvirtGuest {
    pub config_machine: Machine,
    pub testbed_host: Option<String>,
    pub original_disk_path: Option<PathBuf>,
    // is either master filesystem or remote filesystem path
    pub disk_path: Option<String>,
    pub iso_path: Option<String>,
    // master filesystem, path to disk
    pub master_disk_path: Option<String>,
    // master filesystem, path to guest folder
    pub path_for_command: Option<String>,
    // is either master filesystem or remote filesystem path
    pub guest_folder: Option<String>,
    pub path_to_domain_xml: Option<String>,
    pub mac_address: Option<MacAddress>,
    pub ip_address: Option<String>,
    pub unique_id: u32,
    pub reference_image: Option<String>,
}

impl TestbedGuest for LibvirtGuest {
    fn from_config_machine(in_config_machine: &Machine, unique_id: u32) -> Self {
        Self {
            config_machine: in_config_machine.clone(),
            testbed_host: None,
            original_disk_path: None,
            disk_path: None,
            iso_path: None,
            master_disk_path: None,
            path_for_command: None,
            guest_folder: None,
            path_to_domain_xml: None,
            mac_address: None,
            ip_address: None,
            unique_id,
            reference_image: None,
        }
    }

    fn get_guest_name(&self) -> &String {
        &self.config_machine.name
    }

    fn get_network(&self) -> anyhow::Result<String> {
        Ok(get_guest_network(self)?.switch)
    }

    fn get_machine_definition(&self) -> Machine {
        self.config_machine.clone()
    }

    fn get_machine_definition_mut(&mut self) -> &mut Machine {
        &mut self.config_machine
    }

    // fn set_mac_address(&mut self, mac_address: MacAddress) {
    //     self.mac_address = Some(mac_address);
    // }

    fn get_mac_address(&self) -> anyhow::Result<MacAddress> {
        MacAddress::new(
            self.config_machine.network.as_ref()
                .context("getting mac address for guest")?.mac.clone()
        )
    }

    fn get_gateway(&self) -> Option<String> {
        if let Some(net) = &self.config_machine.network {
            net.gateway.clone()
        } else {
            None
        }
    }

    fn get_guest_id(&self) -> u32 {
        self.unique_id
    }

    fn get_interface_name(&self, project_name: &String) -> String {
        components::get_guest_interface_name(project_name, self.unique_id)
    }

    fn get_reference_image(&self) -> anyhow::Result<Option<String>> {
        match &self.config_machine.guest_type {
            GuestType::Libvirt(l) => {
                match &l.libvirt_type {
                    LibvirtGuestOptions::CloudImage { .. } => Ok(None),
                    LibvirtGuestOptions::ExistingDisk { .. } => {
                        Ok(Some(self.original_disk_path
                            .clone()
                            .context("getting original disk path for libvirt guest")?
                            .to_str()
                            .context("converting original disk path to str for libvirt guest")?
                            .to_string()))
                    }
                    LibvirtGuestOptions::IsoGuest { .. } => {
                        Ok(Some(self.original_disk_path
                            .clone()
                            .context("getting original disk path for libvirt guest")?
                            .to_str()
                            .context("converting original disk path to str for libvirt guest")?
                            .to_string()))
                    }
                }
            }
            _ => unreachable!()
        }
    }
}

#[async_trait]
impl TestbedComponent for LibvirtGuest {

    fn set_testbed_host(&mut self, in_host: String) {
        self.testbed_host = Some(in_host)
    }

    fn get_testbed_host(&self) -> &Option<String> {
        &self.testbed_host
    }

    fn set_static_ip(&mut self, in_ip: String) {
        self.ip_address = Some(in_ip);
    }

    fn get_static_ip(&self) -> Option<String> {
        self.ip_address.clone()
    }

    fn specialise(&mut self, context: &SpecialisationContext) -> anyhow::Result<()> {
        // check if guest is on master or not
        let testbed_host_name = self.get_testbed_host().clone().unwrap();
        let project_path = context.project_folder_path.to_str().unwrap().to_string();
        let local_path = if context.master_host.eq(&testbed_host_name) {
            // on master
            format!("{project_path}/artefacts/")
        } else {
            let remote_host_username =
                &context.ssh_config.testbed_host_ssh_config[&testbed_host_name].user;
            let project_name = &context.project_name;
            format!("/home/{remote_host_username}/testbed-projects/{project_name}/artefacts/").to_owned()
        };

        self.path_for_command = Some(format!(
            "{project_path}/artefacts/"
        ));
        // let local_disk_path = format!("{local_path}/{guest_name}-cloud-disk.img");
        let guest_name = self.get_guest_name().clone();
        // use .img if normal or backing image, use .qcow2 for clone guests
        self.disk_path = match &self.config_machine.guest_type {
            GuestType::Libvirt(libvirt_guest) => {
                if libvirt_guest.is_clone_of.is_some() {
                    Some(format!("{local_path}/{guest_name}-linked-clone.qcow2"))
                } else {
                    Some(format!("{local_path}/{guest_name}-cloud-disk.img"))
                }
            }
            _ => {
                unreachable!()
            }
        };
        // set iso path if cloud image
        self.iso_path = match &self.config_machine.guest_type {
            GuestType::Libvirt(libvirt_guest) => {
                match &libvirt_guest.libvirt_type {
                    LibvirtGuestOptions::CloudImage {  .. } => {
                        if libvirt_guest.is_clone_of.is_some() {
                            let iso = format!(
                                "{}/{}-linked-clone.iso",
                                &local_path, &self.config_machine.name
                            );
                            Some(iso)
                        } else {
                            let iso = format!(
                                "{}/{}-cloud-init.iso",
                                &local_path, &self.config_machine.name
                            );
                            Some(iso)
                        }
                    }
                    _ => None
                }
            }
            _ => {
                unreachable!()
            }
        };

        // get the local disk path even if guest will be on remote
        self.master_disk_path = match &self.config_machine.guest_type {
            GuestType::Libvirt(libvirt_guest) => {
                let folder_for_script = self.path_for_command.clone().unwrap();
                if libvirt_guest.is_clone_of.is_some() {
                    Some(format!(
                        "{folder_for_script}{guest_name}-linked-clone.qcow2"
                    ))
                } else {
                    Some(format!("{folder_for_script}{guest_name}-cloud-disk.img"))
                }
            }
            _ => {
                // TODO iso guest and existing disk
                unreachable!()
            }
        };
        self.guest_folder = Some(local_path.clone());
        // customise the config given the specialisations just completed
        match &mut self.config_machine.guest_type {
            GuestType::Libvirt(libivrt_config) => {
                // set disk paths
                match &mut libivrt_config.libvirt_type {
                    LibvirtGuestOptions::CloudImage { path, .. } => {
                        self.original_disk_path = path.clone();
                        if path.is_none() {
                            *path = Some(PathBuf::from(self.disk_path.clone().unwrap()));
                        }
                    }
                    LibvirtGuestOptions::ExistingDisk { path, .. } => {
                        self.original_disk_path = Some(path.clone());
                        *path = PathBuf::from(self.disk_path.clone().unwrap());
                    }
                    LibvirtGuestOptions::IsoGuest { path, .. } => {
                        self.original_disk_path = Some(path.clone());
                        *path = PathBuf::from(self.disk_path.clone().unwrap());
                    }
                }
                // set up hostname, username and ssh address for orchestration
                if libivrt_config.username.is_none() {
                    libivrt_config.username = Some("nocloud".to_string());
                }

                libivrt_config.hostname = format!("{}-{}", context.project_name, guest_name);
                libivrt_config.ssh_address = format!(
                    "{}@{}",
                    libivrt_config
                        .username
                        .clone()
                        .context("getting guest username")?,
                    libivrt_config.hostname.clone()
                );
            }
            _ => {
                unreachable!()
            }
        }
        // save path to domain xml for create_command
        let xml_dest = format!(
            "{}/{}-domain.xml",
            &local_path, self.config_machine.name
        );
        self.path_to_domain_xml = Some(xml_dest);

        Ok(())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}


/// This struct represents all testbed guests that will utilise docker backend
pub struct DockerGuest {
    pub config_machine: Machine,
    pub testbed_host: Option<String>,
    pub container_name: Option<String>,
    // change dir for running docker command, depends on if master or remote testbed host
    pub change_dir: Option<String>,
    pub unique_id: u32,
}

impl TestbedGuest for DockerGuest {
    fn from_config_machine(in_config_machine: &Machine, unique_id: u32) -> Self where Self: Sized {
        Self {
            config_machine: in_config_machine.clone(),
            testbed_host: None,
            container_name: None,
            change_dir: None,
            unique_id,
        }
    }

    fn get_guest_name(&self) -> &String {
        &self.config_machine.name
    }

    fn get_network(&self) -> anyhow::Result<String> {
        Ok(get_guest_network(self)?.switch)
    }

    fn get_machine_definition(&self) -> Machine {
        self.config_machine.clone()
    }

    fn get_machine_definition_mut(&mut self) -> &mut Machine {
        &mut self.config_machine
    }

    // fn set_mac_address(&mut self, mac_address: MacAddress) {
    //     self.mac_address = Some(mac_address);
    // }

    fn get_mac_address(&self) -> anyhow::Result<MacAddress> {
        MacAddress::new(
            self.config_machine.network.as_ref()
                .context("getting mac address for guest")?.mac.clone()
        )
    }

    fn get_gateway(&self) -> Option<String> {
        if let Some(net) = &self.config_machine.network {
            net.gateway.clone()
        } else {
            None
        }
    }

    fn get_guest_id(&self) -> u32 {
        self.unique_id
    }

    fn get_interface_name(&self, project_name: &String) -> String {
        components::get_guest_interface_name(project_name, self.unique_id)
    }

    fn get_reference_image(&self) -> anyhow::Result<Option<String>> {
        // TODO - this might refer to a local image that is not in dockerhub
        Ok(None)
    }
}

#[async_trait]
impl TestbedComponent for DockerGuest {

    fn set_testbed_host(&mut self, in_host: String) {
        self.testbed_host = Some(in_host)
    }

    fn get_testbed_host(&self) -> &Option<String> {
        &self.testbed_host
    }

    fn set_static_ip(&mut self, in_ip: String) {
        match self.config_machine.guest_type {
            GuestType::Docker(ref mut docker) => {
                docker.static_ip = Some(in_ip)
            }
            _ => unreachable!()
        }
    }

    fn get_static_ip(&self) -> Option<String> {
        match &self.config_machine.guest_type {
            GuestType::Docker(docker) => {
                docker.static_ip.clone()
            }
            _ => unreachable!()
        }
    }

    fn specialise(&mut self, context: &SpecialisationContext) -> anyhow::Result<()> {
        self.container_name = Some(format!("{}-{}", context.project_name, self.get_guest_name()).clone());

        self.change_dir = if context.master_host.eq(self.testbed_host.as_ref().unwrap()) {
            // is on master
            Some(context.project_folder_path.to_str().unwrap().to_string())
        } else {
            let testbed_host_name = self.get_testbed_host().clone().unwrap();
            let project_name = &context.project_name;
            let remote_host_username =
                &context.ssh_config.testbed_host_ssh_config[&testbed_host_name].user;
            Some(format!("/home/{remote_host_username}/testbed-projects/{project_name}/"))
        };

        // set hostname
        let name = self.get_guest_name().clone();
        match self.config_machine.guest_type {
            GuestType::Docker(ref mut docker_config) => {
                docker_config.hostname = format!("{}-{}", context.project_name, name);
            }
            _ => unreachable!(),
        }

        Ok(())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// This struct represents all testbed guests that will utilise AVD backend
pub struct AndroidGuest {
    pub config_machine: Machine,
    pub testbed_host: Option<String>,
    pub avd_location: Option<PathBuf>,
    pub namespace: Option<String>,
    pub veth_name: Option<String>,
    pub unique_id: u32,
}

impl TestbedGuest for AndroidGuest {
    fn from_config_machine(in_config_machine: &Machine, unique_id: u32) -> Self where Self: Sized {
        Self {
            config_machine: in_config_machine.clone(),
            testbed_host: None,
            avd_location: None,
            namespace: None,
            veth_name: None,
            unique_id,
        }
    }

    fn get_guest_name(&self) -> &String {
        &self.config_machine.name
    }

    fn get_network(&self) -> anyhow::Result<String> {
        Ok(get_guest_network(self)?.switch)
    }

    fn get_machine_definition(&self) -> Machine {
        self.config_machine.clone()
    }

    fn get_machine_definition_mut(&mut self) -> &mut Machine {
        &mut self.config_machine
    }

    // fn set_mac_address(&mut self, mac_address: MacAddress) {
    //     self.mac_address = Some(mac_address);
    // }

    fn get_mac_address(&self) -> anyhow::Result<MacAddress> {
        MacAddress::new(
            self.config_machine.network.as_ref()
                .context("getting mac address for guest")?.mac.clone()
        )
    }

    fn get_gateway(&self) -> Option<String> {
        if let Some(net) = &self.config_machine.network {
            net.gateway.clone()
        } else {
            None
        }
    }

    fn get_guest_id(&self) -> u32 {
        self.unique_id
    }

    fn get_interface_name(&self, project_name: &String) -> String {
        components::get_guest_interface_name(project_name, self.unique_id)
    }

    fn get_reference_image(&self) -> anyhow::Result<Option<String>> {
        Ok(None)
    }
}

#[async_trait]
impl TestbedComponent for AndroidGuest {

    fn set_testbed_host(&mut self, in_host: String) {
        self.testbed_host = Some(in_host)
    }

    fn get_testbed_host(&self) -> &Option<String> {
        &self.testbed_host
    }

    fn set_static_ip(&mut self, in_ip: String) {
        match self.config_machine.guest_type {
            GuestType::Android(ref mut android) => {
                android.static_ip = Some(in_ip)
            }
            _ => unreachable!()
        }
    }

    fn get_static_ip(&self) -> Option<String> {
        match self.config_machine.guest_type {
            GuestType::Android(ref android) => {
                android.static_ip.clone()
            }
            _ => unreachable!()
        }
    }

    fn specialise(&mut self, context: &SpecialisationContext) -> anyhow::Result<()> {
        // create a namespace based on the project name, guest name, and append with nmspc
        // there is no limit to name for namespace, only interface
        let project_name = &context.project_name;
        let guest_name = &self.config_machine.name;
        self.namespace = Some(format!("{project_name}-{guest_name}-nmspc"));

        // create veth names based on the guest name, note that we will append either "-in" or "-out",
        // so the larger of the two takes 4 characters of the 15 available. Also given that there
        // is a possibility of collisions based on the name of the guest, just use random characters
        let rand_string: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(3)
            .map(char::from)
            .collect();
        self.veth_name = Some(format!("avdveth-{rand_string}"));

        Ok(())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
