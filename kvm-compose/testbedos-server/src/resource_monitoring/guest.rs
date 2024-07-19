use std::sync::Arc;
use std::time::Duration;
use anyhow::{bail, Context};
use serde_json::{json, Value};
use crate::resource_monitoring::{helpers, METRICS_SAMPLE_RATE_S};
use crate::resource_monitoring::helpers::{cgroup_get_cpu_time, cgroup_get_current_memory};
use crate::ServiceClients;

/// This function gets the metrics for a libvirt guest
pub async fn get_libvirt_guest_metrics(
    guest_name: &String,
    project_name: &String,
    _service_clients: &Arc<ServiceClients>,
) -> anyhow::Result<Value> {
    tracing::debug!("getting guest {guest_name} resource metrics");

    // the libvirt connection is not thread safe, so we will drop the connection as soon as were
    // done with it by placing it in a context

    let memory_usage = {
        // connect to libvirt and get domain info
        let domain = helpers::get_libvirt_domain(project_name, guest_name)?;

        // https://libvirt.org/html/libvirt-libvirt-domain.html#virDomainMemoryStatTags
        // https://libvirt.org/html/libvirt-libvirt-domain.html#virDomainMemoryStats
        let mem_stats = &domain.memory_stats(
            // mem_unused,
            0,
        )?;
        // see tag information in links above
        let unused = mem_stats.iter().find(|m| m.tag == 4)
            .context("getting unused memory for libvirt guest")?;
        let available = mem_stats.iter().find(|m| m.tag == 5)
            .context("getting available memory for libvirt guest")?;
        // for now just report the % used
        let fraction = unused.val as f64 / available.val as f64;
        100.0 - ((fraction) * 100.0)
    };


    // https://stackoverflow.com/questions/40468370/what-does-cpu-time-represent-exactly-in-libvirt
    // CPU usage requires two samples to take the average, this could be terribly inaccurate but it
    // is a start - have seen usage occasionally go slightly over 100% but probably due to
    // the inaccuracies of sampling and the type conversions

    let time1 = std::time::SystemTime::now();
    let cpu_time1 = {
        let domain = helpers::get_libvirt_domain(project_name, guest_name)?;
        domain.get_info()?.cpu_time
    };

    // wait half a second before sampling cpu time again
    tokio::time::sleep(Duration::from_secs_f32(METRICS_SAMPLE_RATE_S)).await;

    let time2 = std::time::SystemTime::now();
    let cpu_time2 = {
        let domain = helpers::get_libvirt_domain(project_name, guest_name)?;
        domain.get_info()?.cpu_time
    };

    let time_diff = time2.duration_since(time1)?.as_nanos();
    let cpu_usage = (100 * (cpu_time2 as u128 - cpu_time1 as u128)) as f64 / time_diff as f64;
    // tracing::info!("@1 cpu = {}, @2 cpu = {}, time diff = {time_diff}, usage = {cpu_usage}", cpu_time1, cpu_time2);

    // divide usage by number of cpus to keep usage under 100
    let n_cpu = {
        let domain = helpers::get_libvirt_domain(project_name, guest_name)?;
        domain.get_info()?.nr_virt_cpu
    };
    let cpu_usage = cpu_usage / n_cpu as f64;

    let guest_data = json!({
        "name": guest_name,
        "cpu": cpu_usage,
        "memory": memory_usage,
    });

    Ok(guest_data)
}

