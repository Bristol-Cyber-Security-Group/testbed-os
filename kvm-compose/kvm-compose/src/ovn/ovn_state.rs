use std::collections::{HashMap, HashSet};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::net::IpAddr;
use anyhow::{bail, Context};
use serde::{Deserialize, Serialize};
use kvm_compose_schemas::kvm_compose_yaml::network::acl::ACLRule;
use crate::ovn::components::logical_switch_port::{LogicalSwitchPortType};
use crate::ovn::components::{MacAddress, OvnIpAddr};
use crate::ovn::LogicalOperationResult;
use crate::ovn::configuration::external_gateway::OvnExternalGateway;
use crate::ovn::configuration::nat::{OvnNat, OvnNatType};
use crate::ovn::configuration::route::OvnRoute;
use crate::ovn::components::logical_router::LogicalRouter;
use crate::ovn::components::logical_router_port::LogicalRouterPort;
use crate::ovn::components::logical_switch::LogicalSwitch;
use crate::ovn::components::logical_switch_port::LogicalSwitchPort;
use crate::ovn::components::ovs::OvsPort;
use crate::ovn::components::acl::{ACLRecordType, LogicalACLRecord};
use crate::ovn::configuration::dhcp::{DhcpDatabaseEntry, SwitchDhcpOptions};


/// This represents the OVN logical network components and configuration. Components are the main
/// basic concepts in OVN like switches and routers. Configuration are the optional settings that
/// can be applied on a Component i.e. setting a router as an external gateway.
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct OvnNetwork {
    // components
    pub switches: HashMap<String, LogicalSwitch>,
    pub switch_ports: HashMap<String, LogicalSwitchPort>,
    pub routers: HashMap<String, LogicalRouter>,
    pub router_ports: HashMap<String, LogicalRouterPort>,
    pub ovs_ports: HashMap<String, OvsPort>,
    pub acl: HashMap<String, LogicalACLRecord>,
    // TODO - track the OVN chassis as well?
    // database entries
    pub dhcp_options: HashSet<DhcpDatabaseEntry>,
}

impl OvnNetwork {
    pub fn new() -> Self {
        Self {
            switches: Default::default(),
            switch_ports: Default::default(),
            routers: Default::default(),
            router_ports: Default::default(),
            ovs_ports: Default::default(),
            acl: Default::default(),
            dhcp_options: Default::default(),
        }
    }

    /// Return true if the logical switch already exists, otherwise return false
    fn switch_exists(
        &self,
        name: &String,
    ) -> bool {
        if let Some(existing) = self.switches.get(name) {
            tracing::debug!("switch {} already exists in OvnNetwork", existing.name);
            return true;
        }
        false
    }

    /// Adds a logical switch to the network representation. Will not overwrite the existing
    /// definition if it already exists.
    pub fn add_switch(
        &mut self,
        name: String,
        subnet_ip: IpAddr,
        subnet_mask: u16
    ) -> anyhow::Result<(), LogicalOperationResult> {
        if self.switch_exists(&name) {
            return Err(LogicalOperationResult::AlreadyExists { name });
        }
        self.switches.insert(
            name.clone(),
            LogicalSwitch::new(
                name,
                subnet_ip,
                subnet_mask,
            )
        );
        Ok(())
    }

    /// Deletes a logical switch from the network representation.
    pub fn del_switch(
        &mut self,
        name: &String,
    ) -> anyhow::Result<(), LogicalOperationResult> {
        if !self.switch_exists(name) {
            return Err(LogicalOperationResult::DoesNotExist { name: name.into()  });
        }
        // make sure no it has no assigned ports
        let child_ports = self.switch_ports.iter()
            .filter(|(_, data)| data.parent_switch.eq(name))
            .count();
        if child_ports > 0 {
            return Err(LogicalOperationResult::HasChildren { name: name.into() })
        }
        self.switches.remove(
            name,
        );
        Ok(())
    }

    /// Checks if the logical switch port exists in the network representation, return true if it
    /// already exists otherwise return false.
    fn lsp_exists(
        &self,
        name: &String,
    ) -> bool {
        if let Some(existing) = self.switch_ports.get(name) {
            tracing::debug!("switch port {} already exists in OvnNetwork", existing.name);
            return true;
        }
        false
    }

    /// Adds a logical switch port of type internal. Will not update the definition if it already
    /// exists.
    #[allow(clippy::too_many_arguments)]
    pub fn add_lsp_internal(
        &mut self,
        name: String,
        parent_switch: String,
        ovs_port_name: String, // TODO - we let the guests make the port now, not needed?
        ip: OvnIpAddr,
        chassis_name: Option<String>,
        mac_address: MacAddress,
        provider_network_name: Option<String>,
    ) -> anyhow::Result<(), LogicalOperationResult> {
        // make sure the parent switch exists
        if !self.switch_exists(&parent_switch) {
            return Err(LogicalOperationResult::ParentDoesNotExist { name, parent: parent_switch });
        }
        // check it doesn't already exist
        if self.lsp_exists(&name) {
            return Err(LogicalOperationResult::AlreadyExists { name });
        }
        let port_type = LogicalSwitchPortType::new_internal(
            ovs_port_name,
            ip,
            chassis_name,
            mac_address,
            provider_network_name,
        );
        let lsp = LogicalSwitchPort::new(
            name.clone(),
            parent_switch,
            port_type,
        );
        self.switch_ports.insert(
            name,
            lsp,
        );
        Ok(())
    }

