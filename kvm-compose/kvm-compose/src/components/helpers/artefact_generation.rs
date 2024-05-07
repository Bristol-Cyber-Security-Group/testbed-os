use std::path::PathBuf;
use anyhow::{bail, Context};
use std::os::unix::fs::PermissionsExt;
use tokio::process::Command;
use kvm_compose_schemas::cli_models::Common;
use crate::orchestration::OrchestrationCommon;


/// Copy a file from one location to another and set the permissions when done.
pub async fn copy_and_set_permissions(file_src: &PathBuf, file_tgt: &String, mode: u32, common: &Common) -> anyhow::Result<()> {
    tracing::info!(
        "Copying image from {} to {}",
        file_src.to_str().context("getting copy src path")?,
        file_tgt
    );
    tokio::fs::copy(file_src, &file_tgt)
        .await
        .context("Copying image to guest artefact folder")?;
    let mut perms = tokio::fs::metadata(&file_tgt)
        .await?
        .permissions();
    perms.set_readonly(false);
    perms.set_mode(mode);
    tokio::fs::set_permissions(&file_tgt, perms).await?;
    common.apply_user_file_perms(&PathBuf::from(file_tgt))?;
    Ok(())
}

pub async fn copy_and_set_permissions_orchestration(file_src: &PathBuf, file_tgt: &String, mode: u32, common: &OrchestrationCommon) -> anyhow::Result<()> {
    tracing::info!(
        "Copying image from {} to {}",
        file_src.to_str().context("getting copy src path")?,
        file_tgt
    );
    tokio::fs::copy(file_src, &file_tgt)
        .await
        .context("Copying image to guest artefact folder")?;
    let mut perms = tokio::fs::metadata(&file_tgt)
        .await?
        .permissions();
    perms.set_readonly(false);
    perms.set_mode(mode);
    tokio::fs::set_permissions(&file_tgt, perms).await?;
    common.apply_user_file_perms(&PathBuf::from(file_tgt))?;
    Ok(())
}

/// Resize a qcow2 image
pub async fn resize<T: AsRef<str>>(disk: T, disk_expand: u16) -> anyhow::Result<()> {
    let output = Command::new("qemu-img")
        .arg("resize")
        .arg(disk.as_ref())
        .arg(format!("+{disk_expand}G"))
        .output()
        .await?;
    if !output.status.success() {
        let std_err = std::str::from_utf8(&output.stderr)?;
        bail!("{}", std_err);
    }
    Ok(())
}
