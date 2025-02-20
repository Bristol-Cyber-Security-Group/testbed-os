use anyhow::{bail, Context};
use tempfile::NamedTempFile;
use tokio::sync::mpsc::Sender;
use virt::connect::Connect;
use kvm_compose_schemas::exec::ExecCmdFileTransfer;
use crate::components::helpers::serialisation;
use crate::exec::libvirt::shell_command;
use crate::orchestration::api::OrchestrationLogger;
use crate::orchestration::{run_subprocess_command, OrchestrationCommon};
use crate::state::StateTestbedGuest;

const CDROM_DEVICE_XML: &str = r#"
    <disk type='file' device='cdrom'>
        <driver name='qemu' type='raw'/>
        <target dev='sdc' bus='scsi'/>
        <readonly/>
        <address type='drive' controller='0' bus='0' target='0' unit='2'/>
    </disk>
    "#;

/// The file or folder to be pushed needs to be prepared into an iso image, to be mounted as a
/// CD ROM into the guest. The iso will be placed in the tmp folder of the system before being
/// mounted into the guest.
pub async fn prepare_file_transfer_push(
    transfer: &ExecCmdFileTransfer,
    common: &OrchestrationCommon,
    logging_send: &Sender<OrchestrationLogger>,
) -> anyhow::Result<NamedTempFile> {
    logging_send.send(OrchestrationLogger::info("Preparing file or folder to be pushed".to_string())).await?;

    // create a temporary file with a unique name, which will become our ISO
    let temp_iso_location = NamedTempFile::new()?;

    // works for both single file and a folder, folder structure will be preserved
    serialisation::genisoimage_orchestration(
        &temp_iso_location.path(),
        vec![transfer.source_path.clone()],
        common
    )
        .await
        .context("Could not create file push iso")?;

    // return the temp file so that we hold a reference to prevent drop being called causing it to
    // delete itself from the filesystem
    Ok(temp_iso_location)
}

/// The CD ROM must be attached to the guest via virsh commands
pub async fn attach_cdrom_to_guest(
    guest_name_with_project: &String,
    temp_iso: &NamedTempFile,
    // common: &OrchestrationCommon,
    logging_send: &Sender<OrchestrationLogger>,
) -> anyhow::Result<()> {
    logging_send.send(OrchestrationLogger::info("Attaching CD ROM to guest".to_string())).await?;

    // the guest has an SCSI controller, we can attach a CD rom
    // TODO - if one already attached, will it error?

    // use the libvirt library to directly edit the guest domain definition
    let conn = Connect::open(Some("qemu:///system"))
        .context("connecting to libvirt to get attach CD ROM device")?;
    let domain = virt::domain::Domain::lookup_by_name(&conn, guest_name_with_project)
        .context("getting domain from libvirt connection")?;
    // insert the scsi CD ROM device via an XML definition (will not persist between guest boots)
    domain.attach_device(CDROM_DEVICE_XML)
        .context("Attaching CD ROM device to guest")?;

    // attach the media
    let temp_iso_str_path = temp_iso.path().to_string_lossy().into_owned();
    run_subprocess_command(
        "sudo",
        vec!["virsh", "change-media", &guest_name_with_project, "sdc", &temp_iso_str_path],
        false,
        None,
    ).await?;

    Ok(())
}

