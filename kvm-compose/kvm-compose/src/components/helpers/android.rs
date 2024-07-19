use std::process::Stdio;
use std::process::Command;
use anyhow::{bail, Context};
use kvm_compose_schemas::kvm_compose_yaml::machines::avd::AVDGuestOptions;


/// Check if the parameters for the image exists on disk based on the output from `get_sdk_string`.
/// If the sdk already exists it will just continue.
pub fn download_system_image(
    sdk_string: &String,
) -> anyhow::Result<()> {
    tracing::info!("making sure the system image exists '{sdk_string}', downloading if it doesn't (can be slow)");

    // TODO - look in the filesystem first, then if not there log to user that were going to
    //  download the image?

    // need to send a confirm message to the download `echo y |`
    let echo_yes_command = Command::new("echo")
        .arg("y")
        .stdout(Stdio::piped())
        .spawn()?;
    let yes_stdout = echo_yes_command.stdout
        .context("getting the 'echo y' pipe")?;

    // TODO - is there a way to stream the download log to the user?
    let output = Command::new("sudo")
        .arg("/opt/android-sdk/cmdline-tools/latest/bin/sdkmanager")
        .arg("--install")
        .arg(sdk_string)
        .stdin(Stdio::from(yes_stdout))
        .stdout(Stdio::piped())
        .spawn()
        .with_context(|| format!("downloading avd system image '{sdk_string}'"))?;
    output.wait_with_output()?;

    Ok(())
}

/// Get the string for the `sdkmanager` based on the `AVDGuestOptions`
pub fn get_sdk_string(
    avdguest_options: &AVDGuestOptions,
) -> anyhow::Result<String> {

    // always start with system images
    let mut package_string = "system-images;".to_string();

    match avdguest_options {
        AVDGuestOptions::Avd { android_api_version, playstore_enabled } => {
            package_string.push_str(&format!("android-{android_api_version};"));

            if *playstore_enabled {
                package_string.push_str("google_apis_playstore;");
            } else {
                package_string.push_str("google_apis;");
            }

            // always use x86 for now
            package_string.push_str("x86");

        }
        AVDGuestOptions::ExistingAvd { .. } => bail!("Existing AVD not yet implemented"),
    }

    Ok(package_string)
}

/// This created the AVD image, assuming the system image has already been downloaded otherwise this
/// will fail
pub fn create_avd(
    avd_name: &String,
    build_avd_command: Vec<&str>,
) -> anyhow::Result<()> {
    tracing::info!("creating avd {avd_name}");

    // we need to pipe the no to setting up a hardware config
    let echo_no_command = Command::new("echo")
        .arg("no")
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();

    // we pipe in the "echo no" to the start of the command like:
    // echo no | avd create ...
    let output = Command::new("sudo")
        .args(build_avd_command)
        .stdin(Stdio::from(echo_no_command.stdout.unwrap()))
        .stdout(Stdio::piped())
        .spawn()
        .with_context(|| format!("creating avd {avd_name}"))?;
    let cmd_output = output.wait_with_output()?;
    tracing::info!("create avd log: {cmd_output:?}");
    if !cmd_output.status.success() {
        // nothing useful in err, goes to out
        let std_out = std::str::from_utf8(&cmd_output.stdout)?;
        bail!("could not create avd {avd_name}, error {std_out:#}");
    }
    Ok(())
}
