use serde::Deserialize;
use std::fmt;
use std::fmt::Formatter;
use std::path::PathBuf;
use std::time::Duration;
use anyhow::bail;
use async_trait::async_trait;
use chrono::{TimeZone, Utc};
use crate::orchestration::{OrchestrationCommon, run_testbed_orchestration_command, run_testbed_orchestration_command_allow_fail};
use crate::snapshot::GuestDiskSnapshot;

pub async fn load_qemu_img(
    img_path: &String,
    testbed_host: &str,
    common: &OrchestrationCommon,
) -> anyhow::Result<QemuImg> {
    let json_data = run_testbed_orchestration_command(
        common,
        testbed_host,
        "sudo",
        vec!["qemu-img", "info", "--output=json", &img_path, "--force-share"],
        false,
        None,
    ).await?;
    QemuImg::new(json_data)
}

/// This is the representation of the json format of running `qemu-img info` on a given .qcow2
/// format image. We use this to get the data from an image so we can then create the rest of the
/// abstractions for snapshots. This is only a partial representation of what is returned, only
/// picking specifically what we need.
#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub struct QemuImg {
    pub snapshots: Option<Vec<QemuImgSnapshot>>,
    pub filename: PathBuf,
}

impl QemuImg {
    pub fn new(
        json_data: String,
    ) -> anyhow::Result<Self> {
        let json: Self = serde_json::from_str(&json_data)?;
        Ok(json)
    }

    fn get_path(&self) -> &str {
        self.filename.to_str().unwrap()
    }

    fn get_domain_xml_path(&self) -> anyhow::Result<String> {
        // TODO - this is pretty bad, get the xml definitively from state
        // interpolate from the filepath, we can assume the domain xml is also in the artefacts
        // folder with the image
        let artefacts_folder = self.filename.parent().unwrap();
        let img_name = self.filename.file_stem().unwrap().to_str().unwrap();
        // let project_name_hyphen = format!("{}-", &common.project_name);
        // tracing::info!("{img_name:?}");
        // tracing::info!("{project_name_hyphen:?}");
        // let split = img_name.split(&project_name_hyphen).collect_vec();
        // tracing::info!("{split:?}");
        // let split_2nd = split[1];
        // we know the following types of disk are possible
        let guest_name_idx = if let Some(index) = img_name.rfind("-cloud-disk") {
            Some(index)
        } else if let Some(index) = img_name.rfind("-linked-clone") {
            Some(index)
        } else { img_name.rfind("-iso-guest") };
        let guest_name = if guest_name_idx.is_none() {
            bail!("could not find guest name to work out the domain xml filepath");
        } else {
            img_name.split_at(guest_name_idx.unwrap()).0
        };
        let xml_name = format!("{}/{guest_name}-domain.xml", artefacts_folder.to_str().unwrap());
        Ok(xml_name)
    }

    async fn is_running(&self, guest_name: &str, testbed_host: &str, common: &OrchestrationCommon) -> anyhow::Result<bool> {
        tracing::info!("checking if guest {guest_name} is up");
        let cmd = vec!["virsh", "dominfo", &guest_name];
        let res = run_testbed_orchestration_command_allow_fail(
            common,
            testbed_host,
            "sudo",
            cmd,
            false,
            None,
        ).await;
        match &res {
            Ok(res) => {
                if res.contains("running") {
                    Ok(true)
                } else {
                    Ok(false)
                }
            }
            Err(err) => {
                tracing::error!("{err:#}");
                Ok(false)
            }
        }
    }

    async fn wait_until_stopped(&self, guest_name: &str, testbed_host: &str, common: &OrchestrationCommon) -> anyhow::Result<()> {
        // check if guest is down until a certain timeout
        let timeout_max = 12;
        let timeout_s = 5;
        let mut counter = 0;
        loop {
            tracing::info!("checking if guest is down before continuing ...");
            let res = self.is_running(guest_name, testbed_host, common).await?;
            if counter > timeout_max && !res {
                bail!("could not ensure guest was down, too slow or issue?");
            }
            if res {
                tracing::info!("attempt {counter}/{timeout_max} guest {guest_name} not down yet, waiting 5s and trying again");
                tokio::time::sleep(Duration::from_secs(timeout_s)).await;
                counter += 1;
            } else {
                break;
            }
        }
        Ok(())
    }

