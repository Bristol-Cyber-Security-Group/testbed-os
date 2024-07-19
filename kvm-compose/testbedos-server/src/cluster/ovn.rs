use std::collections::HashMap;
use tokio::process::Command;
use std::sync::Arc;
use reqwest::StatusCode;
use serde::Deserialize;
use tokio::sync::RwLock;
use tokio_cron_scheduler::{Job, JobScheduler};
use kvm_compose_lib::components::network::subnet_to_ip_and_mask;
use kvm_compose_lib::orchestration::{run_subprocess_command, run_subprocess_command_allow_fail};
use kvm_compose_schemas::settings::{OvnConfig, SshConfig, TestbedClusterConfig};
use crate::config::provider::TestbedConfigProvider;


/// This sets up a cron job inside the server runtime to check the client testbeds are still
/// running. The host might be up but the testbed server in client mode might not be, so we check
/// the status endpoint for a 200 HTTP code.
pub async fn set_up_cluster_client_check_cron_jobs(

) -> anyhow::Result<()> {
    // set up cron job to monitor clients
    let sched = JobScheduler::new().await?;
    sched.add(
        Job::new_async("1/10 * * * * *", |_uuid, _lock| Box::pin( async move {
            //tracing::info!("checking client testbed hosts");
            let mut cluster_config = TestbedClusterConfig::read()
                .await
                .expect("scheduled job could not read TestbedClusterConfig");
            let mut remove_list = Vec::new();
            for (name, host) in cluster_config.testbed_host_ssh_config.iter() {
                let is_master = host.is_master_host.unwrap_or_default();
                // tracing::info!("checked {name} and is master = {is_master}");
                // if not master and not online then add to remove list
                if !is_master {
                    // tracing::info!("checking if cluster client {name} is still connected");

                    let res = reqwest::get(&format!("http://{}:3355/api/config/status", host.ip)).await;
                    match res {
                        Ok(ok) => {
                            match ok.status() {
                                StatusCode::OK => {}
                                _ => {
                                    // responded but no Ok, remove
                                    tracing::warn!("client {name} is not connected to the testbed anymore, removing from cluster");
                                    remove_list.push(name.clone());
                                }
                            }
                        }
                        Err(err) => {
                            // problem connecting to client, will remove as client might be down
                            tracing::error!("error when checking if {name} is up, will remove from cluster... got error :{err:#}");
                            remove_list.push(name.clone());
                        }
                    };

                }
            }
            // remove inactive TB hosts from
            for to_delete in &remove_list {
                cluster_config.testbed_host_ssh_config.remove(to_delete);
            }
            // only save if we did something
            if !remove_list.is_empty() {
                cluster_config.write()
                    .await
                    .expect("could not save cluster config when pruning active testbed client list");
            }
        }))?
    ).await?;
    sched.start().await?;
    Ok(())
}

/// This sets up a cron job inside the server runtime to check the master testbeds is still
/// running. The host might be up but the testbed server in master mode might not be, so we check
/// the status endpoint for a 200 HTTP code. If the master goes down, keep trying to re-connect
/// until it is up, then we must re-join the cluster as the master will start a fresh cluster config
/// when it starts, so it won't include this client testbed.
pub async fn set_up_cluster_master_check_cron_jobs(
    master_ip: String,
) -> anyhow::Result<()> {
    // set up cron job to monitor clients
    let sched = JobScheduler::new().await?;

    sched.add(Job::new_async("1/10 * * * * *",  move |_uuid, _l| {
        // need to make a clone in this scope so that we can push the master ip into the cron
        // closure, otherwise the compiler will whinge about reference being used after
        let master_ip = master_ip.clone();
        Box::pin(async move {
            let client_config = SshConfig::read()
                .await
                .expect("could not read client config for re-join cluster check");
            // ask for the cluster config
            let cmd_res = Command::new("sudo")
                .arg("curl")
                .arg("-s")
                .arg("-w")
                .arg("'%{http_code}'")
                .arg(&format!("{}:3355/api/cluster/{}", master_ip, &client_config.ovn.chassis_name))
                .output()
                .await;
            match cmd_res {
                Ok(ok_resp) => {
                    // server responded, check if part of cluster
                    let response_code = String::from_utf8(ok_resp.stdout)
                        .expect("getting status code from master");

                    if !response_code.eq(&"'200'".to_string()) {
                        // not part of cluster, make join request
                        let server_url = format!("http://{master_ip}:3355/api/cluster");
                        tracing::warn!("not part of master testbed's cluster, will try to rejoin at {}", &server_url);

                        // convert to json
                        let client_config = serde_json::to_string_pretty(&client_config)
                            .expect("converting client config to json to send to master");
                        Command::new("sudo")
                            .arg("curl")
                            .arg("-X")
                            .arg("POST")
                            .arg(&server_url)
                            .arg("-H")
                            .arg("Content-Type: application/json")
                            .arg("-d")
                            .arg(client_config)
                            .output()
                            .await
                            .expect("sending client config to master server to re-join cluster");
                        tracing::info!("join request accepted");

                    }
                }
                Err(_) => {
                    tracing::error!("master testbed server is not running, will try to reconnect");
                }
            }
        })
    })?).await?;

    sched.start().await?;
    Ok(())
}

