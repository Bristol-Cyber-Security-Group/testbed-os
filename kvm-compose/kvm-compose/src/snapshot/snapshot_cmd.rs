use anyhow::{bail, Context};
use tokio::sync::mpsc::Sender;
use kvm_compose_schemas::cli_models::{SnapshotSubCommand};
use kvm_compose_schemas::kvm_compose_yaml::machines::GuestType;
use crate::orchestration::api::OrchestrationLogger;
use crate::orchestration::OrchestrationCommon;
use crate::snapshot::TestbedSnapshots;
use crate::state::State;

/// The snapshot action will run the respective command for snapshots. This will be called by either
/// the CLI or GUI, so to ensure compatability with both, we are both logging the output to stdout
/// for the CLI but also returning any information that is relevant to the command completion so
/// that it can also be returned to the GUI.
pub async fn run_snapshot_action(
    state: &State,
    testbed_snapshots: &TestbedSnapshots,
    snp_cmd: &SnapshotSubCommand,
    common: &OrchestrationCommon,
    logging_send: &Sender<OrchestrationLogger>,
) -> anyhow::Result<String> {
    let result_string = match &snp_cmd {
        SnapshotSubCommand::Create(create) => {
            if create.all {
                let group_snapshot_name = testbed_snapshots.snapshot_all_guests(common).await?;
                // reload testbed snapshots to show the new snapshots to user
                let testbed_snapshots = TestbedSnapshots::new(state, common).await?;
                for (guest_name, guest_data) in testbed_snapshots.guests {
                    if let Some(snap) = guest_data.snapshot.info_one(&group_snapshot_name) {
                        tracing::info!("{snap}");
                        logging_send.send(OrchestrationLogger::info(snap)).await?;
                    } else {
                        bail!("could not find {} for {}", &group_snapshot_name, guest_name);
                    }
                }
                "snapshots created".to_string()
            } else if create.name.is_some() && create.snapshot.is_some() {
                // both must be present, cli should prevent this not happening
                testbed_snapshots.create_snapshot(
                    create.name.as_ref().context("getting guest name in run snapshot action")?,
                    create.snapshot.as_ref().context("getting snapshot name in run snapshot action")?,
                    common,
                ).await?;
                "snapshot created".to_string()
            } else {
                bail!("must supply both the guest name and snapshot name");
            }
        }
        SnapshotSubCommand::Delete {name, snapshot, all } => {
            if *all {
                let name = name.as_ref()
                    .context("Getting guest name for delete command")?;
                testbed_snapshots.delete_all_snapshots(name, common).await?;
                "snapshot for all guests created".to_string()
            } else if name.is_some() && snapshot.is_some() {
                // both must be present, cli should prevent this not happening
                testbed_snapshots.delete_snapshot(
                    name.as_ref().context("getting guest name in delete snapshot action")?,
                    snapshot.as_ref().context("getting snapshot name in delete snapshot action")?,
                    common,
                ).await?;
                "snapshot deleted".to_string()
            } else {
                bail!("must supply both the guest name and snapshot name");
            }
        }
        SnapshotSubCommand::Info { name } => {
            // return the info string
            testbed_snapshots.info(name).await?
        }
        SnapshotSubCommand::List(list) => {
            if list.all {
                testbed_snapshots.list_all_snapshots().await?
            } else if let Some(guest) = &list.name {
                // need to make sure the guest is eligible for snapshots, right now it is only
                // libvirt guests that support snapshots
                match state.testbed_guests.0.get(guest).unwrap().guest_type.guest_type {
                    GuestType::Libvirt(_) => {}
                    _ => bail!("only libvirt guests support snapshots"),
                }

                testbed_snapshots.list_snapshots(guest).await?
            } else {
                bail!("no guest name specified");
            }
        }
        SnapshotSubCommand::Restore(restore) => {
            if restore.all {
                testbed_snapshots.restore_all_from_snapshots().await?;
                "snapshots restored".to_string()
            } else if restore.name.is_some() && restore.snapshot.is_some() {
                // both must be present, cli should prevent this not happening
                testbed_snapshots.restore_from_snapshot(
                    restore.name.as_ref().context("getting guest name in restore snapshot action")?,
                    restore.snapshot.as_ref().context("getting snapshot name in restore snapshot action")?,
                    common,
                ).await?;
                "snapshot restored".to_string()
            } else {
                bail!("must supply both the guest name and snapshot name");
            }
        }
    };

    Ok(result_string)
}
