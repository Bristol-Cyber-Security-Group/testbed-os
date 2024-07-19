use crate::components::{LogicalGuests};
use std::collections::{HashMap};
use std::net::{IpAddr};
use anyhow::{bail, Context};
use kvm_compose_schemas::kvm_compose_yaml::{Machine, MachineNetwork};
use kvm_compose_schemas::kvm_compose_yaml::machines::GuestType;
use kvm_compose_schemas::kvm_compose_yaml::network::{OvnNetworkSchema, OvsNetwork};
use kvm_compose_schemas::kvm_compose_yaml::network::router::{NatType, RouterPort};
use kvm_compose_schemas::kvm_compose_yaml::network::switch::{SwitchPort, SwitchPortType};
use kvm_compose_schemas::settings::SshConfig;
use crate::components::logical_load_balancing::LoadBalanceTopology;
use crate::ovn::components::{MacAddress, OvnIpAddr};
use crate::ovn::components::acl::ACLRecordType;
use crate::ovn::configuration::nat::OvnNatType;
use crate::ovn::ovn::OvnNetwork;

pub enum LogicalNetwork {
    Ovn(OvnNetwork),
    Ovs(LogicalOvsNetwork),
}

impl LogicalNetwork {
    pub fn new_ovn(
        ovn_network_schema: &OvnNetworkSchema,
        load_balance_topology: &LoadBalanceTopology,
        tb_config: &HashMap<String, SshConfig>,
        guest_list: &LogicalGuests,
        project_name: &String,
    ) -> anyhow::Result<LogicalNetwork> {
        Ok(LogicalNetwork::Ovn(parse_ovn_schema(ovn_network_schema, load_balance_topology, tb_config, guest_list, project_name)?))
    }
    pub fn new_ovs(_ovs_network: &OvsNetwork) -> LogicalNetwork {
        unimplemented!();
    }
}

pub struct LogicalOvsNetwork {

}

