use std::any::Any;
use anyhow::{bail, Context};
use async_trait::async_trait;
use kvm_compose_schemas::kvm_compose_yaml::network_old::NetworkInterface;
use crate::components::{SpecialisationContext, TestbedComponent, TestbedNetworkInterface, TestbedNetworkInterfaceComponent};

// this file describes the external network interfaces that can be added to the testbed network

pub fn get_network_interface_from_config(in_network_interface: &NetworkInterface, in_id: u16) -> Box<dyn TestbedNetworkInterfaceComponent + Sync + Send> {
    return Box::new(LogicalNetworkInterface::from_config(in_network_interface, in_id));
}

pub struct LogicalNetworkInterface {
    pub config: NetworkInterface,
    pub testbed_host: Option<String>,
    pub static_ip: Option<String>,
    pub id: u16,
}

impl TestbedNetworkInterface for LogicalNetworkInterface {
    fn from_config(in_network_interface: &NetworkInterface, in_id: u16) -> Self {
        Self {
            config: in_network_interface.clone(),
            testbed_host: Some(in_network_interface.testbed_host.clone()),
            static_ip: None,
            id: in_id,
        }
    }

    fn get_interface(&self) -> &String {
        &self.config.nic_name
    }

    fn get_id(&self) -> u16 {
        self.id.clone()
    }
}

#[async_trait]
impl TestbedComponent for LogicalNetworkInterface {

    fn set_testbed_host(&mut self, in_host: String) {
        self.testbed_host = Some(in_host)
    }

    fn get_testbed_host(&self) -> &Option<String> {
        &self.testbed_host
    }

    fn get_static_ip(&self) -> Option<String> {
        self.static_ip.clone()
    }

    fn specialise(&mut self, context: &SpecialisationContext) -> anyhow::Result<()> {

        // for now, we can only support network interfaces that are assigned to bridges that are
        // on the main testbed host - so if the network interface is not on the main testbed
        // host then prevent generate artefacts
        // TODO - with OVN network backend this is not a limitation anymore
        if !self.testbed_host.as_ref().context("getting testbed host config")?.eq(&context.main_host) {
            // not on main testbed
            bail!("the network interface {} is not on the main testbed, currently not supported", self.config.nic_name);
        }

        Ok(())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
