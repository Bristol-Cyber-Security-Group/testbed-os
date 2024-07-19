use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr};
use anyhow::{bail, Context};
use kvm_compose_schemas::kvm_compose_yaml::machines::libvirt::{
    ConfigLibvirtMachine, LibvirtGuestOptions,
};
use kvm_compose_schemas::kvm_compose_yaml::machines::{ConfigScalingInterface, ConfigScalingIpType, ConfigScalingMacRange, GuestType};
use kvm_compose_schemas::kvm_compose_yaml::{Config, Machine, MachineNetwork};
use std::path::PathBuf;
use kvm_compose_schemas::kvm_compose_yaml::machines::avd::{AVDGuestOptions, ConfigAVDMachine};
use kvm_compose_schemas::kvm_compose_yaml::machines::docker::ConfigDockerMachine;
use crate::ovn::components::{MacAddress};

pub fn generate_clone_guests(config: &mut Config) -> anyhow::Result<()> {
    // for any guests with a scaling parameter, append to the machine list to be treated as a normal
    // guest for artefact generation.
    // these new guests must be given the correct interface from the list and be given the correct
    // setup and run scripts from the scaling group, all as if they were an independently defined machine

    tracing::info!("parsing yaml for any scaling definitions");

    let mut new_config_machines = Vec::new();

    if config.machines.is_some() {
        for machine in config
            .machines
            .clone()
            .context("Getting machines from yaml config")?
            .iter()
        {
            // clone setup only work for libvirt based guests
            match &machine.guest_type {
                GuestType::Libvirt(libvirt_guest) => {
                    // for each machine definition, if scaling is missing then move on
                    if libvirt_guest.scaling.is_none() {
                        continue;
                    }

                    // match &libvirt_guest.libvirt_type {
                    //     LibvirtGuestOptions::CloudImage { .. } => {}
                    //     // TODO implement the needed changes here if the guest type is existing disk
                    //     LibvirtGuestOptions::ExistingDisk { .. } => unimplemented!(),
                    //     LibvirtGuestOptions::IsoGuest { .. } => unimplemented!(),
                    // }

                    // scaling is not missing, begin expanding this machine
                    let machine_scaling_config = libvirt_guest
                        .scaling
                        .as_ref()
                        .context("Getting libvirt scaling parameter from yaml config")?;

                    // we have a scaling parameter, create a new ConfigMachine for the scaling count
                    for clone_n in 0..machine_scaling_config.count {
                        // get this clone's interfaces
                        let clone_interfaces = get_clone_interface(clone_n, &machine_scaling_config.interfaces)?;

                        // get this clone's setup script
                        // TODO - make sure the validation of yaml ensures the clone is assigned a setup script once
                        let clone_setup_script = if machine_scaling_config.clone_setup.is_some() {
                            let mut clone_setup_script = PathBuf::new();
                            for setup in machine_scaling_config
                                .clone_setup
                                .as_ref()
                                .context("Getting clone setup script data")?
                                .iter()
                            {
                                if setup.clones.contains(&clone_n) {
                                    // setup script present for clone
                                    clone_setup_script.clone_from(&setup.script)
                                }
                            }
                            Some(clone_setup_script)
                        } else {
                            None
                        };
                        // get this clone's run script
                        // TODO - make sure the validation of yaml ensures the clone is assigned a run script once
                        let clone_run_script = if machine_scaling_config.clone_run.is_some() {
                            let mut clone_run_script = PathBuf::new();
                            for run in machine_scaling_config
                                .clone_run
                                .as_ref()
                                .context("Getting clone run script data")?
                                .iter()
                            {
                                if run.clones.contains(&clone_n) {
                                    // setup script present for clone
                                    clone_run_script.clone_from(&run.script)
                                }
                            }
                            Some(clone_run_script)
                        } else {
                            None
                        };
                        // now build a ConfigMachine for this clone
                        // TODO - when the machine types are not libvirt, this needs to be updated to assign
                        //  the correct "disk"
                        // let clone_config_machine = ConfigMachine {
                        //     name: format!("{}-{}", machine.name.clone(), clone_n),
                        //     interfaces: clone_interfaces,
                        //     memory_mb: machine.memory_mb.clone(),
                        //     cpus: machine.cpus.clone(),
                        //     disk: ConfigDisk::Clone {
                        //         path: Default::default(),  // this is filled in with the real path in artefact gen
                        //         backing_disk: Default::default(), // this is filled in later in enumerate_disk_paths
                        //         driver_type: DiskDriverType::QCow2,
                        //         device_type: DiskDeviceType::Disk,
                        //         readonly: false,
                        //     },
                        //     username: machine.username.clone(),
                        //     extended_graphics_support: machine.extended_graphics_support.clone(),
                        //     run_script: clone_run_script,
                        //     setup_script: clone_setup_script,
                        //     context: machine.context.clone(),
                        //     environment: machine.environment.clone(),
                        //     scaling: None,
                        //     is_clone_of: Some(machine.name.clone()),
                        //     tcp_tty_port: None,
                        //     machine_type: MachineType::LibvirtClone,
                        // };
                        let clone_config_machine = Machine {
                            name: format!("{}-{}", machine.name.clone(), clone_n),
                            network: Some(vec![clone_interfaces]),
                            guest_type: GuestType::Libvirt(ConfigLibvirtMachine {
                                memory_mb: libvirt_guest.memory_mb,
                                cpus: libvirt_guest.cpus,
                                libvirt_type: match &libvirt_guest.libvirt_type {
                                    LibvirtGuestOptions::CloudImage {
                                        name,
                                        expand_gigabytes,
                                        path,
                                        run_script: _,
                                        setup_script: _,
                                        context,
                                        environment,
                                    } => LibvirtGuestOptions::CloudImage {
                                        name: name.clone(),
                                        expand_gigabytes: *expand_gigabytes,
                                        path: path.clone(),
                                        run_script: clone_run_script,
                                        setup_script: clone_setup_script,
                                        context: context.clone(),
                                        environment: environment.clone(),
                                    },
                                    LibvirtGuestOptions::ExistingDisk {
                                        path,
                                        driver_type,
                                        device_type,
                                        readonly, 
                                        create_deep_copy,
                                    } => LibvirtGuestOptions::ExistingDisk {
                                        path: path.clone(),
                                        driver_type: driver_type.clone(),
                                        device_type: device_type.clone(),
                                        readonly: *readonly,
                                        create_deep_copy: *create_deep_copy,
                                    },
                                    LibvirtGuestOptions::IsoGuest { .. } => unimplemented!(),
                                },
                                username: libvirt_guest.username.clone(),
                                password: libvirt_guest.password.clone(),
                                hostname: libvirt_guest.hostname.clone(),
                                ssh_address: libvirt_guest.ssh_address.clone(),
                                scaling: None,
                                is_clone_of: Some(machine.name.clone()),
                                tcp_tty_port: None,
                                static_ip: None,
                            }),
                        };

                        tracing::info!("adding a libvirt clone {}", &clone_config_machine.name);

                        // add clone into intermediate list
                        new_config_machines.push(clone_config_machine);
                    }
                }

                GuestType::Docker(docker_guest) => {
                    // skip non scaling guests
                    if docker_guest.scaling.is_none() {
                        continue;
                    }

                    // scaling is not missing, begin expanding this machine
                    let machine_scaling_config = docker_guest
                        .scaling
                        .as_ref()
                        .context("Getting docker scaling parameter from yaml config")?;

                    for clone_n in 0..docker_guest.scaling.as_ref().context("getting docker scaling config")?.count {
                        // get this clone's interfaces
                        let clone_interfaces = get_clone_interface(clone_n, &machine_scaling_config.interfaces)?;

                        let clone_config_machine = Machine {
                            name: format!("{}-{}", machine.name.clone(), clone_n),
                            network: Some(vec![clone_interfaces]),
                            guest_type: GuestType::Docker(ConfigDockerMachine {
                                image: docker_guest.image.clone(),
                                command: docker_guest.command.clone(),
                                entrypoint: docker_guest.entrypoint.clone(),
                                environment: docker_guest.environment.clone(),
                                env_file: docker_guest.env_file.clone(),
                                volumes: docker_guest.volumes.clone(),
                                privileged: docker_guest.privileged,
                                scaling: None,
                                user: docker_guest.user.clone(),
                                device: docker_guest.device.clone(),
                                hostname: docker_guest.hostname.clone(),
                                static_ip: None,
                            }),
                        };

                        tracing::info!("adding a docker clone {}", &clone_config_machine.name);

                        // add clone into intermediate list
                        new_config_machines.push(clone_config_machine);
                    }
                }
                GuestType::Android(avd_guest) => {
                    // skip non scaling guests
                    if avd_guest.scaling.is_none() {
                        continue;
                    }

                    // scaling is not missing, begin expanding this machine
                    let machine_scaling_config = avd_guest
                        .scaling
                        .as_ref()
                        .context("Getting android scaling parameter from yaml config")?;

                    for clone_n in 0..avd_guest.scaling.as_ref().context("getting avd scaling config")?.count {
                        // get this clone's interfaces
                        let clone_interfaces = get_clone_interface(clone_n, &machine_scaling_config.interfaces)?;

                        let clone_config_machine = Machine {
                            name: format!("{}-{}", machine.name.clone(), clone_n),
                            network: Some(vec![clone_interfaces]),
                            guest_type: GuestType::Android(ConfigAVDMachine {
                                static_ip: None,
                                avd_type: match &avd_guest.avd_type {
                                    AVDGuestOptions::Avd {
                                        android_api_version,
                                        playstore_enabled
                                    } => AVDGuestOptions::Avd {
                                        android_api_version: *android_api_version,
                                        playstore_enabled: *playstore_enabled,
                                    },
                                    AVDGuestOptions::ExistingAvd { .. } => {
                                        // need to create copies
                                        unimplemented!()
                                    }
                                },
                                scaling: None,
                            }),
                        };

                        tracing::info!("adding an android clone {}", &clone_config_machine.name);

                        // add clone into intermediate list
                        new_config_machines.push(clone_config_machine);
                    }

                }
            }
        }
    }
    // merge clone list into main machine list
    if config.machines.is_some() {
        tracing::info!("merging any clones into the guests definition");
        let new_config = config
            .machines
            .clone()
            .context("Getting machines from yaml config")?
            .into_iter()
            .chain(new_config_machines)
            .collect::<Vec<Machine>>();
        config.machines = Some(new_config);
    }
    Ok(())
}