/// This function gets the metrics for a libvirt guest
pub async fn get_docker_guest_metrics(
    guest_name: &String,
    project_name: &String,
    service_clients: &Arc<ServiceClients>,
) -> anyhow::Result<Value> {
    tracing::debug!("getting guest {guest_name} resource metrics");

    let inspect = service_clients.docker_conn
        .write()
        .await
        .inspect_guest(&format!("{}-{}", project_name, guest_name))
        .await.context("getting docker guest inspect json from docker daemon")?;
    let uuid = inspect["Id"]
        .as_str().context("getting uuid as str")?.to_string();
    let status = inspect["State"]["Status"].to_string();
    // tracing::info!("docker uuid = {uuid}");
    // tracing::info!("docker status = {status}");
    if !status.eq("\"running\"") {
        bail!("docker guest {guest_name} is not in a running state");
    }
    if uuid.is_empty() {
        bail!("uuid for docker guest {guest_name} was empty");
    }

    let metrics_folder = helpers::get_docker_cgroup_folder(uuid);

    // the cpu.stat file contains a few different values, we will take usage_usec

    let time1 = std::time::SystemTime::now();
    let cpu_time1 = cgroup_get_cpu_time(&metrics_folder).await?;

    // wait half a second before sampling cpu time again
    tokio::time::sleep(Duration::from_secs_f32(METRICS_SAMPLE_RATE_S)).await;

    let time2 = std::time::SystemTime::now();
    let cpu_time2 = cgroup_get_cpu_time(&metrics_folder).await?;

    let time_diff = time2.duration_since(time1)?.as_nanos();
    let cpu_usage = (100 * (cpu_time2 as u128 - cpu_time1 as u128)) as f64 / time_diff as f64;

    // // TODO - divide by number cpus, this value is empty in the inspect but running `nproc` in the
    // //  container shows the number of cpus of the host. we currently dont offer resource limiting
    // //  containers so just dividing by host core count should be enough

    // do memory https://www.kernel.org/doc/html/latest/admin-guide/cgroup-v2.html#memory

    let mem_usage = cgroup_get_current_memory(&metrics_folder).await? as f64;
    let mem_usage = mem_usage / 1_000_000_000.0;


    // the following is getting starts from the docker stats endpoint, but as you increase the
    // number of guests, even though these endpoints are running concurrently, they are still slow
    // so we use the filesystem directly

    // let stats_json = service_clients.docker_conn
    //     .write()
    //     .await
    //     .get_guest_stats(&format!("{project_name}-{guest_name}"))
    //     .await?;
    //
    // // from https://docs.docker.com/engine/api/v1.43/#tag/Container/operation/ContainerStats
    //
    // // the algorithm to get CPU % is the following
    // let cpu_total = stats_json["cpu_stats"]["cpu_usage"]["total_usage"].as_u64()
    //     .context("getting docker cpu total usage as u64")?;
    // let pre_cpu_total = stats_json["precpu_stats"]["cpu_usage"]["total_usage"].as_u64()
    //     .context("getting docker pre-cpu total usage as u64")?;
    // let cpu_delta = (cpu_total - pre_cpu_total) as f64;
    //
    // let sys_cpu_usage = stats_json["cpu_stats"]["system_cpu_usage"].as_u64()
    //     .context("getting docker system_cpu_usage")?;
    // let sys_pre_cpu_usage = stats_json["precpu_stats"]["system_cpu_usage"].as_u64()
    //     .context("getting docker pre system_cpu_usage")?;
    // let sys_cpu_delta = (sys_cpu_usage - sys_pre_cpu_usage) as f64;
    //
    // let n_cpus = stats_json["cpu_stats"]["online_cpus"].as_u64()
    //     .context("getting docker online cpus")? as f64;
    //
    // let cpu_usage = ((cpu_delta / sys_cpu_delta) * n_cpus) * 100.0;
    //
    // // memory - current usage
    // let mem_usage = stats_json["memory_stats"]["usage"].as_f64()
    //     .context("getting docker mem usage")?;
    // // make into gigabytes
    // let mem_usage = mem_usage / 1_000_000_000.0;


    let guest_data = json!({
        "name": guest_name,
        "cpu": cpu_usage,
        "memory": mem_usage,
    });

    Ok(guest_data)
}

/// This function gets the metrics for a libvirt guest
#[allow(unused_variables)] // TODO - remove once implemented
pub async fn get_android_guest_metrics(
    guest_name: &String,
    project_name: &str,
    _service_clients: &Arc<ServiceClients>,
) -> anyhow::Result<Value> {

    // apply same logic as docker metrics collection, but different cgroup folder

    // TODO - android emulator is not currently in a cgroup, return default values for now

    // let metrics_folder = get_android_cgroup_folder(&guest_name, &project_name)?;
    //
    // let mem_usage = cgroup_get_current_memory(&metrics_folder).await? as f64;
    // let mem_usage = mem_usage / 1_000_000_000.0;
    //
    // let time1 = std::time::SystemTime::now();
    // let cpu_time1 = cgroup_get_cpu_time(&metrics_folder).await?;
    //
    // // wait half a second before sampling cpu time again
    // tokio::time::sleep(Duration::from_secs_f32(METRICS_SAMPLE_RATE_S)).await;
    //
    // let time2 = std::time::SystemTime::now();
    // let cpu_time2 = cgroup_get_cpu_time(&metrics_folder).await?;
    //
    // let time_diff = time2.duration_since(time1)?.as_nanos();
    // let cpu_usage = (100 * (cpu_time2 as u128 - cpu_time1 as u128)) as f64 / time_diff as f64;

    let guest_data = json!({
        "name": guest_name,
        "cpu": 0,
        "memory": 0,
    });

    Ok(guest_data)
}
