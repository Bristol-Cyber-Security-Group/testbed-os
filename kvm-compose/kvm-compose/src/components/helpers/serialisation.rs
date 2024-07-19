use std::path::{Path, PathBuf};
use tokio::fs::File;
use tokio::process::Command;
use anyhow::{bail, Context};
use std::os::unix::fs::PermissionsExt;
use nix::unistd::{Gid, Uid};
use tokio::io::AsyncWriteExt;
use kvm_compose_schemas::cli_models::Common;
use crate::orchestration::OrchestrationCommon;

/// Helper method to write artefact generation files in the correct folder with correct permissions
pub async fn write_file_with_permissions(
    dest_string: String,
    file_contents: String,
    permissions: u32,
    uid: Uid,
    gid: Gid,
) -> anyhow::Result<PathBuf> {
    let dest_path = PathBuf::from(dest_string);
    let mut output = File::create(dest_path.to_str().context("Getting destination path")?).await?;
    output.write_all(file_contents.to_string().as_bytes()).await?;
    let mut perms = tokio::fs::metadata(&dest_path).await?.permissions();
    perms.set_mode(permissions);
    tokio::fs::set_permissions(&dest_path, perms).await?;
    nix::unistd::chown(&dest_path, Some(uid), Some(gid))?;
    Ok(dest_path)
}

/// Helper method to tar together the cloud init input files
pub async fn genisoimage(output: &Path, inputs: Vec<PathBuf>, common: &Common) -> anyhow::Result<()> {
    if output.exists() {
        tracing::trace!("replacing genisoimage");
        tokio::fs::remove_file(output).await?;
    }
    let mut cmd = Command::new("genisoimage");
    cmd.arg("-output")
        .arg(output.to_str().context("getting output path for genisoimage")?)
        .arg("-volid")
        .arg("cidata")
        .arg("-joliet")
        .arg("-rock");

    for i in inputs {
        cmd.arg(i.to_str().context("converting args to string for genisoimage")?);
    }

    let cmd_output = cmd.output()
        .await
        .with_context(|| "writing iso image")?;
    if !cmd_output.status.success() {
        let std_err = std::str::from_utf8(&cmd_output.stderr)?;
        bail!("{}", std_err);
    }

    let mut iso_perms = tokio::fs::metadata(&output).await?.permissions();
    iso_perms.set_readonly(true);
    iso_perms.set_mode(0o755);
    tokio::fs::set_permissions(&output, iso_perms).await?;
    common.apply_user_file_perms(&PathBuf::from(output))?;
    Ok(())
}

pub async fn genisoimage_orchestration(output: &Path, inputs: Vec<PathBuf>, common: &OrchestrationCommon) -> anyhow::Result<()> {
    if output.exists() {
        tracing::trace!("replacing genisoimage");
        tokio::fs::remove_file(output).await?;
    }
    let mut cmd = Command::new("genisoimage");
    cmd.arg("-output")
        .arg(output.to_str().context("getting output path for genisoimage")?)
        .arg("-volid")
        .arg("cidata")
        .arg("-joliet")
        .arg("-rock");

    for i in inputs {
        cmd.arg(i.to_str().context("converting args to string for genisoimage")?);
    }

    let cmd_output = cmd.output()
        .await
        .with_context(|| "writing iso image")?;
    if !cmd_output.status.success() {
        let std_err = std::str::from_utf8(&cmd_output.stderr)?;
        bail!("{}", std_err);
    }

    let mut iso_perms = tokio::fs::metadata(&output).await?.permissions();
    iso_perms.set_readonly(true);
    iso_perms.set_mode(0o755);
    tokio::fs::set_permissions(&output, iso_perms).await?;
    common.apply_user_file_perms(&PathBuf::from(output))?;
    Ok(())
}

/// Helper to tar the use specified context files for cloud-init
pub async fn tar_cf(output: &Path, input: &Path) -> anyhow::Result<()> {
    let mut command = Command::new("tar");
    command.arg("cf").arg(output.to_str().context("converting output path to string for tar_cf")?);
    if input.is_dir() {
        command.arg(".").current_dir(input)
    } else {
        command
            .arg(input.file_name().context("getting filename for tar_cf input")?)
            .current_dir(input.parent().context("getting parent folder for tar_cf")?)
    };
    let output = command.output().await?;
    if !output.status.success() {
        let std_err = std::str::from_utf8(&output.stderr)?;
        bail!("{}", std_err);
    }
    Ok(())
}

/// Helper method to write artefact generation files in the correct folder with correct permissions
pub async fn write_file_vecu8_with_permissions(
    dest_string: String,
    file_contents: Vec<u8>,
    permissions: u32,
    common: &Common,
) -> anyhow::Result<PathBuf> {
    let dest_path = PathBuf::from(dest_string);
    let mut output = File::create(dest_path.to_str().context("getting path string to write with permissions")?).await?;
    output.write_all(&file_contents).await?;
    let mut perms = tokio::fs::metadata(&dest_path).await?.permissions();
    perms.set_mode(permissions);
    tokio::fs::set_permissions(&dest_path, perms).await?;
    common.apply_user_file_perms(&dest_path)?;
    Ok(dest_path)
}

pub async fn write_file_vecu8_with_permissions_orchestration(
    dest_string: String,
    file_contents: Vec<u8>,
    permissions: u32,
    common: &OrchestrationCommon,
) -> anyhow::Result<PathBuf> {
    let dest_path = PathBuf::from(dest_string);
    let mut output = File::create(dest_path.to_str().context("getting path string to write with permissions")?).await?;
    output.write_all(&file_contents).await?;
    let mut perms = tokio::fs::metadata(&dest_path).await?.permissions();
    perms.set_mode(permissions);
    tokio::fs::set_permissions(&dest_path, perms).await?;
    common.apply_user_file_perms(&dest_path)?;
    Ok(dest_path)
}