/// The hashmap of `ConfigScalingInterface` will contain a list of clone ids, where one should
/// contain the id of the clone. At this point, the yaml should have been validated to make sure
/// the clone ID only appears once in the hashmap. With the interface for the guest (the switch in
/// ovn), we can then work out how to derive a `MachineNetwork` based on the position of the clone
/// id in the list i.e. [0,1,2] and clone id = 1, meaning it is position 1. Therefore we can take
/// the ip and mac in their ranges and get the ip and mac in position 1. The ranges will also be in
/// the same range length of the clones id list, so for ips of range from: 10.0.0.1 to: 10.0.0.3
/// the clone with id will get ip 10.0.0.2
fn get_clone_interface(clone_n: u32, clone_interfaces: &HashMap<String, ConfigScalingInterface>) -> anyhow::Result<MachineNetwork> {
    for (interface_name, config_scaling_interface) in clone_interfaces.iter() {
        if config_scaling_interface.clones.contains(&clone_n) {

            // create MachineNetwork using get_clone_ip_from_range and get_mac_from_range
            let clone_ip = get_clone_ip_from_range(
                clone_n,
                &config_scaling_interface.clones,
                &config_scaling_interface.ip_type,
            )?;

            let clone_mac = get_clone_mac_from_range(
                clone_n,
                &config_scaling_interface.clones,
                &config_scaling_interface.mac_range,
            )?;

            return Ok(MachineNetwork {
                switch: interface_name.to_string(),
                gateway: config_scaling_interface.gateway.clone(),
                mac: clone_mac.address,
                ip: clone_ip.to_string(),
                network_name: None,
            });

        }
    }
    bail!("could not find interface for clone");
}

