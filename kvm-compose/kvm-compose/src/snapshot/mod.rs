pub mod qemu_img;
pub mod snapshot_cmd;
pub mod testbed_snapshot;

use std::collections::HashMap;
use anyhow::Context;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use futures_util::future::try_join_all;
use kvm_compose_schemas::kvm_compose_yaml::machines::GuestType;
use kvm_compose_schemas::kvm_compose_yaml::machines::libvirt::LibvirtGuestOptions;
use crate::orchestration::{OrchestrationCommon};
use crate::state::State;

// The definitions in this file abstract over the qemu-img command and the guests in a given
// deployment.

/// This represents one guest and it's list of snapshots, if any. This provides an abstraction
/// around the different guest snapshot implementations so we can provide a generic interface to
/// manipulate the snapshots.
pub struct GuestSnapshots {
    guest_name: String,
    testbed_host: String,
    pub(crate) snapshot: Box<dyn GuestDiskSnapshot + Sync + Send>,
}

impl GuestSnapshots {

    pub fn list(&self) -> String {
        let snap_list = self.snapshot.print_list();
        let mut print_string = format!("Guest name: {}", &self.guest_name);
        for snap in snap_list {
            print_string.push_str(&snap);
        }
        tracing::info!("{print_string}");
        print_string
    }

    pub async fn create(&self, snapshot_name: &String, common: &OrchestrationCommon) -> anyhow::Result<()> {
        self.snapshot.create(&self.guest_name, snapshot_name, &self.testbed_host, common).await
    }

    pub async fn delete(&self, snapshot_name: &String, common: &OrchestrationCommon) -> anyhow::Result<()> {
        self.snapshot.delete(&self.guest_name, snapshot_name, &self.testbed_host, common).await
    }

    pub async fn delete_all(&self, common: &OrchestrationCommon) -> anyhow::Result<()> {
        self.snapshot.delete_all(&self.guest_name, &self.testbed_host, common).await
    }

    pub fn info(&self) -> String {
        tracing::info!("{}", self.snapshot.info());
        self.snapshot.info()
    }

    pub async fn restore(&self, snapshot_name: &String, common: &OrchestrationCommon) -> anyhow::Result<()> {
        // TODO - if restoring from a snapshot in the middle of a chain, need to tell the user that this could
        //  impact the child snapshots
        self.snapshot.restore(&self.guest_name, snapshot_name, &self.testbed_host, common).await
    }

    pub fn get_most_recent_snapshot(&self) -> Option<String> {
        self.snapshot.get_most_recent_snapshot()
    }
}

/// This represents a snapshot with respect to the whole testbed. To be used when creating a full
/// snapshot of the testbed for sharing with other people, to support the use case of reproducing
/// a deployment. Otherwise, this has a reference to all the guests and their snapshot capabilities.
pub struct TestbedSnapshots {
    project_name: String,
    pub guests: HashMap<String, GuestSnapshots>,
}

impl TestbedSnapshots {
    /// Get information about the testbed for setting up snapshots
    pub async fn new(
        state: &State,
        common: &OrchestrationCommon,
    ) -> anyhow::Result<Self> {
        let project_name = common.project_name.clone();
        let mut guests = HashMap::new();
        for (guest_name, guest_data) in state.testbed_guests.0.iter() {
            if guest_data.is_golden_image {continue;}
            let testbed_host = guest_data.testbed_host.as_ref().context("getting testbed host name in testbed snapshots")?;
            match &guest_data.guest_type.guest_type {
                GuestType::Libvirt(libvirt) => {
                    let corrected_guest_name = TestbedSnapshots::guest_name_helper(&project_name, &guest_name.clone());

                    let img_path = match &libvirt.libvirt_type {
                        LibvirtGuestOptions::CloudImage { path, .. } => path.as_ref().context("getting img path in testbed snapshots")?,
                        LibvirtGuestOptions::ExistingDisk { path, .. } => path,
                        LibvirtGuestOptions::IsoGuest { path, .. } => path,
                    };
                    let img_path_to_string = img_path.to_str()
                        .context("qemu img path to string")?.to_string();

                    guests.insert(
                        corrected_guest_name.clone(),
                        GuestSnapshots {
                            guest_name: corrected_guest_name.clone(),
                            testbed_host: testbed_host.clone(),
                            snapshot: Box::new(qemu_img::load_qemu_img(&img_path_to_string, testbed_host, common).await?),
                        },
                    );
                }
                GuestType::Docker(_) => {}
                GuestType::Android(_) => {}
            }
        }
        Ok(Self {
            project_name,
            guests,
        })
    }

    /// Make sure the user's input, whether they use the project name or not can be used with the
    /// internal representation as it is using <project name>-<guest name>. This returns the guest
    /// name always with the project name.
    fn guest_name_helper(
        project_name: &String,
        guest_name: &String,
    ) -> String {
        if guest_name.starts_with(&format!("{}-", &project_name)) {
            guest_name.clone()
        } else {
            format!("{}-{}", &project_name, guest_name)
        }
    }

    pub async fn info(&self, guest_name: &String) -> anyhow::Result<String> {
        let corrected_name = TestbedSnapshots::guest_name_helper(&self.project_name, guest_name);
        tracing::info!("Getting info on disk for guest {}:", corrected_name);
        let info = &self.guests.get(&corrected_name)
            .context("Getting name of guest to give disk info")?
            .info();
        Ok(info.clone())
    }

