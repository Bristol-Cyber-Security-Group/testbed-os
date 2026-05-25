use crate::AppState;
use anyhow::{anyhow, bail, Context};
use futures_util::future::join_all;
use futures_util::{stream, StreamExt, TryStreamExt};
use kvm_compose_lib::state::schema::StateNetwork;
use kvm_compose_lib::state::state_evaluation::{EvaluateState, StateComponentStatus};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;


pub enum DeploymentStatus {
    Up,
    /// Partially up, a mix of up and down, the inner HashMap offers information
    Partial {
        guest_info: HashMap<String, String>,
        network_info: HashMap<String, String>
    },
    /// Fully down, the inner HashMap offers information
    Down{
        guest_info: HashMap<String, String>,
        network_info: HashMap<String, String>
    },
    /// Running means there is a lock on the deployment
    Running,
}

pub async fn total_deployment_state(
    db_config: Arc<AppState>,
    project: String,
) -> anyhow::Result<DeploymentStatus> {

    // first check if project exists
    db_config.deployment_config_db
        .read()
        .await
        .get_deployment(project.clone())
        .await
        .context("Deployment does not exist")?;

    // if the project exists, check to make sure there is no run lock
    if db_config.is_deployment_locked(&project) {
        return Ok(DeploymentStatus::Running);
    }

    // if project exists, then lets get the state json
    let state_json = db_config.deployment_config_db
        .read()
        .await
        .get_state(project.clone())
        .await
        .context("Deployment exists but state json does not")?;

    // for all the known components in the state, we will check each of them to check what is up and
    // what is down
    let guest_states: Vec<_> = stream::iter(state_json.testbed_guests.0)
        .then(|(guest_name, _)| {
            // clone these before creating the async closure
            let db_config = db_config.clone();
            let project = project.clone();
            let guest_name = guest_name.clone();
            async move {

                let guest_exists = guest_state(
                    db_config,
                    project,
                    guest_name.clone(),
                ).await?;

                Ok::<(String, StateComponentStatus), anyhow::Error>((guest_name.clone(), guest_exists))
            }
        })
        .try_collect()
        .await?;

    let network_component_states: Vec<_> = match state_json.network {
        StateNetwork::Ovn(ovn) => {
            let mut all_network_states = vec![];

            all_network_states.extend(collect_component_status(
                ovn.switches,
                logical_switch_state,
                db_config.clone(),
                project.clone(),
            ).await?);
            all_network_states.extend(collect_component_status(
                ovn.switch_ports,
                logical_switch_port_state,
                db_config.clone(),
                project.clone(),
            ).await?);
            all_network_states.extend(collect_component_status(
                ovn.routers,
                logical_router_state,
                db_config.clone(),
                project.clone(),
            ).await?);
            all_network_states.extend(collect_component_status(
                ovn.router_ports,
                logical_router_port_state,
                db_config.clone(),
                project.clone(),
            ).await?);
            all_network_states.extend(collect_component_status(
                ovn.acl,
                logical_acl_state,
                db_config.clone(),
                project.clone(),
            ).await?);
            // TODO - dhcp
            // let logical_dhcp = collect_component_status(
            //     ovn.dhcp_options,
            //     logical_dhcp_state,
            //     db_config.clone(),
            //     project.clone(),
            // ).await?;
            all_network_states.extend(collect_component_status(
                ovn.ovs_ports,
                ovs_port_state,
                db_config.clone(),
                project.clone(),
            ).await?);

            all_network_states
        }
        StateNetwork::Ovs(_) => unimplemented!(),
    };

    // now with the full vectors of both guest and network states, we can do a quick check to see
    // which response to make

    let all_guests_up = if !guest_states.is_empty() && guest_states.iter().all(|(g_n, g_s)| *g_s == StateComponentStatus::Up) {
        true
    } else {
        false
    };
    let all_network_components_up = if !network_component_states.is_empty() && network_component_states.iter().all(|(nc_n, nc_s)| *nc_s == StateComponentStatus::Up) {
        true
    } else {
        false
    };
    // if both all up fully then early exit
    if all_guests_up && all_network_components_up {
        return Ok(DeploymentStatus::Up);
    }
    // record whether there are any components or guests up
    let some_guests_up = if !guest_states.is_empty() && guest_states.iter().any(|(g_n, g_s)| *g_s == StateComponentStatus::Up) {
        true
    } else {
        false
    };
    let some_network_components_up = if !network_component_states.is_empty() && network_component_states.iter().any(|(nc_n, nc_s)| *nc_s == StateComponentStatus::Up) {
        true
    } else {
        false
    };

    // one or both are not fully up, need to outline what is and isn't up, and return a full
    // breakdown to the user
    let mut guest_counts: HashMap<String, String> = HashMap::new();
    for (guest_name, status) in guest_states {
        let key = match status {
            StateComponentStatus::Up => "up".to_string(),
            StateComponentStatus::Down(reason) => format!("down:{}", reason),
            StateComponentStatus::DoesNotExist => "does_not_exist".to_string(),
        };
        let _ = guest_counts.entry(guest_name).or_insert(key).clone();
    }
    let mut network_counts: HashMap<String, String> = HashMap::new();
    for (nc_name, status) in network_component_states {
        let key = match status {
            StateComponentStatus::Up => "up".to_string(),
            StateComponentStatus::Down(reason) => format!("down:{}", reason),
            StateComponentStatus::DoesNotExist => "does_not_exist".to_string(),
        };
        let _ = network_counts.entry(nc_name).or_insert(key).clone();
    }

    if some_guests_up || some_network_components_up {
        // "partial"
        Ok(DeploymentStatus::Partial {
            guest_info: guest_counts,
            network_info: network_counts,
        })
    } else {
        // "down"
        Ok(DeploymentStatus::Down {
            guest_info: guest_counts,
            network_info: network_counts,
        })
    }
}