/// Return the `IpAddr` for the clone based on its clone n, which dictates the ip selected in the
/// range specified.
fn get_clone_ip_from_range(
    clone_n: u32,
    clone_list: &[u32],
    ip_type: &ConfigScalingIpType,
) -> anyhow::Result<String> {
    match ip_type {
        ConfigScalingIpType::IpRange(ip_range) => {
            // make sure ip values are formatted properly
            let from = ip_range.from.parse::<IpAddr>()
                .context("parsing ConfigScalingIpRange (from) into ip address".to_string())?;
            let to = ip_range.to.parse::<IpAddr>()
                .context("parsing ConfigScalingIpRange (to) into ip address".to_string())?;
            // make sure both ips are same type
            if from.is_ipv4() != to.is_ipv4() || from.is_ipv6() != to.is_ipv6() {
                bail!("ip types need to match either both ipv4 or ipv6, from = {} and to = {}", from.to_string(), to.to_string())
            }
            // get range of possible ips, the range can be 0 if the from and to are the same, meaning there
            // is only one ip to choose from
            let range = get_ip_range(from, to)?;

            // we need to get the position of the clone in the list
            let clone_pos = clone_list.iter().position(|&p| p == clone_n)
                .context("getting clone position in clone list")? as u32;
            // make sure the possible ip range suits the number of clones
            let clone_list_len = clone_list.len() as u32;
            if clone_list_len != range + 1 {
                bail!("the number of clones for the interface {clone_list_len} did not match the ip range given {}", range +1);
            }
            // create ip from range
            let mut ip: u32 = match from {
                IpAddr::V4(v4) => v4.into(),
                IpAddr::V6(_) => unimplemented!(),
            };
            // make sure clone number is within the range
            if clone_pos > range {
                bail!("clone id's {} position {} is greater than the number of ips in the range {}", &clone_n, &clone_pos, range + 1);
            }

            ip += clone_pos;
            let ip: IpAddr = match from {
                IpAddr::V4(_) => IpAddr::V4(Ipv4Addr::from(ip)),
                IpAddr::V6(_) => unimplemented!(),
            };

            Ok(ip.to_string())
        }
        ConfigScalingIpType::Dynamic => {
            Ok("dynamic".to_string())
        }
    }

}