/// Take the ovn schema from the yaml file and infer all the OVN components as the yaml schema is a
/// minimal cut down representation of what OVN can do
fn parse_ovn_schema(
    ovn_network_schema: &OvnNetworkSchema,
    load_balance_topology: &LoadBalanceTopology,
    tb_config: &HashMap<String, SshConfig>,
    guest_list: &LogicalGuests,
    project_name: &String,
) -> anyhow::Result<OvnNetwork> {
    tracing::info!("begin defining internal OVN representation");
    let mut ovn = OvnNetwork::new();
    // add switches and it's ports
    if let Some(switches) = &ovn_network_schema.switches {
        for (switch_name, switch_data) in switches {
            // add switch
            let ip_and_mask = subnet_to_ip_and_mask(&switch_data.subnet)?;
            let switch_name = format!("{}-{}", project_name, switch_name);
            tracing::info!("defining logical switch {}", &switch_name);
            ovn.add_switch(
                switch_name.clone(),
                ip_and_mask.0,
                ip_and_mask.1,
            )?;
            // add any ports on the switch, there are a few types
            if let Some(ports) = &switch_data.ports {
                for port in ports {
                    add_switch_port(&mut ovn, port, &switch_name, tb_config, project_name)?;
                }
            }
        }
    }
    // add routers and its ports, and related router ports on switches, and router config
    if let Some(routers) = &ovn_network_schema.routers {
        for (router_name, router_data) in routers {
            // add router
            let router_name = format!("{}-{}", project_name, router_name);
            tracing::info!("defining logical router {}", &router_name);
            ovn.add_router(router_name.clone())?;
            // // add ports on the router and the corresponding switch port
            if let Some(ports) = &router_data.ports {
                for port in ports {
                    add_router_port(&mut ovn, port, &router_name, project_name)?;
                }
            }
            // add static routes
            if let Some(static_routes) = &router_data.static_routes {
                for route in static_routes {
                    let ip_and_mask = subnet_to_ip_and_mask(&route.prefix)?;
                    let next_hop = route.nexthop.parse::<IpAddr>().context(format!("getting nexthop for static route {:?}", route))?;
                    tracing::info!("defining static route (prefix: {}/{}, nexthop: {}) on router {}", &ip_and_mask.0, &ip_and_mask.1, &next_hop, router_name);
                    ovn.lr_route_add(
                       router_name.clone(),
                       OvnIpAddr::Subnet {
                           ip: ip_and_mask.0,
                           mask: ip_and_mask.1,
                       },
                       next_hop,
                    )?;
                }
            }
            // add nat rules
            if let Some(nat_rules) = &router_data.nat {
                for nat in nat_rules {
                    // this can be a ip+mask or just ip
                    let ip_and_mask = subnet_to_ip_and_mask(&nat.logical_ip);
                    // convert nat type, TODO - ovn schema is not in kvm-compose-schema.. should it?
                    let nat_type = match &nat.nat_type {
                        NatType::DnatAndSnat => OvnNatType::DnatSNat,
                        NatType::Snat => OvnNatType::SNat,
                    };
                    if ip_and_mask.is_ok() {
                        // if user input an ip with a mask
                        let res = ip_and_mask?;
                        let external_ip = OvnIpAddr::Ip(nat.external_ip.parse::<IpAddr>()
                            .context(format!("getting nat rule external ip {:?}", nat))?);
                        tracing::info!("defining NAT rule (nat type: {:?}, external ip: {}, logical ip: {}/{}) on router {}",
                            &nat_type, &external_ip.to_string(), &res.0, &res.1, &router_name);
                        ovn.lr_add_nat(
                            router_name.clone(),
                            nat_type,
                            external_ip,
                            OvnIpAddr::Subnet {
                                ip: res.0,
                                mask: res.1,
                            },
                        )?;
                    } else {
                        // if user input an ip without mask
                        let external_ip = OvnIpAddr::Ip(nat.external_ip.parse::<IpAddr>()
                            .context(format!("getting nat rule external ip {:?}", nat))?);
                        let logical_ip = OvnIpAddr::Ip(nat.logical_ip.parse::<IpAddr>()
                            .context(format!("getting nat rule logical ip {:?}", nat))?);
                        tracing::info!("defining NAT rule (nat type: {:?}, external ip: {}, logical ip: {}) on router {}",
                            &nat_type, &external_ip.to_string(), &logical_ip.to_string(), &router_name);
                        ovn.lr_add_nat(
                            router_name.clone(),
                            nat_type,
                            external_ip,
                            logical_ip,
                        )?;
                    }
                }
            }
        }
    }
    // add ports for guests
    for guest in guest_list.iter() {
        let guest_config = guest.get_machine_definition();
        // skip any guests that are backing images for scaling
        match &guest_config.guest_type {
            GuestType::Libvirt(libvirt) => {
                if libvirt.scaling.is_some() {
                    continue;
                }
            }
            GuestType::Docker(docker) => {
                if docker.scaling.is_some() {
                    continue;
                }
            }
            GuestType::Android(android) => {
                if android.scaling.is_some() {
                    continue;
                }
            }
        }
        let net = &guest_config.network.as_ref()
            .context("getting guest network while creating ovn network internal representation")?;

        for (idx, interface) in net.iter().enumerate() {
            add_guest_switch_port(
                idx,
                &mut ovn,
                &guest_config,
                interface,
                load_balance_topology,
                tb_config,
                project_name,
            )?;
        }
    }
    // add_guest_switch_port

    // we add dhcp rules at the end as we need to make sure the router, switch and switch ports
    // exist before we can cross check all the information
    if let Some(routers) = &ovn_network_schema.routers {
        for (router_name, router_data) in routers {
            // add dhcp options
            if let Some(dhcp_list) = &router_data.dhcp {
                for dhcp in dhcp_list {
                    // create DHCP Options database entry, configures the switch and the switch
                    // port(s) with this rule
                    let router_name = format!("{}-{}", project_name, router_name);
                    let switch_name = format!("{}-{}", project_name, &dhcp.switch);
                    ovn.add_dhcp_option(
                        &router_name,
                        &switch_name,
                        &format!("{}..{}", dhcp.exclude_ips.from, dhcp.exclude_ips.to),
                    )?;
                }
            }
        }
    }

    // add ACL
    if let Some(acl) = &ovn_network_schema.acl {
        if acl.apply_deny_all {
            // TODO
            bail!("apply_deny_all not yet implemented")
        }

        // for each rule on a switch, make sure the switch exists
        for (switch, acl_rules) in &acl.switches {
            let switch_name = format!("{}-{}", &project_name, &switch);
            if ovn.switches.contains_key(&switch_name) {
                for rule in acl_rules {
                    let acl_name = format!("{}-{}-{}-{}-{}", &project_name, &switch, &rule.direction, &rule.action, &rule.priority);
                    ovn.add_switch_acl(&acl_name, switch_name.clone(), ACLRecordType::Switch, rule)?;
                }
            } else {
                bail!("switch '{switch}' in ACL ruleset was not defined in the main network topology");
            }
        }

        // if we implement ACL for port groups, create them here and use ACLRecordType::PortGroup
    }

    tracing::info!("validating the OVN internal representation");
    ovn.validate().context("applying constraints to OVN internal representation")?;
    // TODO check if the network names and chassis match up to the kvm-compose-config

    Ok(ovn)
}

pub fn subnet_to_ip_and_mask(string: &String) -> anyhow::Result<(IpAddr, u16)> {
    let split: Vec<_> = string.split('/').collect();
    if split.len() != 2 {
        bail!("format of subnet {string} is not 'ip/mask'");
    }
    // now we have an ip v4/v6 and the mask
    let mask: u16 = split[1].parse::<u16>()?;
    let ip_res = split[0].parse::<IpAddr>()?;
    Ok((ip_res, mask))
}