    /// Adds a logical switch port that corresponds to a logical router port. Will not update if the
    /// switch definition already exists.
    pub fn add_lsp_router(
        &mut self,
        name: String,
        parent_switch: String,
        mac_address: MacAddress,
        router_port_name: String,
    ) -> anyhow::Result<(), LogicalOperationResult> {
        // make sure the parent switch exists
        if !self.switch_exists(&parent_switch) {
            return Err(LogicalOperationResult::ParentDoesNotExist { name, parent: parent_switch });
        }
        // check it doesn't already exist
        if self.lsp_exists(&name) {
            return Err(LogicalOperationResult::AlreadyExists { name });
        }
        let port_type = LogicalSwitchPortType::new_router(
            router_port_name,
            mac_address,
        );
        let lsp = LogicalSwitchPort::new(
            name.clone(),
            parent_switch,
            port_type,
        );
        self.switch_ports.insert(
            name,
            lsp,
        );
        Ok(())
    }

    /// Adds a logical switch port that corresponds to a local network on a chassis. Will not update
    /// if the switch definition already exists.
    pub fn add_lsp_localnet(
        &mut self,
        name: String,
        parent_switch: String,
        provider_network_name: String,
    ) -> anyhow::Result<(), LogicalOperationResult> {
        // make sure the parent switch exists
        if !self.switch_exists(&parent_switch) {
            return Err(LogicalOperationResult::ParentDoesNotExist { name, parent: parent_switch });
        }
        // check it doesn't already exist
        if self.lsp_exists(&name) {
            return Err(LogicalOperationResult::AlreadyExists { name });
        }
        let port_type = LogicalSwitchPortType::new_localnet(
            provider_network_name,
        );
        let lsp = LogicalSwitchPort::new(
            name.clone(),
            parent_switch,
            port_type,
        );
        self.switch_ports.insert(
            name,
            lsp,
        );
        Ok(())
    }

    /// Deletes a logical switch port definition from the network representation.
    pub fn del_lsp(
        &mut self,
        name: &String,
    ) -> anyhow::Result<(), LogicalOperationResult> {
        // check it exists
        if !self.lsp_exists(name) {
            return Err(LogicalOperationResult::DoesNotExist { name: name.into() });
        }
        self.switch_ports.remove(name);
        Ok(())
    }

    /// Checks if the router definition exists in the network representation, if exists returns true
    /// otherwise returns false
    fn router_exists(
        &self,
        name: &String,
    ) -> bool {
        if let Some(existing) = self.routers.get(name) {
            tracing::debug!("router {} already exists in OvnNetwork", existing.name);
            return true;
        }
        false
    }

    /// Adds a logical router to the network representation. Will not update the definition if it
    /// already exists.
    pub fn add_router(
        &mut self,
        name: String,
    ) -> anyhow::Result<(), LogicalOperationResult> {
        if self.router_exists(&name) {
            return Err(LogicalOperationResult::AlreadyExists { name });
        }
        self.routers.insert(
            name.clone(),
            LogicalRouter::new(name),
        );
        Ok(())
    }

    /// Removes a logical router from the network representation.
    pub fn del_router(
        &mut self,
        name: &String,
    ) -> anyhow::Result<(), LogicalOperationResult> {
        if !self.router_exists(name) {
            return Err(LogicalOperationResult::DoesNotExist { name: name.into() });
        }
        // make sure no it has no assigned ports
        let child_ports = self.router_ports.iter()
            .filter(|(_, data)| data.parent_router.eq(name))
            .count();
        if child_ports > 0 {
            return Err(LogicalOperationResult::HasChildren { name: name.into() })
        }
        self.routers.remove(
            name,
        );
        Ok(())
    }

    /// Checks if the logical router port exists in the network representation, return true if it
    /// already exists otherwise return false.
    fn lrp_exists(
        &self,
        name: &String,
    ) -> bool {
        if let Some(existing) = self.router_ports.get(name) {
            tracing::debug!("router port {} already exists in OvnNetwork", existing.name);
            return true;
        }
        false
    }

    /// Adds a logical router port to the network representation, if it already exists it will not
    /// update the existing definition.
    pub fn add_lrp(
        &mut self,
        name: String,
        parent_router: String,
        mac_address: MacAddress,
        ip: IpAddr,
        mask: u16,
        chassis_name: Option<String>,
    ) -> anyhow::Result<(), LogicalOperationResult> {
        // make sure the parent switch exists
        if !self.router_exists(&parent_router) {
            return Err(LogicalOperationResult::ParentDoesNotExist { name, parent: parent_router });
        }
        // check it doesn't already exist
        if self.lrp_exists(&name) {
            return Err(LogicalOperationResult::AlreadyExists { name });
        }
        let lrp = LogicalRouterPort::new(
            name.clone(),
            parent_router,
            mac_address,
            ip,
            mask,
            chassis_name,
        );
        self.router_ports.insert(
            name,
            lrp,
        );
        Ok(())
    }

    /// Deletes the logical router port definition from the network representation.
    pub fn del_lrp(
        &mut self,
        name: &String,
    ) -> anyhow::Result<(), LogicalOperationResult> {
        // check it exists
        if !self.lrp_exists(name) {
            return Err(LogicalOperationResult::DoesNotExist { name: name.into() });
        }
        self.router_ports.remove(name);
        Ok(())
    }

    fn ovs_exists(
        &self,
        name: &String,
    ) -> bool {
        if let Some(existing) = self.ovs_ports.get(name) {
            tracing::debug!("OVS port {} already exists in OvnNetwork", existing.name);
            return true;
        }
        false
    }