/// Return the `MacAddress` for the clone based on it's clone n, which dictates the mac selected in
/// the range specified.
fn get_clone_mac_from_range(
    clone_n: u32,
    clone_list: &[u32],
    mac_range: &ConfigScalingMacRange,
) -> anyhow::Result<MacAddress> {
    let from = MacAddress::new(mac_range.from.clone())?;
    let to = MacAddress::new(mac_range.to.clone())?;
    let range = get_mac_range(from.clone(), to)?;

    let clone_list_len = clone_list.len();

    // we need to get the position of the clone in the list
    let clone_pos = clone_list.iter().position(|&p| p == clone_n)
        .context("getting clone position in clone list")?;

    // make sure the possible mac range suits the number of clones
    if clone_list_len as u64 != range + 1 {
        bail!("the number of clones for the interface {clone_list_len} did not match the ip range given {}", range +1);
    }

    // make sure clone position is within the range
    if clone_pos as u64 > range {
        bail!("clone id's {} position {} is greater than the number of macs in the range {}", &clone_n, &clone_pos, range + 1);
    }

    // add clone n to the from mac u64 to get the clone mac
    let from_bytes = &from.as_bytes
        .context("getting mac as bytes")?;
    

    MacAddress::from_u64(from_bytes + clone_pos as u64)
}

/// Since we are working with IP ranges, we want work out how many consecutive ips in this range.
/// Return the number of IPs in the range (inclusive). Make sure that the "from" range is less than
/// or equal to the "to" range.
fn get_ip_range(
    from: IpAddr,
    to: IpAddr,
) -> anyhow::Result<u32> {
    // get range of ip, depends on type, assume we have checked both are same type, we can just work
    // from one
    
    match from {
        IpAddr::V4(from_v4) => {
            // let from_octets = v4.octets();
            let to_v4 = match to {
                IpAddr::V4(v4) => v4,
                _ => unreachable!()
            };

            let from_ip_bytes: u32 = from_v4.into();
            let to_ip_bytes: u32 = to_v4.into();
            if to_ip_bytes < from_ip_bytes {
                bail!("the ip range \"to\" is less than the \"from\" range, from: {}, to: {}", from_v4, to_v4)
            }
            Ok(to_ip_bytes - from_ip_bytes)
        }
        IpAddr::V6(_) => unimplemented!(),
    }
}

/// Since we are working with mac ranges, we want to work out how many consecutive mac addresses in
/// this mac range. Returns the number of macs in this range (inclusive). Make sure that the "from"
/// range is less than or equal to the "to" range.
fn get_mac_range(
    from: MacAddress,
    to: MacAddress,
) -> anyhow::Result<u64> {
    let from_bytes = from.as_bytes.context(format!("getting mac {} as bytes", &from.address))?;
    let to_bytes = to.as_bytes.context(format!("getting mac {} as bytes", &to.address))?;

    if to_bytes < from_bytes {
        bail!("the mac range \"to\" is less than the \"from\" range, from: {}, to: {}", to_bytes, from_bytes);
    }

    Ok(to_bytes - from_bytes)
}