/// There are different kinds of switch ports to add, depending on the data given. This function
/// adds switch ports that are defined in the network section of the yaml, that are not tied to a
/// guest.
fn add_switch_port(
    ovn: &mut OvnNetwork,
    port: &SwitchPort,
    parent_switch: &String,
    // load_balance_topology: &LoadBalanceTopology,
    tb_config: &HashMap<String, SshConfig>,
    project_name: &String,
) -> anyhow::Result<()> {
    let port_name = format!("{}-{}", project_name, &port.name);
    tracing::info!("defining logical switch port {}", &port_name);
    match &port.port_type {
        SwitchPortType::Internal { ip, mac, network_name } => {
            // internal switch port can be bound to a chassis, check if chassis exists
            let chassis = {
                if port.chassis.is_some() {
                    let chassis = port.chassis.clone().unwrap();
                    tb_config.get(&chassis)
                        .context(format!("checking if host config {} exists when adding a switch port", chassis))?;
                    Some(chassis)
                } else {
                    None
                }
            };

            // need to take the string IP and convert into a testbed ovn ip
            let ip = ip
                .clone()
                .context("getting IP for internal logical port")?;
            let ip = ip_string_to_ovn_ip(&ip, &port_name)?;
            // port name
            ovn.add_lsp_internal(
                port_name.clone(),
                parent_switch.clone(),
                port_name.clone(),
                ip.clone(),
                chassis.clone(),
                MacAddress::new(mac.clone().context(format!("getting mac address for internal logical port {}", &port.name))?)?,
                network_name.clone(),
            )?;

            // TODO - we may not want an OVS port to be created, the user may want to bind something
            //  else, we could add OVS port definitions in the yaml. the user may want to bind a
            //  network interface to another network for example
            // // create ovs port if a chassis was defined
            // if chassis.is_some() {
            //     ovn.ovs_add_port(
            //         port_name.clone(), // reuse logical port name
            //         host_config.ovn.bridge.clone(),
            //         port_name.clone(),
            //         host_config.ovn.chassis_name.clone()
            //     )?;
            // }
        }
        SwitchPortType::Router { .. } => {
            bail!("defining router ports on the switch is not supported, they are automatically added when creating the corresponding router port");
        }
        SwitchPortType::Localnet { network_name } => {
            ovn.add_lsp_localnet(
                port_name.clone(),
                parent_switch.clone(),
                network_name.clone(),
            )?;
        }
    }
    Ok(())
}

/// This function adds switch ports for each guest defined in the yaml file. The port name will be
/// derived from the project name, switch name and guest name to make it unique.
fn add_guest_switch_port(
    idx: usize,
    ovn: &mut OvnNetwork,
    guest_config: &Machine,
    interface_definition: &MachineNetwork,
    load_balance_topology: &LoadBalanceTopology,
    tb_config: &HashMap<String, SshConfig>,
    project_name: &String,
) -> anyhow::Result<()> {
    // create a composite name that is unique to this guest and project
    let port_name = format!("{}-{}-{}-{}", project_name, interface_definition.switch, guest_config.name, idx);
    tracing::info!("defining guest switch port {}", &port_name);
    let ip = ip_string_to_ovn_ip(&interface_definition.ip, &port_name)?;
    let host = load_balance_topology.guest_to_host.get(&guest_config.name)
        .context(format!("getting host for guest {} to assign chassis to switch port", &guest_config.name))?;
    let host_config = tb_config.get(host)
        .context(format!("getting host config {}", host))?;
    let parent_switch = format!("{}-{}", project_name, interface_definition.switch);
    // guest ports are always internal
    ovn.add_lsp_internal(
        port_name.clone(),
        parent_switch,
        // the OVS port name on the integration bridge is the same as logical port name
        port_name.clone(),
        ip,
        Some(host_config.ovn.chassis_name.clone()),
        MacAddress::new(interface_definition.mac.clone())?,
        interface_definition.network_name.clone(),
    )?;
    Ok(())
}

fn add_router_port(
    ovn: &mut OvnNetwork,
    port: &RouterPort,
    parent_router: &String,
    project_name: &String,
) -> anyhow::Result<()> {
    // TODO - prevent switch and router being linked twice
    let port_name = format!("{}-{}", project_name, &port.name);
    tracing::info!("defining logical router port {}", &port_name);
    // add router port
    let ip_and_mask = subnet_to_ip_and_mask(&port.gateway_ip)?;
    ovn.add_lrp(
        port_name.clone(),
        parent_router.clone(),
       MacAddress::new(port.mac.clone())?,
       ip_and_mask.0,
        ip_and_mask.1,
        port.set_gateway_chassis.clone(),
    )?;
    // if gateway chassis is set
    if let Some(chassis) = &port.set_gateway_chassis {
        ovn.lrp_add_external_gateway(
            parent_router.clone(),
            port_name.clone(),
            chassis.clone(),
        )?;
    }
    // add corresponding switch port
    ovn.add_lsp_router(
        format!("rp-{}", &port_name),
        format!("{}-{}", project_name, &port.switch),
        MacAddress::new("router".into())?,
        port_name.clone(),
    )?;
    Ok(())
}

