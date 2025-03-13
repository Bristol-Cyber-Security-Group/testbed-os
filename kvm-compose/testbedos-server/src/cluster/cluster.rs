use std::collections::HashMap;
use std::sync::Arc;
use anyhow::{bail, Context};
use reqwest::Client;
use tokio::sync::RwLock;
use kvm_compose_lib::orchestration::run_subprocess_command_allow_fail;
use kvm_compose_schemas::settings::{SshConfig, TestbedClusterConfig};
use crate::cluster::ovn::{configure_host_ovn, configure_ovn_cluster, infer_subnet};
use crate::cluster::{ClusterOperation, ServerModeCmd};
use crate::config::provider::TestbedConfigProvider;

/// This function will run on startup of the server, if the server is a main then it will check
/// if there is a kvm-compose-config, if not the main then it will try to join the cluster at the
/// specified main ip - triggering the main to update the kvm-compose-config
pub async fn configure_testbed_host(
    mode: &ServerModeCmd,
    db_config: &Arc<RwLock<Box<dyn TestbedConfigProvider + Sync + Send>>>,
) -> anyhow::Result<()> {
    match mode {
        ServerModeCmd::CreateConfig => {
            // do nothing, just continue
            unreachable!()
        }
        _ => {}
    }
    tracing::info!("configuring host to ensure environment is ready for the testbed");

    // host config host.json must exist
    let mut host_config = db_config.read().await.get_host_config()
        .await
        .context("reading host's configuration host.json")?;
    // set up interface, if the interface is not up then continue but cannot run in cluster mode
    let ip = infer_subnet(&host_config.ip)?;
    run_subprocess_command_allow_fail(
        "sudo",
        vec!["ip", "addr", "add", ip.as_str(), "dev", host_config.testbed_nic.as_str()],
        false,
        None,
    ).await?;
    // make sure it is up
    run_subprocess_command_allow_fail(
        "sudo",
        vec!["ip", "link", "set", host_config.testbed_nic.as_str(), "up"],
        false,
        None,
    ).await?;

    // make sure OVN settings are correct for this host
    let is_main = match host_config.is_main_host {
        None => false,
        Some(main) => main,
    };

    // get the main interface, if there is none in the config, then use the interface with default
    // route
    let main_interface = if host_config.main_interface.is_some() {
        host_config.main_interface.as_ref().unwrap()
    } else {
        host_config.main_interface = Some(get_default_interface().await?);
        host_config.main_interface.as_ref().unwrap()
    };

    // configure any OVN related settings and make sure ovn and ovs are up, before other services
    configure_host_ovn(&host_config.ovn, &main_interface, is_main, &host_config).await?;

    // make sure libvirt, docker are up
    ensure_services_up().await?;

    // if in client mode, make sure ssh server is running for main testbed to be able to control
    match mode {
        ServerModeCmd::Client(_) => {
            tracing::info!("making sure sshd is up");
            run_subprocess_command_allow_fail(
                "sudo",
                vec!["systemctl", "start", "sshd"],
                false,
                None,
            ).await?;
        }
        _ => {}
    }

    match mode {
        ServerModeCmd::Client(client) => {
            // request to join cluster
            let client_ovn_remote = client_join_cluster(&host_config, &client.main_ip).await?;
            // update local ovn remote to point to main
            host_config.ovn.client_ovn_remote = Some(client_ovn_remote.clone());
            let _ = &db_config.write()
                .await
                .set_host_config(host_config.clone())
                .await?;
            // update openvswitch
            let remote = format!("external-ids:ovn-remote={}", &client_ovn_remote);
            tracing::info!("updating ovn remote to point to main host: {}", &remote);
            run_subprocess_command_allow_fail(
                "sudo",
                vec!["ovs-vsctl", "set", "open", ".", &remote],
                false,
                None,
            ).await?;
        }
        ServerModeCmd::Main => {
            manage_cluster(&ClusterOperation::Init, db_config).await?;
        }
        ServerModeCmd::CreateConfig => {}
    }

    // we may have edited the configs or filled in any default values, so write any changes
    db_config.write().await.set_host_config(host_config).await?;

    Ok(())
}