async fn collect_component_status<L, F>(
    hash_map: HashMap<String, L>,
    state_check_func: F,
    db_config: Arc<AppState>,
    project: String,
) -> anyhow::Result<Vec<(String, StateComponentStatus)>>
    where F: AsyncFn(Arc<AppState>, String, String) -> anyhow::Result<StateComponentStatus> + Send + Sync + 'static,
{
    // for all network components, get their dedicated status check function and create a bunch of
    // futures to check them all in one go
    let status_vec_futures: Vec<_> = hash_map.iter()
        .map(|(component_name, component)| async {
            let component_exists = state_check_func(
                db_config.clone(),
                project.clone(),
                component_name.clone(),
            ).await?;

            Ok::<(String, StateComponentStatus), anyhow::Error>((component_name.clone(), component_exists))
        }).collect();
    // consume all futures
    let status_vec = join_all(status_vec_futures).await;
    // it is a vec of results so need to consume the results, if any fail then bail
    let mut result = vec![];
    for status in status_vec {
        match status {
            Ok(ok) => result.push(ok),
            Err(err) => bail!("{:?}", err),
        }
    }
    Ok(result)
}

pub async fn guest_state(
    db_config: Arc<AppState>,
    project: String,
    component: String,
) -> anyhow::Result<StateComponentStatus> {
    // first check if project exists
    db_config.deployment_config_db
        .read()
        .await
        .get_deployment(project.clone())
        .await
        .context("Deployment does not exist")?;

    // if project exists, then lets get the state json
    let state_json = db_config.deployment_config_db
        .read()
        .await
        .get_state(project.clone())
        .await
        .context("Deployment exists but state json does not")?;

    let guest = state_json.testbed_guests.0
        .get(&component)
        .ok_or(anyhow!("Guest {} does not exist", component))?;

    // get the name of the host that holds this guest
    let guest_host = guest.testbed_host.clone().ok_or(anyhow!("Guest {} not assigned to testbed host", component))?;
    // get the cluster config with all host information
    let cluster_config = db_config.config_db
        .read()
        .await
        .get_cluster_config()
        .await?;
    // get the host's config
    let host_config = cluster_config.testbed_host_ssh_config
        .get(&guest_host)
        .ok_or(anyhow!("Guest's host {} does not exist", guest_host))?;
    // if this host config is "main" then we can poll from this as this is the "main" server as well
    // but if it is not main, then we need to relay the state request to the correct testbed server
    let guest_exists = if host_config.is_main_host.ok_or(anyhow!("Host {} has not been given an if main", guest_host))? {
        // on main
        guest.check_exists(project).await?
    } else {
        unimplemented!()
    };

    Ok(guest_exists)
}

pub async fn logical_switch_state(
    db_config: Arc<AppState>,
    project: String,
    component: String,
) -> anyhow::Result<StateComponentStatus> {
    // first check if project exists
    db_config.deployment_config_db
        .read()
        .await
        .get_deployment(project.clone())
        .await
        .context("Deployment does not exist")?;

    // if project exists, then lets get the state json
    let state_json = db_config.deployment_config_db
        .read()
        .await
        .get_state(project.clone())
        .await
        .context("Deployment exists but state json does not")?;

    let component_exists = match state_json.network {
        StateNetwork::Ovn(ovn) => {
            let ls = ovn.switches
                .get(&component)
                .ok_or(anyhow!("Logical Switch {} does not exist", component))?
                .check_exists(project).await?;
            ls
        }
        StateNetwork::Ovs(_) => {
            unimplemented!()
        },
    };

    Ok(component_exists)
}

pub async fn logical_switch_port_state(
    db_config: Arc<AppState>,
    project: String,
    component: String,
) -> anyhow::Result<StateComponentStatus> {

    // first check if project exists
    db_config.deployment_config_db
        .read()
        .await
        .get_deployment(project.clone())
        .await
        .context("Deployment does not exist")?;

    // if project exists, then lets get the state json
    let state_json = db_config.deployment_config_db
        .read()
        .await
        .get_state(project.clone())
        .await
        .context("Deployment exists but state json does not")?;

    let component_exists = match state_json.network {
        StateNetwork::Ovn(ovn) => {
            let lsp = ovn.switch_ports
                .get(&component)
                .ok_or(anyhow!("Logical Switch Port {} does not exist", component))?
                .check_exists(project).await?;
            lsp
        }
        StateNetwork::Ovs(_) => {
            unimplemented!()
        },
    };

    Ok(component_exists)

}