// /// Each guest needs to be assigned the mac address of the port they are assigned to in OVN.
// pub fn assign_guests_mac(
//     logical_guests: &mut LogicalGuests,
//     ovn: &LogicalNetwork,
//     project_name: &String,
// ) -> anyhow::Result<()> {
//     for guest in logical_guests {
//         // add project name prefix to interface, at this point the OVN components have been given
//         // the project name already
//         let interface = format!("{}-{}", project_name, guest.get_network_interface());
//         match ovn {
//             LogicalNetwork::Ovn(ovn) => {
//                 let port = ovn.switch_ports.get(&interface)
//                     .context("getting logical switch port that matches guest's interface to assign mac")?;
//                 match &port.port_type {
//                     LogicalSwitchPortType::Internal { mac_address, .. } => {
//                         guest.set_mac_address(mac_address.clone());
//                     }
//                     _ => unreachable!(),
//                 };
//
//             }
//             LogicalNetwork::Ovs(_) => unimplemented!(),
//         }
//     }
//
//     Ok(())
// }

// /// Each guest needs to be assigned the ip address of the port they are assigned to in OVN. The port
// /// can be set to dynamic if using DHCP.
// pub fn assign_and_validate_guest_ip(
//     logical_guests: &mut LogicalGuests,
//     ovn: &LogicalNetwork,
//     project_name: &String,
// ) -> anyhow::Result<()> {
//     for guest in logical_guests {
//         match guest.get_machine_definition().guest_type {
//             GuestType::Libvirt(libvirt) => if libvirt.scaling.is_some() {continue;},
//             GuestType::Docker(docker) => if docker.scaling.is_some() {continue;},
//             GuestType::Android(android) => if android.scaling.is_some() {continue;},
//         }
//         match ovn {
//             LogicalNetwork::Ovn(ovn) => {
//                 // add project name prefix to interface, at this point the OVN components have been given
//                 // the project name already
//                 let interface = format!(
//                     "{}-{}-{}",
//                     project_name,
//                     guest.get_network().context("getting guest network when validating guest ip")?,
//                     guest.get_guest_name(),
//                 );
//                 let port = ovn.switch_ports.get(&interface)
//                     .context("getting logical switch port for guest to assign ip")?;
//                 match &port.port_type {
//                     LogicalSwitchPortType::Internal { ip, .. } => {
//                         guest.set_static_ip(ip.to_string());
//                     }
//                     _ => unreachable!(),
//                 }
//             }
//             LogicalNetwork::Ovs(_) => unimplemented!(),
//         }
//     }
//     Ok(())
// }

// pub fn assign_guest_gateway(
//     logical_guests: &mut LogicalGuests,
//     ovn: &LogicalNetwork,
//     project_name: &String,
// ) -> anyhow::Result<()> {
//     for guest in logical_guests {
//         match ovn {
//             LogicalNetwork::Ovn(ovn) => {
//                 // add project name prefix to interface, at this point the OVN components have been given
//                 // the project name already
//                 let interface = format!("{}-{}", project_name, guest.get_interface());
//                 let port = ovn.switch_ports.get(&interface)
//                     .context("getting logical switch port that matches guest's interface")?;
//
//                 // TODO - get the router port, gateway, for that switch - or do we need to set this
//                 //  in the yaml? how would DHCP do it? just get the ip of the router port on the
//                 //  same subnet mask
//
//                 match &port.port_type {
//                     LogicalSwitchPortType::Internal { ip, .. } => {
//                         guest.set_static_ip(ip.to_string());
//                     }
//                     _ => unreachable!(),
//                 }
//             }
//             LogicalNetwork::Ovs(_) => unimplemented!(),
//         }
//     }
//     Ok(())
// }

fn ip_string_to_ovn_ip(
    ip: &String,
    port_name: &String,
) -> anyhow::Result<OvnIpAddr> {
    match ip.as_str() {
        "dynamic" => Ok(OvnIpAddr::Dynamic),
        string => {
            // could be an ip or subnet
            match string.parse::<IpAddr>() {
                Ok(ipaddr) => Ok(OvnIpAddr::Ip(ipaddr)),
                Err(err) => {
                    // is likely now a mask, now allowed
                    bail!("ip address set for LSP {port_name} was not in a correct format, err: {err:#}")
                }
            }
        }
    }
}