use std::collections::HashMap;
use std::sync::Arc;
use anyhow::{Context};
use futures_util::future::{try_join_all};
use serde_json::{json, Value};
use sysinfo::{CpuRefreshKind, MemoryRefreshKind, RefreshKind, System};
use tokio::sync::RwLockWriteGuard;
use kvm_compose_lib::state::{State};
use kvm_compose_schemas::deployment_models::{Deployment, DeploymentList, DeploymentState};
use kvm_compose_schemas::kvm_compose_yaml::machines::GuestType;
use kvm_compose_schemas::settings::{TestbedClusterConfig};
use crate::resource_monitoring::guest::{get_android_guest_metrics, get_docker_guest_metrics, get_libvirt_guest_metrics};
use crate::ServiceClients;

/// This function will find all the active deployments the testbed is currently running
pub async fn get_active_deployments(
    deployment_list: &HashMap<String, Deployment>,
) -> anyhow::Result<Vec<(&String, &Deployment)>> {
    let active_deployments: Vec<_> = deployment_list.iter()
        .filter(|(_,d)| {
            match d.state {
                DeploymentState::Up => true,
                _ => false,
            }
        })
        .collect();
    Ok(active_deployments)
}

/// This function will return a vector of strings, which are the names of the guests
pub async fn get_active_guests_for_deployment(
    deployment: &Deployment,
) -> anyhow::Result<Vec<String>> {
    let state = get_state_file(deployment).await?;
    let guest_list: Vec<_> = state.testbed_guests.0.values().map(|g| g.guest_type.name.clone())
        .collect();
    Ok(guest_list)
}

/// This function will return a vector of strings, which are the names of the hosts
pub async fn get_active_hosts_for_deployment(
    deployment: &Deployment,
) -> anyhow::Result<Vec<String>> {
    let state = get_state_file(deployment).await?;
    let host_list: Vec<_> = state.testbed_hosts.0.keys().cloned()
        .collect();
    Ok(host_list)
}

/// This function will collect resource data on the current testbed host
pub async fn collect_from_host(
    host_name: String,
    mut system: RwLockWriteGuard<'_,System>,
) -> anyhow::Result<Value> {

    // poll for new data
    system.refresh_specifics(
        RefreshKind::new()
            .with_cpu(CpuRefreshKind::everything())
            .with_memory(MemoryRefreshKind::everything())
    );
    // we will aggregate all cpu values
    let mut cpu_usage = 0f32;
    let cpu_n = system.cpus().len();
    for cpu in system.cpus() {
        cpu_usage += cpu.cpu_usage();
    }
    let final_cpu_usage = (cpu_usage / cpu_n as f32) as u32;
    // we need to put this in gigabytes
    let memory_usage = system.used_memory() / 1_000_000_000;

    let host_data = json!({
        "name": host_name,
        "cpu": final_cpu_usage,
        "memory": memory_usage,
    });

    Ok(host_data)
}

/// This function will collect guest resource data on the current testbed. Since the guest is
/// possibly on a client testbed host, we need to get the guest's name for the machine type and
/// then look at the corresponding service providing the guest i.e. libvirt.
pub async fn collect_from_guest(
    guest_name: String,
    project_name: String,
    master_ip: &String,
    service_clients: Arc<ServiceClients>,
) -> anyhow::Result<Value> {
    let deployment_state_url = format!("http://{master_ip}:3355/api/deployments/{}/state", &project_name);
    // make call to master testbed to get the deployment and then find the guest to get it's type
    let deployment_state = reqwest::get(deployment_state_url)
        .await?
        .text()
        .await?;
    let deployment_state: State = serde_json::from_str(&deployment_state)?;
    let guest_state = deployment_state.testbed_guests.0.get(&guest_name)
        .context("getting guest from state in metrics collection")?;
    let guest_type = &guest_state.guest_type.guest_type;

    // let metrics_url = format!("http://{master_ip}:3355/api/metrics/guest/{}/{}", &project_name, &guest_name);

    let guest_stats = match guest_type {
        GuestType::Libvirt(_) => {
            get_libvirt_guest_metrics(&guest_name, &project_name, &service_clients).await?
        }
        GuestType::Docker(_) => {
            get_docker_guest_metrics(&guest_name, &project_name, &service_clients).await?
        }
        GuestType::Android(_) => {
            get_android_guest_metrics(&guest_name, &project_name, &service_clients).await?
        }
    };


    Ok(guest_stats)
}

/// This function will collect all metrics and return the prometheus ready response for prometheus
/// to scrape and add to it's database. This function will need to take the active deployments and
/// then inspect each state file for each deployment to get which testbed host the guest has been
/// assigned to. Then each testbed host is polled for the
pub async fn collect_metrics_for_hosts(
    testbed_cluster_config: &TestbedClusterConfig,
) -> anyhow::Result<String> {

    // store the response
    let mut metrics = String::new();

    // get testbed host resource data (any host in the cluster)
    for (_, host_config) in &testbed_cluster_config.testbed_host_ssh_config {
        // in case the testbed is running only on master and the private interface is not up or
        // set in the config, we must just resort to localhost
        let host_metrics_url = if let Some(is_master) = host_config.is_master_host {
            if is_master {
                "http://localhost:3355/api/metrics/host".to_string()
            } else {
                format!("http://{}:3355/api/metrics/host", &host_config.ip)
            }
        } else {
            format!("http://{}:3355/api/metrics/host", &host_config.ip)
        };
        let host_metrics_response = reqwest::get(host_metrics_url)
            .await?
            .text()
            .await?;
        let host_metrics: Value = serde_json::from_str(&host_metrics_response)?;
        parse_host_metrics(&mut metrics, host_metrics)?;
    }

    Ok(metrics)
}