    async fn stop_vm(&self, guest_name: &str, testbed_host: &str, common: &OrchestrationCommon) -> anyhow::Result<()> {
        tracing::info!("stopping guest {guest_name}");
        run_testbed_orchestration_command(
            common,
            testbed_host,
            "sudo",
            vec!["virsh", "shutdown", guest_name],
            false,
            None,
        ).await?;
        self.wait_until_stopped(guest_name, testbed_host, common).await?;
        Ok(())
    }


    async fn start_vm(&self, guest_name: &str, testbed_host: &str, common: &OrchestrationCommon) -> anyhow::Result<()> {
        tracing::info!("resuming guest {guest_name}");
        run_testbed_orchestration_command(
            common,
            testbed_host,
            "sudo",
            vec!["virsh", "create", &self.get_domain_xml_path()?],
            false,
            None,
        ).await?;
        Ok(())
    }
}

#[async_trait]
impl GuestDiskSnapshot for QemuImg {
    fn print_list(&self) -> Vec<String> {
        let mut snap_list = Vec::new();
        if let Some(snapshot_list) = &self.snapshots {
            for snapshot in snapshot_list {
                snap_list.push(format!("{}", snapshot));
            }
        } else {
            snap_list.push("\nno snapshots...".to_string());
        }
        snap_list
    }

    fn info(&self) -> String {
        format!("{self}")
    }

    fn info_one(&self, snapshot_name: &str) -> Option<String> {
        if let Some(snapshots) = &self.snapshots {
            for snap in snapshots {
                if snap.name.eq(snapshot_name) {
                    return Some(format!("{}", snap))
                }
            }
        }
        None
    }

    fn list(&self) -> Option<Vec<String>> {
        if let Some(snapshots) = &self.snapshots {
            return Some(snapshots.iter()
                .map(|snp| snp.name.clone())
                .collect())
        }
        None
    }

    async fn create(&self, guest_name: &str, snapshot_name: &str, testbed_host: &str, common: &OrchestrationCommon) -> anyhow::Result<()> {
        // TODO - do we want a snapshot with memory or without?
        // we need to shut down the guest first if it is on
        let is_running = self.is_running(guest_name, testbed_host, common).await?;
        // if guest is running, we can use virsh snapshot so we don't have to turn off the guest
        // if it is not running, we have to use qemu-img
        if is_running {
            let cmd = vec!["virsh", "snapshot-create-as", &guest_name, "--name", snapshot_name, self.get_path()];
            let res = run_testbed_orchestration_command(
                common,
                testbed_host,
                "sudo",
                cmd,
                false,
                None,
            ).await;
            match res {
                Ok(_) => {
                    // tracing::info!("{ok}");
                }
                Err(err) => {
                    tracing::error!("{err:#}");
                }
            }
        } else {
            let cmd = vec!["qemu-img", "snapshot", "-c", snapshot_name, &self.get_path()];
            let res = run_testbed_orchestration_command(
                common,
                testbed_host,
                "sudo",
                cmd,
                false,
                None,
            ).await;
            match res {
                Ok(_) => {
                    // tracing::info!("{ok}");
                }
                Err(err) => {
                    tracing::error!("{err:#}");
                }
            }
        }
        Ok(())
    }

