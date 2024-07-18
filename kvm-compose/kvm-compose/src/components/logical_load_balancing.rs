use std::collections::HashMap;
use anyhow::Context;
use crate::components::LogicalTestbed;
use kvm_compose_schemas::kvm_compose_yaml::network::NetworkBackend;
use kvm_compose_schemas::kvm_compose_yaml::testbed_options::LoadBalancing;


/// It is up to the algorithm to work out resource
/// limits that might influence the load balancing. For example, an implementation may
/// make requests to the various testbed hosts and work out how much disk space and memory is
/// available to deploy guests there.
/// However, this check is done when this code runs and the resource usage may change once the
/// orchestration is executed.
/// The algorithm must also be implemented for the different network backend.
pub fn load_balance(logical_testbed: &mut LogicalTestbed) -> anyhow::Result<LoadBalanceTopology> {
    Ok(match logical_testbed.common.config.testbed_options.clone().load_balancing {
        LoadBalancing::NaiveRoundRobin => naive_round_robin(logical_testbed)?,
        LoadBalancing::Balanced => balanced(logical_testbed)?,
    })
}

pub struct LoadBalanceTopology {
    pub guest_to_host: HashMap<String, String>,
    // pub interface_to_host: HashMap<String, String>,
}

/// Simply assign to each testbed host until we run out of resources to assign
fn naive_round_robin(logical_testbed: &mut LogicalTestbed) -> anyhow::Result<LoadBalanceTopology> {
    let mut guest_to_host = HashMap::new();
    // let mut interface_to_host = HashMap::new();
    match &logical_testbed.common.config.network {
        NetworkBackend::Ovn(_) => {
            // for OVN network, just assign guests to each testbed host until we run out of guests
            // to assign

            // create a cycling iterator on available testbed hosts
            let mut hosts = logical_testbed.common
                .kvm_compose_config
                .testbed_host_ssh_config
                .iter()
                .cycle();
            for guest in logical_testbed.logical_guests.iter_mut() {
                // get next available host, tuple (name, config)
                let next_host = hosts.next().context("getting next host for naive round robin")?;
                // map guest to host
                tracing::info!("assigning guest {} to host {}", guest.get_guest_name(), next_host.0);
                guest.set_testbed_host(next_host.0.to_string());
                guest_to_host.insert(guest.get_guest_name().clone(), next_host.0.to_string());
                // interface_to_host.insert(guest.get_interface(), next_host.0.to_string());
            }

        }
        NetworkBackend::Ovs(_) => {
            // for OVS network, need to assign each bridge to each host which will dictate where
            // each guest will be
            unimplemented!()
        }
    }
    return Ok(LoadBalanceTopology {
        guest_to_host,
        // interface_to_host,
    })
}

/// Assign guests to each testbed host to maximise free resources across the testbed cluster.
/// This only considers memory usage of the host for now.
fn balanced(logical_testbed: &mut LogicalTestbed) -> anyhow::Result<LoadBalanceTopology> {
    // first get the list of testbed hosts in the current cluster
    // TODO

    // find out how much memory is free on each host
    // TODO

    // work out how much memory each guest needs
    // TODO

    // we can find out directly for libvirt guests from the config
    // android lets assign 4GB, as this can change
    // docker can use an unbounded amount, we currently don't re-export the mem max for containers in our yaml

    // since, we don't know how much docker containers will use, we will assign them last and on
    // hosts with the most free memory
    // we will assign libvirt, then android

    // the guests will crash if out of memory, by assigning docker last it is likely the user has
    // oversubscribed their hosts and available memory anyway with their yaml definition

    // as we assign guests, decrease the free memory on that host
    // assign libvirt, then android
    // TODO - algorithm to optimally fill space (doesn't need to be too complicated)
    // TODO - write tests for this

    // depending on how many docker guests and free space, assign the guests in a ratio from the
    // memory of the host with the least free memory, so that we can spread out containers evenly
    // TODO - write algorithm with tests


    unimplemented!()
}