    pub fn ovs_add_port(
        &mut self,
        name: String,
        integration_bridge_name: String,
        lsp_name: String,
        chassis: String,
    ) -> anyhow::Result<(), LogicalOperationResult> {
        // check it exists
        if self.ovs_exists(&name) {
            return Err(LogicalOperationResult::AlreadyExists { name });
        }
        self.ovs_ports.insert(
            name.clone(),
            OvsPort::new(
                name,
                integration_bridge_name,
                lsp_name,
                chassis,
            )
        );
        Ok(())
    }

    pub fn ovs_del_port(
        &mut self,
        name: &String,
    ) -> anyhow::Result<(), LogicalOperationResult> {
        // check it exists
        if !self.ovs_exists(name) {
            return Err(LogicalOperationResult::DoesNotExist { name: name.into() });
        }
        self.ovs_ports.remove(name);
        Ok(())
    }

    pub fn lr_route_add(
        &mut self,
        router_name: String,
        prefix: OvnIpAddr,
        next_hop: IpAddr,
    ) -> anyhow::Result<(), LogicalOperationResult> {
        // TODO check if already exists
        let route = OvnRoute::new(
            router_name.clone(),
            prefix.clone(),
            next_hop,
        );
        if route.is_err() {
            return Err(LogicalOperationResult::Error {
                msg: format!("Could not create OvnRoute, incorrect configuration: {:?}", route),
            })
        }
        // get parent router
        if let Some(parent_router) = self.routers.get_mut(&router_name) {
            // insert route
            parent_router.routing.0.insert(
                (router_name.clone(), prefix.to_string(), next_hop.to_string()),
                route.unwrap(),
            );
        } else {
            // could not find parent router
            return Err(LogicalOperationResult::ParentDoesNotExist {
                name: format!("Route(prefix: {prefix:?}, next hop: {next_hop:?})"),
                parent: router_name,
            })
        }
        Ok(())
    }

    pub fn lr_route_del(
        &mut self,
        _router_name: String,
        _prefix: OvnIpAddr,
        _next_hop: IpAddr,
    ) -> anyhow::Result<(), LogicalOperationResult> {
        // TODO check exists
        todo!()
    }

    pub fn lrp_add_external_gateway(
        &mut self,
        router_name: String,
        router_port_name: String,
        chassis_name: String,
    ) -> anyhow::Result<(), LogicalOperationResult> {
        let name_tuple = (router_port_name.clone(), chassis_name.clone());
        // TODO check exists

        // get parent router
        if let Some(parent_router) = self.routers.get_mut(&router_name) {
            // insert
            parent_router.external_gateway.0.insert(
                name_tuple,
                OvnExternalGateway::new(
                    router_port_name,
                    chassis_name,
                ),
            );
        } else {
            // could not find parent router
            return Err(LogicalOperationResult::ParentDoesNotExist {
                name: format!("ExternalGateway(router_port_name: {router_port_name:?}, chassis_name: {chassis_name:?})"),
                parent: router_name,
            })
        }
        Ok(())
    }

    pub fn lrp_del_external_gateway(
        &mut self,
        router_port_name: String,
        chassis_name: String,
    ) -> anyhow::Result<(), LogicalOperationResult> {
        // TODO check exists
        let _name_tuple = (router_port_name.clone(), chassis_name.clone());
        todo!()
    }

    pub fn lr_add_nat(
        &mut self,
        logical_router_name: String,
        nat_type: OvnNatType,
        external_ip: OvnIpAddr,
        logical_ip: OvnIpAddr,
    ) -> anyhow::Result<(), LogicalOperationResult> {
        // TODO check exists
        let name_tuple = (logical_router_name.clone(), nat_type.to_string(), external_ip.to_string());
        let nat = OvnNat::new(
            logical_router_name.clone(),
            external_ip.clone(),
            logical_ip.clone(),
            nat_type.clone(),
        );
        if nat.is_err() {
            return Err(LogicalOperationResult::Error {
                msg: format!("Could not create OvnNat, incorrect configuration: {:?}", nat),
            })
        }
        // get parent router
        if let Some(parent_router) = self.routers.get_mut(&logical_router_name) {
            // insert
            parent_router.nat.0.insert(
                name_tuple,
                nat.unwrap(),
            );
        } else {
            // could not find parent router
            return Err(LogicalOperationResult::ParentDoesNotExist {
                name: format!("ExternalGateway(external_ip: {external_ip:?}, logical_ip: {logical_ip:?} nat_type: {nat_type:?})"),
                parent: logical_router_name,
            })
        }
        Ok(())
    }

    pub fn lr_del_nat(
        &mut self,
        logical_router_name: String,
        nat_type: OvnNatType,
        external_ip: OvnIpAddr,
    ) -> anyhow::Result<(), LogicalOperationResult> {
        // TODO check exists
        let _name_tuple = (logical_router_name.clone(), nat_type.to_string(), external_ip.to_string());
        todo!()
    }

    pub fn switch_get(
        &self,
        name: &String,
    ) -> anyhow::Result<&LogicalSwitch> {
        self.switches.get(name)
            .context(format!("Getting switch name {name}"))
    }

    pub fn switch_get_mut(
        &mut self,
        name: &String,
    ) -> anyhow::Result<&mut LogicalSwitch> {
        self.switches.get_mut(name)
            .context(format!("Getting switch name {name}"))
    }

    pub fn switch_port_get(
        &self,
        name: &String,
    ) -> anyhow::Result<&LogicalSwitchPort> {
        self.switch_ports.get(name)
            .context(format!("Getting switch port name {name}"))
    }