/// This fn will configure OVN to make sure all the settings are as the testbed needs it.
/// Additionally, we must also make sure that the external bridge(s) have the correct IP address and
/// there are iptables rules set to forward traffic from the external bridges to the testbed host's
/// main interface.
pub async fn configure_host_ovn(
    ovn: &OvnConfig,
    main_interface: &String,
    is_master: bool,
    host_config: &SshConfig,
) -> anyhow::Result<()> {

    tracing::info!("making sure OVS is up");
    let chassis_name = format!("--system-id={}", host_config.ovn.chassis_name);
    run_subprocess_command_allow_fail(
        "sudo",
        vec!["/usr/local/share/openvswitch/scripts/ovs-ctl", "start", &chassis_name],
        false,
        None,
    ).await?;

    tracing::info!("making sure ovn controller is up");
    run_subprocess_command_allow_fail(
        "sudo",
        vec!["/usr/local/share/ovn/scripts/ovn-ctl", "start_controller"],
        false,
        None,
    ).await?;
    tracing::info!("making sure ovn northbound database is up");
    run_subprocess_command_allow_fail(
        "sudo",
        vec!["/usr/local/share/ovn/scripts/ovn-ctl", "start_northd"],
        false,
        None,
    ).await?;

    // TODO - if an external bridge is removed from config, it should be destroyed (delta change)
    // make sure the external bridge(s) exist and have an ip address and up and have NAT rule
    for (_, ext, ip) in ovn.bridge_mappings.iter() {
        // add external bridge
        tracing::info!("making sure external bridge {ext} exists");
        run_subprocess_command(
            "sudo",
            vec!["ovs-vsctl", "--may-exist", "add-br", ext],
            false,
            None,
        ).await?;
        // if input has no mask, we need to add a default
        let ip_and_mask = match subnet_to_ip_and_mask(ip) {
            Ok(_) => ip.clone(), // has ip and mask
            Err(_) => format!("{ip}/24") // no mask, so default to /24
        };
        // add ip
        tracing::info!("making sure {ext} has ip {ip}");
        run_subprocess_command_allow_fail(
            "sudo",
            vec!["ip", "addr", "add", &ip_and_mask, "dev", ext],
            false,
            None,
        ).await?;
        // make sure its up
        tracing::info!("making sure {ext} is up");
        run_subprocess_command_allow_fail(
            "sudo",
            vec!["ip", "link", "set", ext, "up"],
            false,
            None,
        ).await?;
        // only do this if master
        if is_master {
            // check if NAT rule exists
            tracing::info!("checking if NAT rule for {ext} {ip} to forward to {main_interface} exists");
            let cmd = Command::new("sudo")
                .args(vec!["iptables", "-t", "nat", "-C", "POSTROUTING", "-o", main_interface, "-s", &infer_subnet(ip)?, "-j", "MASQUERADE"])
                .output()
                .await;
            let exists = match cmd {
                Ok(res) => {
                    String::from_utf8(res.stderr)?
                }
                Err(err) => {
                    // assume rule is not there
                    tracing::warn!("could not test if NAT rule exists, adding anyway. err: {err:#}");
                    String::new()
                }
            };
            if exists.contains("No chain/target/match by that name") || exists.contains("does a matching rule exist in that chain?") {
                // add NAT rule
                tracing::info!("adding NAT rule for {ext} {ip}");
                // TODO - do we need to change the octets to 0 depending on mask?
                run_subprocess_command_allow_fail(
                    "sudo",
                    vec!["iptables", "-t", "nat", "-A", "POSTROUTING", "-o", main_interface, "-s", &infer_subnet(ip)?, "-j", "MASQUERADE"],
                    false,
                    None,
                ).await?;
                // add forward rules
                tracing::info!("adding forward rules for {ext} {ip}");
                run_subprocess_command_allow_fail(
                    "sudo",
                    vec!["iptables", "-A", "FORWARD", "-i", main_interface, "-o", ext, "-m", "state", "--state", "RELATED,ESTABLISHED", "-j", "ACCEPT"],
                    false,
                    None,
                ).await?;
                run_subprocess_command_allow_fail(
                    "sudo",
                    vec!["iptables", "-A", "FORWARD", "-i", ext, "-o", main_interface, "-j", "ACCEPT"],
                    false,
                    None,
                ).await?;
            } else {
                tracing::info!("NAT rule for {ext} {ip} already exists, skipping");
            }
        }
    }

    // TODO make sure OVN settings are correct (chassis) - only on master

    // make sure OVS settings are correct (external ids) for either master or client, based on host config
    set_ovs_external_ids(ovn).await?;

    // TODO ask remote testbeds if their local settings are correct? or is this in their join script


    Ok(())
}