#[cfg(test)]
mod tests {
    use std::net::Ipv4Addr;
    use kvm_compose_schemas::kvm_compose_yaml::machines::ConfigScalingIpRange;
    use super::*;

    #[test]
    fn test_get_ip_range_v4() {
        let from = IpAddr::V4(Ipv4Addr::new(0,0,0,1));
        let to = IpAddr::V4(Ipv4Addr::new(0,0,0,3));
        let range = get_ip_range(from, to);
        assert!(range.is_ok());
        assert_eq!(range.unwrap(), 2);

        let from = IpAddr::V4(Ipv4Addr::new(0,0,0,1));
        let to = IpAddr::V4(Ipv4Addr::new(0,0,0,1));
        let range = get_ip_range(from, to);
        assert!(range.is_ok());
        assert_eq!(range.unwrap(), 0);

        let from = IpAddr::V4(Ipv4Addr::new(0,0,0,3));
        let to = IpAddr::V4(Ipv4Addr::new(0,0,0,1));
        let range = get_ip_range(from, to);
        assert!(range.is_err());

    }

    #[test]
    fn test_get_mac_range() {
        let from = MacAddress::new("00:00:00:00:00:03".into()).unwrap();
        let to = MacAddress::new("00:00:00:00:00:05".into()).unwrap();
        let range = get_mac_range(from, to);
        assert!(range.is_ok());
        assert_eq!(range.unwrap(), 2);

        let from = MacAddress::new("00:00:00:00:00:03".into()).unwrap();
        let to = MacAddress::new("00:00:00:00:00:03".into()).unwrap();
        let range = get_mac_range(from, to);
        assert!(range.is_ok());
        assert_eq!(range.unwrap(), 0);

        let from = MacAddress::new("00:00:00:00:00:05".into()).unwrap();
        let to = MacAddress::new("00:00:00:00:00:03".into()).unwrap();
        let range = get_mac_range(from, to);
        assert!(range.is_err());
    }

    #[test]
    fn test_get_clone_ip_from_range_v4() {
        let clone_n = 0;
        let ip_addr_range = ConfigScalingIpRange {
            from: "10.0.0.1".to_string(),
            to: "10.0.0.3".to_string()
        };
        let ip = get_clone_ip_from_range(
            clone_n,
            &[0,1,2],
            &ConfigScalingIpType::IpRange(ip_addr_range),
        );
        assert!(ip.is_ok());
        assert_eq!(ip.unwrap().to_string(), "10.0.0.1".to_string());

        let clone_n = 2;
        let ip_addr_range = ConfigScalingIpRange {
            from: "10.0.0.1".to_string(),
            to: "10.0.0.3".to_string()
        };
        let ip = get_clone_ip_from_range(
            clone_n,
            &[0,1,2],
            &ConfigScalingIpType::IpRange(ip_addr_range),
        );
        assert!(ip.is_ok());
        assert_eq!(ip.unwrap().to_string(), "10.0.0.3".to_string());

        let clone_n = 0;
        let ip_addr_range = ConfigScalingIpRange {
            from: "10.0.0.1".to_string(),
            to: "10.0.0.1".to_string()
        };
        let ip = get_clone_ip_from_range(
            clone_n,
            &[0],
            &ConfigScalingIpType::IpRange(ip_addr_range),
        );
        assert!(ip.is_ok());
        assert_eq!(ip.unwrap().to_string(), "10.0.0.1".to_string());
    }

    #[test]
    fn test_get_clone_ip_from_range_v4_bad_clone_n() {
        // clone n is greater than the possible ip range
        let clone_n = 3;
        let ip_addr_range = ConfigScalingIpRange {
            from: "10.0.0.1".to_string(),
            to: "10.0.0.3".to_string()
        };
        let ip = get_clone_ip_from_range(
            clone_n,
            &[0,1,2],
            &ConfigScalingIpType::IpRange(ip_addr_range),
        );
        assert!(ip.is_err());
    }

    #[test]
    fn test_get_clone_ip_from_range_v4_bad_clone_list_n() {
        // clone list n is more than the ip addr range
        let clone_n = 0;
        let ip_addr_range = ConfigScalingIpRange {
            from: "10.0.0.1".to_string(),
            to: "10.0.0.2".to_string()
        };
        let ip = get_clone_ip_from_range(
            clone_n,
            &[0,1,2],
            &ConfigScalingIpType::IpRange(ip_addr_range),
        );
        assert!(ip.is_err());
    }