    pub fn switch_port_get_mut(
        &mut self,
        name: &String,
    ) -> anyhow::Result<&mut LogicalSwitchPort> {
        self.switch_ports.get_mut(name)
            .context(format!("Getting switch port name {name}"))
    }

    pub fn router_get(
        &self,
        name: &String,
    ) -> anyhow::Result<&LogicalRouter> {
        self.routers.get(name)
            .context(format!("Getting router name {name}"))
    }

    pub fn router_get_mut(
        &mut self,
        name: &String,
    ) -> anyhow::Result<&mut LogicalRouter> {
        self.routers.get_mut(name)
            .context(format!("Getting router name {name}"))
    }

    pub fn router_port_get(
        &self,
        name: &String,
    ) -> anyhow::Result<&LogicalRouterPort> {
        self.router_ports.get(name)
            .context(format!("Getting router port name {name}"))
    }

    pub fn router_port_get_mut(
        &mut self,
        name: &String,
    ) -> anyhow::Result<&mut LogicalRouterPort> {
        self.router_ports.get_mut(name)
            .context(format!("Getting router port name {name}"))
    }

    pub fn ovs_port_get(
        &self,
        name: &String,
    ) -> anyhow::Result<&OvsPort> {
        self.ovs_ports.get(name)
            .context(format!("Getting OVS port name {name}"))
    }

    pub fn ovs_port_get_mut(
        &mut self,
        name: &String,
    ) -> anyhow::Result<&mut OvsPort> {
        self.ovs_ports.get_mut(name)
            .context(format!("Getting OVS port name {name}"))
    }

    pub fn route_rule_get(
        &self,
        router_name: &String,
        prefix: &OvnIpAddr,
        next_hop: &IpAddr,
    ) -> anyhow::Result<&OvnRoute> {
        let name_tuple = (router_name.into(), prefix.to_string(), next_hop.to_string());
        self.routers.get(router_name)
            .context(format!("Getting router {} for route_rule_get", router_name))?
            .routing.0.get(&name_tuple)
            .context(format!("Getting Route rule name tuple {name_tuple:?}"))
    }

    pub fn route_rule_get_mut(
        &mut self,
        router_name: &String,
        prefix: &OvnIpAddr,
        next_hop: &IpAddr,
    ) -> anyhow::Result<&mut OvnRoute> {
        let name_tuple = (router_name.into(), prefix.to_string(), next_hop.to_string());
        self.routers.get_mut(router_name)
            .context(format!("Getting router {} for route_rule_get_mut", router_name))?
            .routing.0.get_mut(&name_tuple)
            .context(format!("Getting Route rule name tuple {name_tuple:?}"))
    }

    pub fn add_dhcp_option(
        &mut self,
        router_name: &String,
        switch_name: &String,
        exclude_ips: &str,
    ) -> anyhow::Result<(), LogicalOperationResult> {

        // borrow checker avoidance - we will work on their names up here then below once we have
        // everything we will then get the mutable versions and apply the changes
        let sw = switch_name.clone();
        let rt = router_name.clone();

        // check if these resources exist
        let switch = self.switch_get(switch_name)
            .or(Err(LogicalOperationResult::DoesNotExist { name: switch_name.clone() }))?.clone();
        self.router_get(router_name)
            .or(Err(LogicalOperationResult::DoesNotExist { name: router_name.clone() }))?;
        let lsp_lrp_port_pair = self.get_lsp_lrp_pair(&sw, &rt)?;

        // given we have a switch, we need to get all logical switch ports on this switch that have
        // the ip address as dynamic - they will all share the same subnet therefore will need the
        // same DHCP rule UUID from OVN
        let mut lsp_dynamic = Vec::new();
        for (lsp_name, lsp_data) in self.switch_ports.iter() {
            if lsp_data.parent_switch.eq(&sw) {
                if let LogicalSwitchPortType::Internal { ip, .. } = &lsp_data.port_type {
                    if ip == &OvnIpAddr::Dynamic {
                        // only take LSP type internal AND has dynamic as IP address for
                        // this switch
                        // take a copy as we will get the mutable version later
                        lsp_dynamic.push(lsp_name.clone());
                    }
                }
            }
        }

        if lsp_dynamic.is_empty() {
            return Err(LogicalOperationResult::Error {
                msg: format!("the switch {} did not have any internal switch ports with a dynamic ip", &sw)
            })
        }
        // need the ip of the LRP without a subnet mask
        let lrp_ip_no_mask = match lsp_lrp_port_pair.1.ip {
            OvnIpAddr::Subnet { ip, .. } => ip,
            _ => unreachable!(),
        };

        // create dhcp options database rule
        let dhcp = DhcpDatabaseEntry {
            cidr: switch.subnet.clone(),
            lease_time: "3600".to_string(),
            router: lrp_ip_no_mask.to_string(),
            server_id: lrp_ip_no_mask.to_string(),
            server_mac: lsp_lrp_port_pair.1.mac_address.clone(),
        };

        // create a hash of entry
        let mut s = DefaultHasher::new();
        dhcp.hash(&mut s);
        let dhcp_hash = s.finish();
        // store dhcp option
        self.dhcp_options.insert(dhcp);
        // now link this rule to the logical switch port(s)
        for lsp_name in lsp_dynamic {
            let lsp = self.switch_ports.get_mut(&lsp_name)
                .unwrap(); // unwrap as we know it definitely exists
            lsp.dhcp_options_uuid = Some(dhcp_hash);
        }

        // add config to logical switch
        let switch = self.switches.get_mut(&sw)
            .unwrap(); // unwrap as we definitely know it exists
        switch.dhcp = Some(SwitchDhcpOptions { exclude_ips: exclude_ips.to_owned() });

        Ok(())
    }