    async fn delete(&self, guest_name: &str, snapshot_name: &str, testbed_host: &str, common: &OrchestrationCommon) -> anyhow::Result<()> {
        // we need to shut down the guest first if it is on
        let is_running = self.is_running(guest_name, testbed_host, common).await?;
        if is_running {
            tracing::info!("guest is running, turning off to release write lock on image before continuing...");
            self.stop_vm(guest_name,  testbed_host, common).await?;
        }
        let cmd = vec!["qemu-img", "snapshot", "-d", snapshot_name, &self.get_path()];
        let res = run_testbed_orchestration_command(
            common,
            testbed_host,
            "sudo",
            cmd,
            false,
            None,
        ).await?;
        tracing::info!("{res}");
        if is_running {
            tracing::info!("starting guest as it was running before executing snapshot command");
            self.start_vm(guest_name, testbed_host, common).await?;
        }
        Ok(())
    }

    async fn delete_all(&self, guest_name: &String, testbed_host: &String, common: &OrchestrationCommon) -> anyhow::Result<()> {
        if let Some(snapshots) = &self.snapshots {
            // we need to shut down the guest first if it is on
            let is_running = self.is_running(guest_name, testbed_host, common).await?;
            if is_running {
                tracing::info!("guest is running, turning off to release write lock on image before continuing...");
                self.stop_vm(guest_name,  testbed_host, common).await?;
            }
            for snap in snapshots {
                tracing::info!("deleting snapshot {}", &snap.name);
                let cmd = vec!["qemu-img", "snapshot", "-d", &snap.name, &self.get_path()];
                let res = run_testbed_orchestration_command(
                    common,
                    testbed_host,
                    "sudo",
                    cmd,
                    false,
                    None,
                ).await?;
                tracing::info!("{res}");
            }
            if is_running {
                tracing::info!("starting guest as it was running before executing snapshot command");
                self.start_vm(guest_name, testbed_host, common).await?;
            }
        }

        Ok(())
    }

    async fn restore(&self, guest_name: &String, snapshot_name: &String, testbed_host: &String, common: &OrchestrationCommon) -> anyhow::Result<()> {
        // we need to shut down the guest first if it is on
        let is_running = self.is_running(guest_name, testbed_host, common).await?;
        if is_running {
            tracing::info!("guest is running, turning off to release write lock on image before continuing...");
            self.stop_vm(guest_name,  testbed_host, common).await?;
        }
        let cmd = vec!["qemu-img", "snapshot", "-a", snapshot_name, &self.get_path()];
        let res = run_testbed_orchestration_command(
            common,
            testbed_host,
            "sudo",
            cmd,
            false,
            None,
        ).await?;
        tracing::info!("{res}");
        if is_running {
            tracing::info!("starting guest as it was running before executing snapshot command");
            self.start_vm(guest_name, testbed_host, common).await?;
        }
        Ok(())
    }

    fn get_most_recent_snapshot(&self) -> Option<String> {
        if let Some(snapshots) = &self.snapshots {
            let max = snapshots.iter()
                .max_by_key(|snap| snap.date_sec);
            max.map(|latest_snap| latest_snap.name.clone())
        } else {
            None
        }
    }
}

impl fmt::Display for QemuImg {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let mut string = format!("\nQemu Img:\nfilename = {:?}\nSnapshot list:\n", &self.filename);
        if let Some(snapshot_list) = &self.snapshots {
            for snap in snapshot_list {
                string.push_str(&format!("\t{}", snap));
            }
        } else {
            string.push_str("\n\tNo snapshots.");
        }
        f.write_str(&string)
            .expect("Pretty printing QemuImgSnapshot failed");

        Ok(())
    }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub struct QemuImgSnapshot {
    pub name: String,
    pub id: String,
    #[serde(rename = "date-sec")]
    pub date_sec: i64,
}

impl fmt::Display for QemuImgSnapshot {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let epoch_to_date = Utc.timestamp_opt(self.date_sec, 0).unwrap();
        let string = format!(
            "\nSnapshot:\n\tname = {}\n\tid = {}\n\tdate = {}\n",
            self.name, self.id, epoch_to_date,
        );
        f.write_str(&string)
            .expect("Pretty printing QemuImgSnapshot failed");

        Ok(())
    }
}