pub async fn logical_router_state(
    db_config: Arc<AppState>,
    project: String,
    component: String,
) -> anyhow::Result<StateComponentStatus> {

    // first check if project exists
    db_config.deployment_config_db
        .read()
        .await
        .get_deployment(project.clone())
        .await
        .context("Deployment does not exist")?;

    // if project exists, then lets get the state json
    let state_json = db_config.deployment_config_db
        .read()
        .await
        .get_state(project.clone())
        .await
        .context("Deployment exists but state json does not")?;

    let component_exists = match state_json.network {
        StateNetwork::Ovn(ovn) => {
            let lr = ovn.routers
                .get(&component)
                .ok_or(anyhow!("Logical Router {} does not exist", component))?
                .check_exists(project).await?;
            lr
        }
        StateNetwork::Ovs(_) => {
            unimplemented!()
        },
    };

    Ok(component_exists)

}

pub async fn logical_router_port_state(
    db_config: Arc<AppState>,
    project: String,
    component: String,
) -> anyhow::Result<StateComponentStatus> {

    // first check if project exists
    db_config.deployment_config_db
        .read()
        .await
        .get_deployment(project.clone())
        .await
        .context("Deployment does not exist")?;

    // if project exists, then lets get the state json
    let state_json = db_config.deployment_config_db
        .read()
        .await
        .get_state(project.clone())
        .await
        .context("Deployment exists but state json does not")?;

    let component_exists = match state_json.network {
        StateNetwork::Ovn(ovn) => {
            let lrp = ovn.router_ports
                .get(&component)
                .ok_or(anyhow!("Logical Router Port {} does not exist", component))?
                .check_exists(project).await?;
            lrp
        }
        StateNetwork::Ovs(_) => {
            unimplemented!()
        },
    };

    Ok(component_exists)

}

pub async fn logical_acl_state(
    db_config: Arc<AppState>,
    project: String,
    component: String,
) -> anyhow::Result<StateComponentStatus> {

    // first check if project exists
    db_config.deployment_config_db
        .read()
        .await
        .get_deployment(project.clone())
        .await
        .context("Deployment does not exist")?;

    // if project exists, then lets get the state json
    let state_json = db_config.deployment_config_db
        .read()
        .await
        .get_state(project.clone())
        .await
        .context("Deployment exists but state json does not")?;

    // network components saved as their full name with project prepended
    let name = format!("{}-{}", project, component);

    let component_exists = match state_json.network {
        StateNetwork::Ovn(ovn) => {
            let acl = ovn.acl
                .get(&name)
                .ok_or(anyhow!("ACL {} does not exist", name))?
                .check_exists(project).await?;
            acl
        }
        StateNetwork::Ovs(_) => {
            unimplemented!()
        },
    };

    Ok(component_exists)

}

pub async fn logical_dhcp_state(
    db_config: Arc<AppState>,
    project: String,
    component: String,
) -> anyhow::Result<StateComponentStatus> {
    unimplemented!()
}

pub async fn ovs_port_state(
    db_config: Arc<AppState>,
    project: String,
    component: String,
) -> anyhow::Result<StateComponentStatus> {

    // first check if project exists
    db_config.deployment_config_db
        .read()
        .await
        .get_deployment(project.clone())
        .await
        .context("Deployment does not exist")?;

    // if project exists, then lets get the state json
    let state_json = db_config.deployment_config_db
        .read()
        .await
        .get_state(project.clone())
        .await
        .context("Deployment exists but state json does not")?;

    // network components saved as their full name with project prepended
    let name = format!("{}-{}", project, component);

    let ovs_port = match state_json.network {
        StateNetwork::Ovn(ref ovn) => {
            ovn.ovs_ports.get(&name).ok_or(anyhow!("OVS Port {} does not exist", name))?
        }
        StateNetwork::Ovs(_) => unimplemented!(),
    };
    // get the cluster config with all host information
    let cluster_config = db_config.config_db
        .read()
        .await
        .get_cluster_config()
        .await?;
    // get the host's config
    let host_config = cluster_config.testbed_host_ssh_config
        .get(&ovs_port.chassis)
        .ok_or(anyhow!("OVS port's host {} does not exist", ovs_port.chassis))?;
    // if this host config is "main" then we can poll from this as this is the "main" server as well
    // but if it is not main, then we need to relay the state request to the correct testbed server
    let port_exists = if host_config.is_main_host.ok_or(anyhow!("Host {} has not been given an if main", ovs_port.chassis))? {
        // on main
        ovs_port.check_exists(project).await?
    } else {
        unimplemented!()
    };

    Ok(port_exists)

}