    pub fn get_lsp_lrp_pair(
        &self,
        switch_name: &String,
        router_name: &String,
    ) -> anyhow::Result<(&LogicalSwitchPort, &LogicalRouterPort), LogicalOperationResult> {
        let switch = self.switches.get(switch_name)
            .ok_or(LogicalOperationResult::DoesNotExist { name: switch_name.clone() })?;
        let router = self.routers.get(router_name)
            .ok_or(LogicalOperationResult::DoesNotExist { name: router_name.clone() })?;
        // the switch should have a switch port with type router, and this port will point to it's
        // router port pair, which will have a parent router ... if we cannot find this link, then
        // there is no pair
        for lsp_data in self.switch_ports.values() {
            // LSP must have the parent port specified
            if !lsp_data.parent_switch.eq(&switch.name) {continue;}
            // must be type router
            if let LogicalSwitchPortType::Router { router_port_name, .. } = &lsp_data.port_type {
                // LSP is type router, now try to match to the LRP
                for (lrp_name, lrp_data) in &self.router_ports {
                    // find the LRP with our router as parent
                    if lrp_data.parent_router.eq(&router.name) {
                        // see if the pair router port for this switch port type router match
                        if lrp_name.eq(router_port_name) {
                            // we have found the pair
                            return Ok((lsp_data, lrp_data))
                        }
                    }
                }
            }
        }
        Err(LogicalOperationResult::Error {
            msg: format!("Could not find LSP type route and LRP pair for switch {} and router {}", &switch_name, &router_name),
        })
    }

    pub fn add_switch_acl(
        &mut self,
        acl_name: &String,
        entity_name: String,
        _type: ACLRecordType,
        acl_rule: &ACLRule
    ) -> anyhow::Result<(), LogicalOperationResult> {
        // make sure it doesn't already exist
        if self.acl.contains_key(acl_name) {
            return Err(LogicalOperationResult::AlreadyExists { name: acl_name.to_string() });
        }

        self.acl.insert(acl_name.clone(), LogicalACLRecord::new(
            entity_name,
            _type,
            acl_rule.direction.clone(),
            acl_rule.priority,
            acl_rule._match.clone(),
            acl_rule.action.clone(),
            acl_name.clone(),
        ));

        Ok(())
    }

    /// Validate the OvnNetwork to make sure all relations are valid. While the logical switch
    /// and logical switch port, and logical router and logical router port do have a mechanism
    /// to prevent parent-less ports, we must validate everything else.
    pub fn validate(
        &self,
    ) -> anyhow::Result<()> {
        // make sure ports match subnets
        // for (_, lsp) in &self.switch_ports {
        //     // get parent router subnet
        //     let parent_subnet = &self.switches.get(&lsp.parent_switch)
        //         .context("getting parent switch for logical switch port")?
        //         .subnet;
        //     // TODO - convert ip to integer and mask the subnet with the port IP to check
        //     // TODO - also do logical router port on the switch
        // }

        // make sure each port has a unique IP address, record the ip and name of resource for
        // reporting the error
        let mut ip_set = HashMap::new();
        for (name, lsp) in &self.switch_ports {
            if let LogicalSwitchPortType::Internal { ip, .. } = &lsp.port_type {
                let option = ip_set.get(ip);
                if option.is_some() {
                    bail!("ports {} and {} both have the same ip {}", option.unwrap(), name, ip.to_string());
                } else {
                    if ip.to_string().eq("dynamic") {
                        continue;
                    }
                    ip_set.insert(ip.clone(), name);
                }
            }
        }
        // now also do router ports
        for (name, lrp) in &self.router_ports {
            let get = match &lrp.ip {
                OvnIpAddr::Ip(ip) => {
                    // need to convert IpAddr to OvnIpAddr
                    let ip = OvnIpAddr::Ip(*ip);
                    (ip_set.get(&ip), ip)
                },
                OvnIpAddr::Dynamic => {
                    // ignore dynamic
                    continue;
                }
                OvnIpAddr::Subnet { ip, .. } => {
                    // need to convert IpAddr to OvnIpAddr
                    let ip = OvnIpAddr::Ip(*ip);
                    (ip_set.get(&ip), ip)
                }
            };
            // get.0 = Option of the get on ip_set
            // get.1 is the OvnIpAddr for this logical router port
            if get.0.is_some() {
                bail!("ports {} and {} both have the same ip {}", get.0.unwrap(), name, &get.1.to_string());
            } else {
                ip_set.insert(get.1, name);
            }
        }


        // TODO - check if external gateway ips match external bridge ips
        // TODO - check LRP gateway chassis matches kvm-compose-config
        // TODO - check static routes have valid next hops?
        // TODO - check NAT rules
        // TODO - check if a router and switch havent been linked twice
        // TODO - check if ip is dynamic, there is a corresponding DHCP rule (the opposite exists)

        Ok(())
    }

}

#[cfg(test)]
mod tests {
    use std::net::Ipv4Addr;
    use kvm_compose_schemas::kvm_compose_yaml::network::acl::{ACLAction, ACLDirection};
    use super::*;