pub async fn collect_metrics_for_guests(
    testbed_cluster_config: &TestbedClusterConfig,
    deployment_list: &DeploymentList,
    guest_type: &str,
) -> anyhow::Result<String> {
    // store the response
    let mut metrics = String::new();

    // get testbed guest resource data
    // filter for active deployments
    let active_deployments = get_active_deployments(&deployment_list.deployments).await?;
    // store the futures so we can call them all concurrently
    let mut guest_metrics_futures = Vec::new();
    // for each deployment, we need to access the state file and get the guest and it's assigned
    // testbed host
    for (_, deployment) in active_deployments {
        // get and read the state for the deployment
        let state = get_state_file(deployment).await?;
        // get the metrics for each guest in the deployment
        for (guest_name, guest_config) in state.testbed_guests.0 {
            let testbed_host = guest_config.testbed_host
                .context("get tb host for guest in metrics collector")?;

            // the endpoint that calls this function will be specific to a type of guest
            match guest_config.guest_type.guest_type {
                GuestType::Libvirt(_) => {
                    if guest_type != "libvirt" {
                        continue;
                    }
                }
                GuestType::Docker(_) => {
                    if guest_type != "docker" {
                        continue;
                    }
                }
                GuestType::Android(_) => {
                    if guest_type != "android" {
                        continue;
                    }
                }
            }

            let host_metrics_url = if let Some(testbed_host) = &testbed_cluster_config
                .testbed_host_ssh_config
                .get(&testbed_host)

            {
                if let Some(is_master) = testbed_host.is_master_host {
                    if is_master {
                        format!("http://localhost:3355/api/metrics/guest/{}/{}", &deployment.name, &guest_name)
                    } else {
                        format!("http://{}:3355/api/metrics/guest/{}/{}", &testbed_host.ip, &deployment.name, &guest_name)
                    }
                } else {
                    format!("http://{}:3355/api/metrics/guest/{}/{}", &testbed_host.ip, &deployment.name, &guest_name)
                }

            } else {
                tracing::error!("could not get testbed host config for {} for guest {} in project {} to collect metrics, not currently part of cluster", &testbed_host, &guest_name, &deployment.name);
                continue;
            };
            guest_metrics_futures.push(guest_metrics_from_testbed_hosts(host_metrics_url, deployment.name.clone(), guest_name.clone()));

        }
    }
    // join all futures so we can get the guest metrics in parallel in case there are many guests
    // where a sequential poll could risk taking longer than 5 seconds
    let results = try_join_all(guest_metrics_futures).await?;
    for maybe_metric in results {
        if let Some(metric) = maybe_metric {
            metrics.push_str(&metric);
        }
    }
    Ok(metrics)
}

/// This function requests the guest metrics from the testbed host that is running the guest
async fn guest_metrics_from_testbed_hosts(
    host_metrics_url: String,
    deployment_name: String,
    guest_name: String,
) -> anyhow::Result<Option<String>>{
    let guest_metrics_response = reqwest::get(host_metrics_url)
        .await?;
    // dont fail the whole response, just put error in logs and continue
    if !guest_metrics_response.status().is_success() {
        let err = &guest_metrics_response.text().await?;
        tracing::error!("error collecting guest {} metrics, will skip, error: {err:?}", &guest_name);
        return Ok(None);
    }
    // get the successful response text
    let guest_metrics_response = guest_metrics_response
        .text()
        .await?;

    let mut metrics = String::new();
    let guest_metrics: Value = serde_json::from_str(&guest_metrics_response)
        .context("getting guest metrics response")?;
    parse_guest_metrics(&mut metrics, &deployment_name, guest_metrics)
        .context("parsing guest metrics response")?;
    Ok(Some(metrics))
}

/// Get the state file for the given deployment on the master testbed's filesystem
async fn get_state_file(
    deployment: &Deployment,
) -> anyhow::Result<State> {
    let state_file_path = format!("{}/{}-state.json", &deployment.project_location, &deployment.name);
    let text = tokio::fs::read_to_string(state_file_path).await?;
    let state: State = serde_json::from_str(&text)?;
    Ok(state)
}

/// Create the metric values for the specified testbed host. Each time series is tagged with the
/// name of the testbed host.
fn parse_host_metrics(
    metrics: &mut String,
    value: Value,
) -> anyhow::Result<()> {
    let name = value["name"].as_str()
        .context("getting host name from serde json Value")?;

    metrics.push_str(&format!("testbed_host_cpu{{host_name=\"{}\"}} {}\n", name, value["cpu"]));

    metrics.push_str(&format!("testbed_host_memory{{host_name=\"{}\"}} {}\n", name, value["memory"]));
    Ok(())
}

/// Create the metric values for the specified testbed guest. Each time series is tagged with a
/// label that corresponds to the project the guest is part of and the guest name.
fn parse_guest_metrics(
    metrics: &mut String,
    project_name: &String,
    value: Value,
) -> anyhow::Result<()> {
    let name = value["name"].as_str()
        .context("getting guest name from serde json Value")?;

    metrics.push_str(&format!("testbed_guest_cpu{{project=\"{}\",guest_name=\"{}\"}} {}\n", project_name, name, value["cpu"]));

    metrics.push_str(&format!("testbed_guest_memory{{project=\"{}\",guest_name=\"{}\"}} {}\n", project_name, name, value["memory"]));
    Ok(())
}