    pub async fn list_snapshots(&self, guest_name: &String) -> anyhow::Result<String> {
        let corrected_name = TestbedSnapshots::guest_name_helper(&self.project_name, guest_name);
        tracing::info!("Listing snapshots for guest {}:", corrected_name);
        let list_snapshots = &self.guests.get(&corrected_name)
            .context("Getting name of guest to list snapshots")?
            .list();
        Ok(list_snapshots.clone())
    }

    pub async fn list_all_snapshots(&self) -> anyhow::Result<String> {
        tracing::info!("Listing all snapshots.");
        let mut all_info = String::new();
        for (_, guest_snapshot) in &self.guests {
            all_info.push_str(&guest_snapshot.list());
            all_info.push_str("\n");
        }
        if all_info.is_empty() {
            return Ok("No snapshots to list".to_string());
        }
        Ok(all_info)
    }

    pub async fn delete_snapshot(
        &self,
        guest_name: &String,
        snapshot_name: &String,
        common: &OrchestrationCommon,
    ) -> anyhow::Result<()> {
        let corrected_name = TestbedSnapshots::guest_name_helper(&self.project_name, guest_name);
        tracing::info!("Deleting snapshot {} for guest {}:", snapshot_name, corrected_name);
        let _ = &self.guests.get(&corrected_name)
            .context("Getting name of guest to delete snapshot")?
            .delete(snapshot_name, common).await?;
        tracing::info!("snapshot delete successful");
        Ok(())
    }

    pub async fn delete_all_snapshots(
        &self,
        guest_name: &String,
        common: &OrchestrationCommon,
    ) -> anyhow::Result<()> {
        let corrected_name = TestbedSnapshots::guest_name_helper(&self.project_name, guest_name);
        tracing::info!("Deleting all snapshots for guest {}:", corrected_name);
        let guest = self.guests.get(&corrected_name)
            .context("Getting name of guest to delete snapshot")?;
        if let Some(_) = guest.snapshot.list() {
            guest.delete_all(common).await?;
        } else {
            tracing::info!("no snapshots to delete");
        }
        Ok(())
    }

    pub async fn create_snapshot(
        &self,
        guest_name: &String,
        snapshot_name: &String,
        common: &OrchestrationCommon
    ) -> anyhow::Result<()> {
        let corrected_name = TestbedSnapshots::guest_name_helper(&self.project_name, guest_name);
        tracing::info!("Creating snapshot {} for guest {}:", snapshot_name, corrected_name);
        let _ = &self.guests.get(&corrected_name)
            .context("Getting name of guest to create snapshot")?
            .create(snapshot_name, common).await?;
        tracing::info!("snapshot create successful");
        Ok(())
    }

    pub async fn snapshot_all_guests(
        &self,
        common: &OrchestrationCommon
    ) -> anyhow::Result<String> {
        // let corrected_name = TestbedSnapshots::guest_name_helper(&self.project_name, guest_name);
        let time_now: DateTime<Utc> = std::time::SystemTime::now().into();
        let group_snapshot_name = format!("group-snapshot-{}", time_now.format("%+"));
        tracing::info!("Creating snapshots for all guests, all will be given a snapshot with name: {}", &group_snapshot_name);
        let mut snapshot_futures = Vec::new();
        for (_, guest_data) in &self.guests {
            snapshot_futures.push(guest_data.create(&group_snapshot_name, common));
        }
        try_join_all(snapshot_futures).await?;
        tracing::info!("Successfully created snapshots for all guests with name: {}", &group_snapshot_name);
        Ok(group_snapshot_name)
    }

    pub async fn restore_from_snapshot(
        &self,
        guest_name: &String,
        snapshot_name: &String,
        common: &OrchestrationCommon,
    ) -> anyhow::Result<()> {
        let corrected_name = TestbedSnapshots::guest_name_helper(&self.project_name, guest_name);
        tracing::info!("Restoring from snapshot {} for guest {}:", snapshot_name, corrected_name);
        let _ = &self.guests.get(&corrected_name)
            .context("Getting name of guest to restore snapshot")?
            .restore(snapshot_name, common).await?;
        tracing::info!("snapshot restore successful");
        Ok(())
    }

    pub async fn restore_all_from_snapshots(&self) -> anyhow::Result<()> {
        // will restore from the latest snapshot
        todo!()
    }
}

/// All guests that support snapshots should implement this so that we have a unified interface
/// for calling the various snapshot manipulation commands.
#[async_trait]
pub trait GuestDiskSnapshot {
    /// Return a vector of of strings formatted information about any snapshots available for guest
    fn print_list(&self) -> Vec<String>;
    /// Return a string of the whole guest snapshot representation for this guest
    fn info(&self) -> String;
    fn info_one(&self, snapshot_name: &String) -> Option<String>;
    /// Return a vector of snapshot names for this guest
    fn list(&self) -> Option<Vec<String>>;
    async fn create(&self, guest_name: &String, snapshot_name: &String, testbed_host: &String, common: &OrchestrationCommon) -> anyhow::Result<()>;
    async fn delete(&self, guest_name: &String, snapshot_name: &String, testbed_host: &String, common: &OrchestrationCommon) -> anyhow::Result<()>;
    async fn delete_all(&self, guest_name: &String, testbed_host: &String, common: &OrchestrationCommon) -> anyhow::Result<()>;
    async fn restore(&self, guest_name: &String, snapshot_name: &String, testbed_host: &String, common: &OrchestrationCommon) -> anyhow::Result<()>;
    /// Get the name of the most recent snapshot for this guest, if it exists
    fn get_most_recent_snapshot(&self) -> Option<String>;
}