    #[test]
    fn test_switch_and_ports() -> anyhow::Result<(), LogicalOperationResult> {
        let mut ovn = OvnNetwork::new();
        ovn.add_switch(
            "sw0".into(),
            IpAddr::V4(Ipv4Addr::new(10, 0, 0, 0)),
            24)?;
        // try adding again
        let op_res = ovn.add_switch(
            "sw0".into(),
            IpAddr::V4(Ipv4Addr::new(20, 10, 10, 0)),
            24);
        assert_eq!(op_res, Err(LogicalOperationResult::AlreadyExists { name: "sw0".to_string() }));
        // add a port type internal
        ovn.add_lsp_internal(
            "sw0-port0".into(),
            "sw0".into(),
            "ovs-sw0-port0".into(),
            OvnIpAddr::Ip(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 2))),
            Some("ovn".into()),
            MacAddress::new("00:00:00:00:00:01".into()).unwrap(),
            Some("public".into()),
        )?;
        // try to add another port with same name
        let op_res = ovn.add_lsp_internal(
            "sw0-port0".into(),
            "sw0".into(),
            "ovs-sw0-port0".into(),
            OvnIpAddr::Ip(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 2))),
            Some("ovn".into()),
            MacAddress::new("00:00:00:00:00:02".into()).unwrap(),
            Some("public".into()),
        );
        assert_eq!(op_res, Err(LogicalOperationResult::AlreadyExists { name: "sw0-port0".to_string() }));
        // try to add a port to a switch that doesnt exist
        let op_res = ovn.add_lsp_internal(
            "sw1-port0".into(),
            "sw1".into(),
            "ovs-sw1-port0".into(),
            OvnIpAddr::Ip(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 2))),
            Some("ovn".into()),
            MacAddress::new("00:00:00:00:00:02".into()).unwrap(),
            Some("public".into()),
        );
        assert_eq!(op_res, Err(LogicalOperationResult::ParentDoesNotExist { name: "sw1-port0".to_string(), parent: "sw1".to_string() }));
        // add a port type router
        ovn.add_lsp_router(
            "sw0-port1".into(),
            "sw0".into(),
            MacAddress::new("00:00:00:00:00:02".into()).unwrap(),
            "lr0-port0".into(),
        )?;
        // add another port type router with same name
        let op_res = ovn.add_lsp_router(
            "sw0-port1".into(),
            "sw0".into(),
            MacAddress::new("00:00:00:00:00:02".into()).unwrap(),
            "lr0-port0".into(),
        );
        assert_eq!(op_res, Err(LogicalOperationResult::AlreadyExists { name: "sw0-port1".to_string() }));
        // add type router with switch that doesnt exist
        let op_res = ovn.add_lsp_router(
            "sw0-port1".into(),
            "sw1".into(),
            MacAddress::new("00:00:00:00:00:02".into()).unwrap(),
            "lr0-port0".into(),
        );
        assert_eq!(op_res, Err(LogicalOperationResult::ParentDoesNotExist { name: "sw0-port1".to_string(), parent: "sw1".to_string() }));
        // add a port type localnet
        ovn.add_lsp_localnet(
            "sw0-port2".into(),
            "sw0".into(),
            "public".into()
        )?;
        // add another port type localnet with same name
        let op_res = ovn.add_lsp_localnet(
            "sw0-port2".into(),
            "sw0".into(),
            "public".into()
        );
        assert_eq!(op_res, Err(LogicalOperationResult::AlreadyExists { name: "sw0-port2".to_string() }));
        // add localnet with switch that doesnt exist
        let op_res = ovn.add_lsp_localnet(
            "sw0-port2".into(),
            "sw1".into(),
            "public".into()
        );
        assert_eq!(op_res, Err(LogicalOperationResult::ParentDoesNotExist { name: "sw0-port2".to_string(), parent: "sw1".to_string() }));


        // make sure we cant delete a switch before its ports
        let op_res = ovn.del_switch(&"sw0".into());
        assert_eq!(op_res, Err(LogicalOperationResult::HasChildren { name: "sw0".to_string() }));
        // now delete everything
        ovn.del_lsp(&"sw0-port2".into())?;
        ovn.del_lsp(&"sw0-port1".into())?;
        ovn.del_lsp(&"sw0-port0".into())?;
        // try to delete something that doesnt exist
        let op_res = ovn.del_lsp(&"sw0-port0".into());
        assert_eq!(op_res, Err(LogicalOperationResult::DoesNotExist { name: "sw0-port0".to_string() }));
        // delete switch
        ovn.del_switch(&"sw0".into())?;
        // delete switch that doesnt exist
        let op_res = ovn.del_switch(&"sw0".into());
        assert_eq!(op_res, Err(LogicalOperationResult::DoesNotExist { name: "sw0".to_string() }));

        Ok(())
    }

    #[test]
    fn test_router_and_ports() -> anyhow::Result<(), LogicalOperationResult> {
        let mut ovn = OvnNetwork::new();
        ovn.add_router("lr0".into())?;
        // try to add again
        let op_res = ovn.add_router("lr0".into());
        assert_eq!(op_res, Err(LogicalOperationResult::AlreadyExists { name: "lr0".to_string() }));
        // add port
        ovn.add_lrp(
            "lr0-port0".into(),
            "lr0".into(),
            MacAddress::new("00:00:00:00:00:01".into()).unwrap(),
            IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)),
            24,
            Some("ovn".into())
        )?;
        // add port with same name
        let op_res = ovn.add_lrp(
            "lr0-port0".into(),
            "lr0".into(),
            MacAddress::new("00:00:00:00:00:02".into()).unwrap(),
            IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)),
            24,
            Some("ovn".into())
        );
        assert_eq!(op_res, Err(LogicalOperationResult::AlreadyExists { name: "lr0-port0".to_string() }));
        // add port to router that doesnt exist
        let op_res = ovn.add_lrp(
            "lr0-port0".into(),
            "lr1".into(),
            MacAddress::new("00:00:00:00:00:03".into()).unwrap(),
            IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)),
            24,
            Some("ovn".into())
        );
        assert_eq!(op_res, Err(LogicalOperationResult::ParentDoesNotExist { name: "lr0-port0".to_string(), parent: "lr1".to_string() }));

        // try to delete router before ports
        let op_res = ovn.del_router(&"lr0".into());
        assert_eq!(op_res, Err(LogicalOperationResult::HasChildren { name: "lr0".to_string() }));
        // delete port
        ovn.del_lrp(&"lr0-port0".into())?;
        // delete port that doesnt exist
        let op_res = ovn.del_lrp(&"lr0-port0".into());
        assert_eq!(op_res, Err(LogicalOperationResult::DoesNotExist { name: "lr0-port0".to_string() }));
        // delete router
        ovn.del_router(&"lr0".into())?;
        // delete router that doesnt exist
        let op_res = ovn.del_router(&"lr0".into());
        assert_eq!(op_res, Err(LogicalOperationResult::DoesNotExist { name: "lr0".to_string() }));


        Ok(())
    }

    #[test]
    fn test_ovs_port() -> anyhow::Result<(), LogicalOperationResult> {
        let mut ovn = OvnNetwork::new();
        ovn.ovs_add_port(
            "ovs-sw0-port0".into(),
            "br-int".into(),
            "sw0-port0".into(),
            "ovn".into(),
        )?;
        // try to add again
        let op_res = ovn.ovs_add_port(
            "ovs-sw0-port0".into(),
            "br-int".into(),
            "sw0-port0".into(),
            "ovn".into(),
        );
        assert_eq!(op_res, Err(LogicalOperationResult::AlreadyExists { name: "ovs-sw0-port0".to_string() }));
        // delete
        ovn.ovs_del_port(&"ovs-sw0-port0".into())?;
        // delete again
        let op_res = ovn.ovs_del_port(&"ovs-sw0-port0".into());
        assert_eq!(op_res, Err(LogicalOperationResult::DoesNotExist { name: "ovs-sw0-port0".to_string() }));

        Ok(())
    }

    #[test]
    fn test_dhcp() -> anyhow::Result<(), LogicalOperationResult> {
        // TODO create switch and port with dynamic, make sure the dhcp rules exist

        let mut ovn = OvnNetwork::new();
        ovn.add_switch(
            "sw0".into(),
            IpAddr::V4(Ipv4Addr::new(10, 0, 0, 0)),
            24)?;
        // add port with dynamic
        ovn.add_lsp_internal(
            "sw0-port0".into(),
            "sw0".into(),
            "ovs-sw0-port0".into(),
            OvnIpAddr::Dynamic,
            Some("ovn".into()),
            MacAddress::new("00:00:00:00:00:01".into()).unwrap(),
            Some("public".into()),
        )?;
        // add port without
        ovn.add_lsp_internal(
            "sw0-port1".into(),
            "sw0".into(),
            "ovs-sw0-port1".into(),
            OvnIpAddr::Ip(IpAddr::V4(Ipv4Addr::new(10,0,0,10))),
            Some("ovn".into()),
            MacAddress::new("00:00:00:00:00:02".into()).unwrap(),
            Some("public".into()),
        )?;
        // add another port with dynamic
        ovn.add_lsp_internal(
            "sw0-port2".into(),
            "sw0".into(),
            "ovs-sw0-port0".into(),
            OvnIpAddr::Dynamic,
            Some("ovn".into()),
            MacAddress::new("00:00:00:00:00:03".into()).unwrap(),
            Some("public".into()),
        )?;
        // add the router and router port pair
        ovn.add_router("lr0".into())?;
        ovn.add_lrp(
            "lr0-port0".into(),
            "lr0".into(),
            MacAddress::new("00:00:00:00:00:04".into()).unwrap(),
            IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)),
            24,
            Some("ovn".into())
        )?;
        ovn.add_lsp_router(
            "sw0-port3".into(),
            "sw0".into(),
            MacAddress::new("router".into()).unwrap(),
            "lr0-port0".into(),
        )?;
        // now add the DHCP rule
        ovn.add_dhcp_option(
            &"lr0".into(),
            &"sw0".into(),
            "10.0.0.1..10.0.0.10"
        )?;
        // check all components have the correct information, unwrap as we know they exist
        ovn.switches.get("sw0").unwrap();
        // there are two ports on this switch with dynamic, one without
        let switch_port_1 = ovn.switch_ports.get("sw0-port0").unwrap();
        let switch_port_2 = ovn.switch_ports.get("sw0-port1").unwrap(); // this one does not have dynamic
        let switch_port_3 = ovn.switch_ports.get("sw0-port2").unwrap();
        let dhcp_option = ovn.dhcp_options.iter().next().unwrap(); // there is only one
        println!("{:?}", dhcp_option);
        // get hash of dhcp option
        let mut s = DefaultHasher::new();
        dhcp_option.hash(&mut s);
        let dhcp_hash = s.finish();
        // now check the correct ports have this option
        assert_eq!(switch_port_1.dhcp_options_uuid.unwrap(), dhcp_hash);
        assert!(switch_port_2.dhcp_options_uuid.is_none());
        assert_eq!(switch_port_3.dhcp_options_uuid.unwrap(), dhcp_hash);
        // manually create the DHCP option, hash it and test again to see if the hash works to be
        // extra sure - this did actually catch a but in the server_id having a mask as well
        let dhcp = DhcpDatabaseEntry {
            cidr: OvnIpAddr::Subnet {
                ip: IpAddr::V4(Ipv4Addr::new(10, 0, 0, 0)),
                mask: 24,
            },
            lease_time: "3600".to_string(),
            router: "10.0.0.1".into(),
            server_id: "10.0.0.1".into(),
            server_mac: MacAddress::new("00:00:00:00:00:04".into()).unwrap(),
        };
        let mut s = DefaultHasher::new();
        dhcp.hash(&mut s);
        let dhcp_hash = s.finish();
        assert_eq!(switch_port_1.dhcp_options_uuid.unwrap(), dhcp_hash);
        assert_eq!(switch_port_3.dhcp_options_uuid.unwrap(), dhcp_hash);

        // add a bad rule and test it doesnt work
        let res = ovn.add_dhcp_option(
            &"lr1".into(), // this router doesnt exist
            &"sw0".into(),
            "10.0.0.1..10.0.0.10"
        );
        assert!(res.is_err());
        let res = ovn.add_dhcp_option(
            &"lr0".into(),
            &"sw1".into(), // this switch doesnt exist
            "10.0.0.1..10.0.0.10",
        );
        assert!(res.is_err());

        Ok(())
    }

    #[test]
    fn test_lsp_pair() -> anyhow::Result<(), LogicalOperationResult> {
        let mut ovn = OvnNetwork::new();
        // do base case
        ovn.add_switch(
            "sw0".into(),
            IpAddr::V4(Ipv4Addr::new(10, 0, 0, 0)),
            24)?;
        ovn.add_router("lr0".into())?;
        ovn.add_lrp(
            "lr0-port0".into(),
            "lr0".into(),
            MacAddress::new("00:00:00:00:00:01".into()).unwrap(),
            IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)),
            24,
            Some("ovn".into())
        )?;
        ovn.add_lsp_router(
            "sw0-port0".into(),
            "sw0".into(),
            MacAddress::new("00:00:00:00:00:01".into()).unwrap(),
            "lr0-port0".into(),
        )?;
        let pair = ovn.get_lsp_lrp_pair(
            &"sw0".into(),
            &"lr0".into(),
        )?;
        assert_eq!(pair.0.name, "sw0-port0".to_string());
        assert_eq!(pair.1.name, "lr0-port0".to_string());
        // now add a second switch to the same router
        ovn.add_switch(
            "sw1".into(),
            IpAddr::V4(Ipv4Addr::new(20, 0, 0, 0)),
            24)?;
        ovn.add_lrp(
            "lr0-port1".into(),
            "lr0".into(),
            MacAddress::new("00:00:00:00:00:02".into()).unwrap(),
            IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)),
            24,
            Some("ovn".into())
        )?;
        ovn.add_lsp_router(
            "sw1-port0".into(),
            "sw1".into(),
            MacAddress::new("00:00:00:00:00:02".into()).unwrap(),
            "lr0-port1".into(),
        )?;
        // test the previous case again and then the new case
        let pair = ovn.get_lsp_lrp_pair(
            &"sw0".into(),
            &"lr0".into(),
        )?;
        assert_eq!(pair.0.name, "sw0-port0".to_string());
        assert_eq!(pair.1.name, "lr0-port0".to_string());
        let pair = ovn.get_lsp_lrp_pair(
            &"sw1".into(),
            &"lr0".into(),
        )?;
        assert_eq!(pair.0.name, "sw1-port0".to_string());
        assert_eq!(pair.1.name, "lr0-port1".to_string());
        // add a second router but dont link
        ovn.add_router("lr1".into())?;
        // test there is no pair
        let pair = ovn.get_lsp_lrp_pair(
            &"sw0".into(),
            &"lr1".into(),
        );
        assert!(pair.is_err());
        // test once again
        let pair = ovn.get_lsp_lrp_pair(
            &"sw0".into(),
            &"lr0".into(),
        )?;
        assert_eq!(pair.0.name, "sw0-port0".to_string());
        assert_eq!(pair.1.name, "lr0-port0".to_string());
        let pair = ovn.get_lsp_lrp_pair(
            &"sw1".into(),
            &"lr0".into(),
        )?;
        assert_eq!(pair.0.name, "sw1-port0".to_string());
        assert_eq!(pair.1.name, "lr0-port1".to_string());

        Ok(())
    }

    #[test]
    fn test_acl_success() -> anyhow::Result<(), LogicalOperationResult> {
        let mut ovn = OvnNetwork::new();
        let acl_name = "ovn-sw0-to-lport-drop-10".to_string();
        let rule = ACLRule {
            direction: ACLDirection::ToLport,
            priority: 10,
            _match: "".to_string(),
            action: ACLAction::AllowRelated,
        };
        ovn.add_switch_acl(&acl_name, "sw0".to_string(), ACLRecordType::Switch, &rule)?;
        Ok(())
    }

    #[test]
    fn test_acl_fail() -> anyhow::Result<(), LogicalOperationResult> {
        let mut ovn = OvnNetwork::new();
        let acl_name = "ovn-sw0-to-lport-drop-10".to_string();
        let rule = ACLRule {
            direction: ACLDirection::ToLport,
            priority: 10,
            _match: "".to_string(),
            action: ACLAction::AllowRelated,
        };
        ovn.add_switch_acl(&acl_name, "sw0".to_string(), ACLRecordType::Switch, &rule)?;
        let res = ovn.add_switch_acl(&acl_name, "sw0".to_string(), ACLRecordType::Switch, &rule);
        assert_eq!(res, Err(LogicalOperationResult::AlreadyExists { name: "ovn-sw0-to-lport-drop-10".to_string() }));
        Ok(())
    }
}
