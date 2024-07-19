
use virt::domain::Domain;
use virt::connect::Connect;
use anyhow::{bail, Context};
use glob::{glob};
use tokio::fs::File;
use tokio::io;
use tokio::io::AsyncBufReadExt;

/// Return the libvirt Domain struct for the given guest
pub fn get_libvirt_domain(
    project_name: &String,
    guest_name: &String,
) -> anyhow::Result<Domain> {
    let conn = Connect::open(Some("qemu:///system"))
        .context("connecting to libvirt to get guest metrics")?;
    let guest_project_name = format!("{project_name}-{guest_name}");
    let domain = virt::domain::Domain::lookup_by_name(&conn, &guest_project_name)
        .context("getting domain from libvirt connection")?;
    Ok(domain)
}

/// Return the folder for the docker container in the system cgroups folder, which contains the
/// guests resource usage
pub fn get_docker_cgroup_folder(
    uuid: String,
) -> String {
    format!("/sys/fs/cgroup/system.slice/docker-{uuid}.scope/")
}

pub async fn cgroup_get_cpu_time(
    cgroup_folder: &String,
) -> anyhow::Result<u64> {
    let cpu_stat_file = File::open(format!("{cgroup_folder}cpu.stat")).await?;
    let reader = io::BufReader::new(cpu_stat_file);
    let mut lines = reader.lines();

    while let Some(line) = lines.next_line().await? {
        let split: Vec<_> = line.split(' ').collect();
        if split[0].eq("usage_usec") {
            return Ok(split[1].parse::<u64>()?);
        }
    }

    bail!("could not get usage_usec");
}

/// current mem in bytes
pub async fn cgroup_get_current_memory(
    cgroup_folder: &String,
) -> anyhow::Result<u64> {
    let current_memory_file = File::open(format!("{cgroup_folder}memory.current")).await?;
    let reader = io::BufReader::new(current_memory_file);
    let mut lines = reader.lines();
    if let Some(mem) = lines.next_line().await? {
        Ok(mem.parse::<u64>()?)
    } else {
        bail!("could not get current memory");
    }
}

/// Return the folder for the android qemu virtual machine in the machine cgroups folder, which
/// contains the guests resource usage. We make sure there is only one match otherwise it is
/// ambiguous which cgroup folder to take. We just throw an error for now so we can then investigate
/// if this ever happens.
#[allow(dead_code)] // TODO - remove once implemented
pub fn get_android_cgroup_folder(
    guest_name: &String,
    project_name: &String,
) -> anyhow::Result<String> {
    // TODO - this is wrong, this is for libvirt
    let pattern = "/sys/fs/cgroup/machine.slice/machine-qemu*.scope/";
    let mut matches = Vec::new();
    if let Ok(entries) = glob(pattern) {
        for entry in entries {
            match entry {
                Ok(ref path) => {
                    // tracing::info!("{guest_name}, {project_name}, {:?}", &entry);
                    let path = path.to_str().context("path to str")?.to_string();
                    if path.contains(guest_name) && path.contains(project_name) {
                        // has both the project and guest name, candidate
                        matches.push(path);
                    }
                }
                Err(_) => {}
            }
        }
    }
    if matches.len() > 1 {
        bail!("matched cgroups with more than one guest, cant pick the correct cgroup: {matches:?}");
    } else if matches.len() == 1 {
        return Ok(matches[0].to_string());
    }

    bail!("could not find android qemu cgroups folder");
}
