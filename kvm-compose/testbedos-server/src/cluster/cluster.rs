use std::collections::HashMap;
use std::sync::Arc;
use anyhow::Context;
use reqwest::Client;
use tokio::sync::RwLock;
use kvm_compose_lib::orchestration::run_subprocess_command_allow_fail;
use kvm_compose_schemas::settings::{SshConfig, TestbedClusterConfig};
use crate::cluster::ovn::{configure_host_ovn, configure_ovn_cluster, infer_subnet};
use crate::cluster::{ClusterOperation, ServerModeCmd};
use crate::config::provider::TestbedConfigProvider;

/// This function will run on startup of the server, if the server is a master then it will check
/// if there is a kvm-compose-config, if not the master then it will try to join the cluster at the
/// specified master ip - triggering the master to update the kvm-compose-config
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
    let is_master = match host_config.is_master_host {
        None => false,
        Some(master) => master,
    };

    // configure any OVN related settings and make sure ovn and ovs are up, before other services
    configure_host_ovn(&host_config.ovn, &host_config.main_interface, is_master, &host_config).await?;

    // make sure libvirt, docker are up
    ensure_services_up().await?;

    match mode {
        ServerModeCmd::Client(client) => {
            // request to join cluster
            let client_ovn_remote = client_join_cluster(&host_config, &client.master_ip).await?;
            // update local ovn remote to point to master
            host_config.ovn.client_ovn_remote = Some(client_ovn_remote.clone());
            let _ = &db_config.write()
                .await
                .set_host_config(host_config.clone())
                .await?;
            // update openvswitch
            let remote = format!("external-ids:ovn-remote={}", &client_ovn_remote);
            tracing::info!("updating ovn remote to point to master host: {}", &remote);
            run_subprocess_command_allow_fail(
                "sudo",
                vec!["ovs-vsctl", "set", "open", ".", &remote],
                false,
                None,
            ).await?;
        }
        ServerModeCmd::Master => {
            manage_cluster(&ClusterOperation::Init, db_config).await?;
        }
        ServerModeCmd::CreateConfig => {}
    }

    // we may have edited the configs or filled in any default values, so write any changes
    db_config.write().await.set_host_config(host_config).await?;

    Ok(())
}

/// The master testbed server will maintain the `TestbedClusterConfig`, at any time there needs to
/// be an update or the server starts, it needs to check the validity of the config
pub async fn manage_cluster(
    cluster_operation: &ClusterOperation,
    db_config: &Arc<RwLock<Box<dyn TestbedConfigProvider + Sync + Send>>>,
) -> anyhow::Result<()> {
    tracing::info!("running checks on TestbedClusterConfig");

    let cluster_config = db_config
        .read()
        .await
        .get_cluster_config()
        .await;
    let mut cluster_config = match cluster_config {
        Ok(ok) => {
            tracing::info!("TestbedClusterConfig found, continuing");
            ok
        }
        Err(_) => {
            tracing::info!("TestbedClusterConfig not found, creating one");
            // does not exist, create one
            let mut host_config = HashMap::new();
            host_config.insert("master".into(), db_config.read().await.get_host_config().await?);
            let mut new_config = TestbedClusterConfig {
                testbed_host_ssh_config: host_config,
                ssh_public_key_location: "".to_string(),
                ssh_private_key_location: "".to_string(),
            };
            TestbedClusterConfig::insert_default_values(&mut new_config);
            // save new config to disk
            db_config.write().await.set_cluster_config(new_config.clone()).await?;
            new_config
        }
    };

    match cluster_operation {
        ClusterOperation::Init => {
            tracing::info!("Running cluster Init");
            // make sure a fresh cluster config is used, we don't want previous state here as we can't
            // guarantee between the master turning on and off the cluster is the same
            tracing::debug!("getting cluster config");
            // TODO - make sure we insert the master config if it is not there for some reason
            let mut cluster_config = db_config.read().await.get_cluster_config().await?;
            tracing::debug!("getting host config");
            let host_config = db_config.read().await.get_host_config().await?;
            // filter for the master (this host)
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
            // make sure it is not master host so the client doesnt have to edit this
            client_config_copy.is_master_host = Some(false);

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

/// This function will ask the master testbed server to join the cluster, which will update the
/// master's kvm-compose-config.
pub async fn client_join_cluster(
    client_config: &SshConfig,
    master_ip: &String
) -> anyhow::Result<String> {
    let server_url = format!("http://{master_ip}:3355/api/cluster");
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

/// This function will check each member of the cluster (that is not the master) and check if they
/// are still up. If they are not up, then the kvm-compose-config will be updated.
pub async fn check_cluster_clients(

) {
    // TODO - ask the client for it's SshConfig again? check if something changed?
    todo!()
}

/// Removes a cluster client from the kvm-compose-config on the master testbed.
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