/// We must compare the `testbed-cluster-config` to the OVN chassis information
pub async fn configure_ovn_cluster(
    db_config: &Arc<RwLock<Box<dyn TestbedConfigProvider + Sync + Send>>>,
) -> anyhow::Result<()> {
    tracing::info!("comparing cluster config to OVN southbound database");
    // get config
    let cluster_config = db_config.read().await.get_cluster_config().await?;
    // check which hosts exist in kvm-compose-config
    let chassis_list = get_chassis_list().await?;
    // compare chassis in OVN to kvm-compose-config
    let mut unknown_chassis = vec![];
    for (host_name, _) in chassis_list {
        if cluster_config.testbed_host_ssh_config.get(&host_name).is_none() {
            // entry in southbound database does not exist in kvm-compose config, remove
            tracing::warn!("found chassis {} in OVN southbound database that is not in cluster config, adding to remove list", &host_name);
            unknown_chassis.push(host_name);
        }
    }
    // remote any in the remove list
    for chassis in unknown_chassis {
        tracing::info!("removing chassis {} from OVN southbound database", &chassis);
        run_subprocess_command(
            "sudo",
            vec!["ovn-sbctl", "chassis-del", &chassis],
            false,
            None,
        ).await?;
    }

    Ok(())
}

/// Return a list of chassis registered in OVN southbound database. The data is requested in csv
/// format and parsed with serde.
async fn get_chassis_list(

) -> anyhow::Result<HashMap<String, OvnChassisCsvRecord>> {
    // get chassis list as a csv
    let chassis_csv_raw = run_subprocess_command(
        "sudo",
        vec!["ovn-sbctl", "-f", "csv", "list", "chassis"],
        false,
        None,
    ).await?;
    let mut ovn_chassis = HashMap::new();
    let mut chassis_csv = csv::Reader::from_reader(chassis_csv_raw.as_bytes());
    for row in chassis_csv.deserialize() {
        let record: OvnChassisCsvRecord = row?;
        ovn_chassis.insert(record.name.clone(), record);
    }
    Ok(ovn_chassis)
}

#[derive(Debug, Deserialize)]
struct OvnChassisCsvRecord {
    name: String,
    // // could be useful to differentiate between two clients with same chassis name to report error
    // // back to the user
    // hostname: String,
    // // its in a weird hashmap format, will need to do some further processing if we need to inspect
    // other_config: String,
}

async fn set_ovs_external_ids(
    ovn: &OvnConfig,
) -> anyhow::Result<()> {
    tracing::info!("setting OVS external ids");

    let encap_type = format!("external-ids:ovn-encap-type={}", &ovn.encap_type);
    tracing::info!("setting {}", &encap_type);
    run_subprocess_command_allow_fail(
        "sudo",
        vec!["ovs-vsctl", "set", "open", ".", &encap_type],
        false,
        None,
    ).await?;

    let encap_ip = format!("external-ids:ovn-encap-ip={}", &ovn.encap_ip);
    tracing::info!("setting {}", &encap_ip);
    run_subprocess_command_allow_fail(
        "sudo",
        vec!["ovs-vsctl", "set", "open", ".", &encap_ip],
        false,
        None,
    ).await?;

    let remote = format!("external-ids:ovn-remote={}", &ovn.master_ovn_remote);
    tracing::info!("setting {}", &remote);
    run_subprocess_command_allow_fail(
        "sudo",
        vec!["ovs-vsctl", "set", "open", ".", &remote],
        false,
        None,
    ).await?;

    let bridge = format!("external-ids:ovn-bridge={}", &ovn.bridge);
    tracing::info!("setting {}", &bridge);
    run_subprocess_command_allow_fail(
        "sudo",
        vec!["ovs-vsctl", "set", "open", ".", &bridge],
        false,
        None,
    ).await?;
    // work out the list of bridge mappings
    let bridge_mappings = {
        let mut map = Vec::new();
        for (network, bridge, _) in ovn.bridge_mappings.iter() {
            map.push(format!("{}:{}", network, bridge));
        }
        
        map.join(",")
    };
    let mapping = format!("external-ids:ovn-bridge-mappings={}", &bridge_mappings);
    tracing::info!("setting {}", &mapping);
    run_subprocess_command_allow_fail(
        "sudo",
        vec!["ovs-vsctl", "set", "open", ".", &mapping],
        false,
        None,
    ).await?;

    Ok(())
}

/// Given an ip with or without a mask, determine the subnet and mask. If no mask given, default to
/// the /24 range
pub fn infer_subnet(
    ip: &String,
) -> anyhow::Result<String> {
    match subnet_to_ip_and_mask(ip) {
        Ok((ip, mask)) => {
            // nothing to be done
            let subnet = format!("{ip}/{mask}");
            Ok(subnet)
        }
        Err(_) => {
            // no mask, default to just the last octet and add /24
            Ok(format!("{ip}/24"))
        }
    }
}