/// The main testbed server will maintain the `TestbedClusterConfig`, at any time there needs to
/// be an update or the server starts, it needs to check the validity of the config
pub async fn manage_cluster(
    cluster_operation: &ClusterOperation,
    db_config: &Arc<RwLock<Box<dyn TestbedConfigProvider + Sync + Send>>>,
) -> anyhow::Result<()> {
    tracing::info!("running checks on TestbedClusterConfig");
    
    // the cluster config should not be kept between starts of the testbed server, as the host.json
    // config might have been updated while offline so we need to update it - but also the chassis 
    // name could have changed meanwhile, so best to create a fresh one.
    // this does have implications on any guests or network components that have already been 
    // created and are already up, which will essentially be "lost", will need to manage the clean
    // up separately TODO once we have more detailed resource tracking in a database with accounts
    tracing::info!("re-building TestbedClusterConfig for this session");

    let mut kvm_compose_config = HashMap::new();
    let host_config = db_config
        .read()
        .await
        .get_host_config()
        .await?;
    kvm_compose_config.insert(host_config.ovn.chassis_name.clone(), host_config);
    let mut cluster_config = TestbedClusterConfig {
        testbed_host_ssh_config: kvm_compose_config,
        ssh_public_key_location: "".to_string(),
        ssh_private_key_location: "".to_string(),
    };
    TestbedClusterConfig::insert_default_values(&mut cluster_config);
    // save new config to disk
    db_config.write().await.set_cluster_config(cluster_config.clone()).await?;

    match cluster_operation {
        ClusterOperation::Init => {
            tracing::info!("Running cluster Init");
            // make sure a fresh cluster config is used, we don't want previous state here as we can't
            // guarantee between the main turning on and off the cluster is the same
            tracing::debug!("getting cluster config");
            // TODO - make sure we insert the main config if it is not there for some reason
            let mut cluster_config = db_config.read().await.get_cluster_config().await?;
            tracing::debug!("getting host config");
            let host_config = db_config.read().await.get_host_config().await?;
            // filter for the main (this host)
            cluster_config.testbed_host_ssh_config.retain(|name,_| {
                if name.eq(&host_config.ovn.chassis_name) {
                    true
                } else {
                    false
                }
            });
            tracing::debug!("writing host config");
            db_config.write().await.set_cluster_config(cluster_config).await?;
        }
        ClusterOperation::Join(client_config) => {
            tracing::info!("Running cluster Join");
            let client_name = client_config.ovn.chassis_name.clone();
            // this could mean a client could replace another client accidentally..
            // TODO - compare with the results from get_chassis_list using the hostname which should
            //  be unique to the host if the chassis name is the same as another client
            if cluster_config.testbed_host_ssh_config.get(&client_name).is_some() {
                tracing::info!("client {} already exists in cluster, updating info", client_name);
            } else {
                tracing::info!("adding client {} to cluster", &client_name);
            }
            // overwrite the client info to make sure it is a client
            let mut client_config_copy = client_config.clone();
            // make sure it is not main host so the client doesnt have to edit this
            client_config_copy.is_main_host = Some(false);

            // a client testbed wants to join the cluster
            cluster_config.testbed_host_ssh_config.insert(
                client_config_copy.ovn.chassis_name.clone(),
                client_config_copy.clone(),
            );
            db_config.write().await.set_cluster_config(cluster_config).await?;
            // make sure the chassis list in OVN match the cluster config
            configure_ovn_cluster(db_config).await?;
        }
        ClusterOperation::Leave(_) => {
            tracing::info!("Running cluster Leave");
            // a client testbed wants to leave the cluster
            // TODO
        }
    }

    Ok(())
}

/// This function will ask the main testbed server to join the cluster, which will update the
/// main's kvm-compose-config.
pub async fn client_join_cluster(
    client_config: &SshConfig,
    main_ip: &String
) -> anyhow::Result<String> {
    let server_url = format!("http://{main_ip}:3355/api/cluster");
    tracing::info!("joining testbed cluster at {}", &server_url);
    // TODO - make sure protocol and port are correct
    let http_client = Client::new();
    let response = http_client.post(server_url)
        .json(client_config)
        .send()
        .await?;
    let ovn_remote = response.text().await?;
    Ok(ovn_remote)
}

/// This function will check each member of the cluster (that is not the main) and check if they
/// are still up. If they are not up, then the kvm-compose-config will be updated.
pub async fn check_cluster_clients(

) {
    // TODO - ask the client for it's SshConfig again? check if something changed?
    todo!()
}

/// Removes a cluster client from the kvm-compose-config on the main testbed.
pub async fn remove_cluster_client(

) {
    todo!()
}

async fn ensure_services_up(

) -> anyhow::Result<()> {
    tracing::info!("making sure the testbed service dependencies are running");

    tracing::info!("making sure libvirt is up");
    run_subprocess_command_allow_fail(
        "sudo",
        vec!["systemctl", "start", "libvirtd"],
        false,
        None,
    ).await?;
    tracing::info!("making sure docker is up");
    run_subprocess_command_allow_fail(
        "sudo",
        vec!["systemctl", "start", "docker.service"],
        false,
        None,
    ).await?;

    Ok(())
}

/// Check using 'ip route' to get the default interface that is used for an internet connection
/// for the host.
async fn get_default_interface() -> anyhow::Result<String> {
    // get resulting string from command, will be something like
    // default via 10.150.16.250 dev eno8403 proto static metric 100
    let output = run_subprocess_command_allow_fail(
        "sudo",
        vec!["ip", "route", "show", "default"],
        false,
        None,
    ).await?;

    // check if the command retrieved the right sort of result
    let split_output = output.split(" ").collect::<Vec<&str>>();
    if split_output.len() > 5 && split_output[3].eq("dev") {
        return Ok(split_output[4].trim().to_string());
    }
    bail!("No default interface found")
}