    #[test]
    fn test_get_clone_ip_from_range_v4_bad_addr_range() {
        // clone list n is more than the ip addr range
        let clone_n = 0;
        let ip_addr_range = ConfigScalingIpRange {
            from: "10.0.0.2".to_string(),
            to: "10.0.0.1".to_string()
        };
        let ip = get_clone_ip_from_range(
            clone_n,
            &[0,1,2],
            &ConfigScalingIpType::IpRange(ip_addr_range),
        );
        assert!(ip.is_err());
    }

    #[test]
    fn test_get_clone_ip_from_range_where_clones_distributed_across_switches() {
        // if we have 4 clones and want 2 to be on one switch and the other 2 on another
        // lets take the second switch
        let clone_n = 2;
        let ip_addr_range = ConfigScalingIpRange {
            from: "10.0.0.3".to_string(),
            to: "10.0.0.4".to_string()
        };
        let ip = get_clone_ip_from_range(
            clone_n,
            &[2,3],
            &ConfigScalingIpType::IpRange(ip_addr_range),
        );
        assert!(ip.is_ok());
        assert_eq!(ip.unwrap().to_string(), "10.0.0.3".to_string());
        // second guest
        let clone_n = 3;
        let ip_addr_range = ConfigScalingIpRange {
            from: "10.0.0.3".to_string(),
            to: "10.0.0.4".to_string()
        };
        let ip = get_clone_ip_from_range(
            clone_n,
            &[2,3],
            &ConfigScalingIpType::IpRange(ip_addr_range),
        );
        assert!(ip.is_ok());
        assert_eq!(ip.unwrap().to_string(), "10.0.0.4".to_string());
    }

    #[test]
    fn test_get_clone_mac_from_range() {
        // this test simulates as if all clones are being put on the same switch
        let clone_n = 0;
        let mac_addr_range = ConfigScalingMacRange {
            from: "00:00:00:00:00:01".to_string(),
            to: "00:00:00:00:00:03".to_string(),
        };
        let mac = get_clone_mac_from_range(
            clone_n,
            &[0,1,2],
            &mac_addr_range,
        );
        assert!(mac.is_ok());
        assert_eq!(mac.unwrap().address, "00:00:00:00:00:01".to_string());

    }

    #[test]
    fn test_get_clone_mac_from_range_where_clones_distributed_across_switches() {
        // this is the more complicated case, if we have say 4 clones and we want 2 to be on one
        // switch, but the third clone to be on another switch - this means the clone list for this
        // second switch will only be 1

        let clone_n = 2; // third clone
        let mac_addr_range = ConfigScalingMacRange {
            from: "00:00:00:00:00:03".to_string(), // only one mac address to pick from
            to: "00:00:00:00:00:03".to_string(),
        };
        let mac = get_clone_mac_from_range(
            clone_n,
            &[2],
            &mac_addr_range,
        );
        assert!(mac.is_ok());
        assert_eq!(mac.unwrap().address, "00:00:00:00:00:03".to_string());

        // now do a test with 4 clones split between 2 switches
        // sw0 = clones 0,3
        // sw1 = clones 2,1
        let clone_n = 2; // third clone
        let mac_addr_range = ConfigScalingMacRange {
            from: "00:00:00:00:00:03".to_string(), // only one mac address to pick from
            to: "00:00:00:00:00:04".to_string(),
        };
        let mac = get_clone_mac_from_range(
            clone_n,
            &[2, 1],
            &mac_addr_range,
        );
        assert!(mac.is_ok());
        assert_eq!(mac.unwrap().address, "00:00:00:00:00:03".to_string());
        // do second clone
        let clone_n = 1; // fourth clone
        let mac_addr_range = ConfigScalingMacRange {
            from: "00:00:00:00:00:03".to_string(), // only one mac address to pick from
            to: "00:00:00:00:00:04".to_string(),
        };
        let mac = get_clone_mac_from_range(
            clone_n,
            &[2, 1],
            &mac_addr_range,
        );
        assert!(mac.is_ok());
        assert_eq!(mac.unwrap().address, "00:00:00:00:00:04".to_string());

    }

}