/// The CD ROM, once attached to the guest, needs to be mounted from inside the guest. This also
/// works out which device the CD ROM just attached is available under.
pub async fn mount_cdrom_in_guest(
    guest_name_with_project: &String,
    guest_data: &StateTestbedGuest,
    common: &OrchestrationCommon,
    logging_send: &Sender<OrchestrationLogger>,
) -> anyhow::Result<()> {
    logging_send.send(OrchestrationLogger::info("Mounting CD ROM in guest".to_string())).await?;
    // We need to know what is the device in the guest before we can mount it from inside the guest.
    // There may be other cd rom devices attached, which will show up under srX i.e. if cloud init
    // then that disk will be sr0. Any further disks attached will increment this value. However,
    // once there has been a CD ROM attached once, and this is a second file push, we will need to
    // re-use the previous device. In the meanwhile, the user may have attached another unrelated
    // device and incremented the srX count.

    // we know our SCSI cdrom device has these parameters controller='0' bus='0' target='0' unit='2'
    // meaning it should be named scsi0-0-0-2
    // meaning if we look at `ls -l /dev/disk/by-id/` it should return something like:
    // scsi-0QEMU_QEMU_CD-ROM_drive-scsi0-0-0-2 -> ../../sr1
    // where sr1 happens to be the /dev/sr1 device and this should remain the same
    // .. so this is a symlink, we can do `readlink -f /dev/disk/by-id/*scsi0-0-0-2` to get the dev
    // .. then we can mount with `mount -o ro /dev/sr1 /mnt/filepush`

    // given the steps above, first get the dev
    let (dev, exit_code) = shell_command(
        vec!["readlink", "-f", "/dev/disk/by-id/*scsi0-0-0-2"],
        5_000,
        guest_data,
        guest_name_with_project,
        common,
        logging_send,
    ).await?;

    logging_send.send(OrchestrationLogger::info(format!("Getting device output:\n{dev}"))).await?;

    if exit_code != 0 {
        bail!("Getting CD ROM device in guest gave exit code {exit_code}, cannot continue");
    }

    // create the mount point before we mount the iso, make sure not to error if it exists
    let _ = shell_command(
        vec!["sudo", "mkdir", "-p", "/mnt/filepush"],
        5_000,
        guest_data,
        guest_name_with_project,
        common,
        logging_send,
    ).await?;

    // then mount the dev to an intermediate location
    let (output, exit_code) = shell_command(
        vec!["sudo", "mount", "-o", "ro", &dev, "/mnt/filepush"],
        5_000,
        guest_data,
        guest_name_with_project,
        common,
        logging_send,
    ).await?;

    if exit_code != 0 {
        bail!("Mount CD ROM failed with exit code {exit_code}, output:\n{output}");
    }

    logging_send.send(OrchestrationLogger::info("Mounting CD ROM in guest OK".to_string())).await?;

    Ok(())
}

/// The user will specify where they want the file to be pushed to in the guest, this will move that
/// to the specified location
pub async fn move_pushed_file(
    transfer: &ExecCmdFileTransfer,
    guest_name_with_project: &String,
    guest_data: &StateTestbedGuest,
    common: &OrchestrationCommon,
    logging_send: &Sender<OrchestrationLogger>,
) -> anyhow::Result<()> {
    logging_send.send(OrchestrationLogger::info("Moving file or folder into desired location".to_string())).await?;

    // in case the target destination doesn't exist
    let _ = shell_command(
        vec!["mkdir", "-p", &transfer.target_path.display().to_string()],
        5_000,
        guest_data,
        guest_name_with_project,
        common,
        logging_send,
    ).await?;

    // use the destination provided by the user to move the file or folder from the mounted ISO to
    // the target location
    let (output, exit_code) = shell_command(
        vec!["cp", "-r", "/mnt/filepush/.", &transfer.target_path.display().to_string()],
        5_000,
        guest_data,
        guest_name_with_project,
        common,
        logging_send,
    ).await?;

    if exit_code != 0 {
        logging_send.send(OrchestrationLogger::error(format!("Could not move file or folder from ISO to target location with exit code {exit_code}, and error:\n{output}"))).await?;
    }

    Ok(())
}

/// The CD ROM can then be unmounted from the disk then detached from the guest once we have
/// finished using it.
pub async fn unmount_and_detach_cdrom_from_guest(
    guest_name_with_project: &String,
    guest_data: &StateTestbedGuest,
    common: &OrchestrationCommon,
    logging_send: &Sender<OrchestrationLogger>,
) -> anyhow::Result<()> {
    logging_send.send(OrchestrationLogger::info("Unmounting and detaching the CD ROM".to_string())).await?;

    // first unmount
    let (output, exit_code) = shell_command(
        vec!["sudo", "umount", "/mnt/filepush"],
        5_000,
        guest_data,
        guest_name_with_project,
        common,
        logging_send,
    ).await?;

    if exit_code != 0 {
        logging_send.send(OrchestrationLogger::error(format!("There was a problem unmounting the ISO with code {exit_code}, and error:\n{output}"))).await?;
    }

    // then detatch the device via libvirt
    let conn = Connect::open(Some("qemu:///system"))
        .context("connecting to libvirt to get detach CD ROM device")?;
    let domain = virt::domain::Domain::lookup_by_name(&conn, guest_name_with_project)
        .context("getting domain from libvirt connection")?;
    // insert the scsi CD ROM device via an XML definition (will not persist between guest boots)
    domain.detach_device(CDROM_DEVICE_XML)
        .context("Detaching CD ROM device from guest")?;

    Ok(())
}
