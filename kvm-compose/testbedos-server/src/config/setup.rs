use std::process::Output;
use std::process::Stdio;
use serde_json::{Map, Value};
use tokio::process::Command;

/// Get the user and group in the qemu.conf file to see if libvirt is configured to allow images to
/// be used outside the libvirt images folder. This is necessary for libvirt guests in the
/// testbed as we place the images in the deployment folder.
pub async fn get_qemu_conf_user_and_group() -> anyhow::Result<(Option<String>, Option<String>)> {
    let user_grep = vec!["grep", "^user = \"", "/etc/libvirt/qemu.conf"];
    let group_grep = vec!["grep", "^group = \"", "/etc/libvirt/qemu.conf"];

    let user_command = Command::new("sudo")
        .args(user_grep)
        .stdout(Stdio::piped())
        .spawn()?
        .wait_with_output()
        .await?;

    let std_out_user = String::from_utf8(user_command.stdout)?;
    // let std_err = String::from_utf8(user_command.stderr)?;
    // println!("stdout: {std_out_user:?}");
    // println!("stderr: {std_err:?}");

    let group_command = Command::new("sudo")
        .args(group_grep)
        .stdout(Stdio::piped())
        .spawn()?
        .wait_with_output()
        .await?;

    let std_out_group = String::from_utf8(group_command.stdout)?;
    // let std_err = String::from_utf8(group_command.stderr)?;
    // println!("stdout: {std_out_group:?}");
    // println!("stderr: {std_err:?}");

    // if the stdout is empty string, then we didn't match and therefore is commented out.
    // this is because we are matching from the start of the string, so if the line is commented
    // out (which is the default), then the grep won't match

    let characters_to_remove = ['\\', '\n'];
    let remove_fn = |&c: &char| !characters_to_remove.contains(&c);

    let user = if std_out_user.eq("") {
        None
    } else {
        Some(std_out_user.chars().filter(remove_fn).collect())
    };
    let group = if std_out_group.eq("") {
        None
    } else {
        Some(std_out_group.chars().filter(remove_fn).collect())
    };

    Ok((user, group))
}

/// Set the user and group in the qemu.conf file. See `get_qemu_conf_user_and_group`.
pub async fn set_qemu_conf_user_and_group() {
    // use `sed` to replace the line in the file
    todo!()
}

/// Trigger a libvirt daemon restart, to be used after using `set_qemu_conf_user_and_group`
pub async fn restart_libvirt() {
    todo!()
}

/// Get the top nameserver in the resolv.conf to determine if the Android emulator will have working
/// DNS. If set to `127.0.0.53` the emulator is unlikely to work.
pub async fn get_host_resolv_conf_top_nameserver() {
    todo!()
}

/// Check the docker daemon to see if the resource monitoring stack is running.
pub async fn get_resource_monitoring_state() -> anyhow::Result<Map<String, Value>> {
    // TODO - this should ideally use the docker service socket connection rather than subprocess

    let grafana_cmd = vec!["docker", "inspect", "-f", "{{.State.Running}}", "resource_monitoring-grafana-1"];
    let prometheus_cmd = vec!["docker", "inspect", "-f", "{{.State.Running}}", "resource_monitoring-prometheus-1"];
    let nginx_cmd = vec!["docker", "inspect", "-f", "{{.State.Running}}", "resource_monitoring-proxy-1"];

    let grafana_out = Command::new("sudo")
        .args(grafana_cmd)
        .stdout(Stdio::piped())
        .spawn()?
        .wait_with_output()
        .await?;

    let prometheus_out = Command::new("sudo")
        .args(prometheus_cmd)
        .stdout(Stdio::piped())
        .spawn()?
        .wait_with_output()
        .await?;

    let nginx_out = Command::new("sudo")
        .args(nginx_cmd)
        .stdout(Stdio::piped())
        .spawn()?
        .wait_with_output()
        .await?;

    let get_container_state: fn(Output) -> anyhow::Result<bool> = |cmd_output: Output| {
        // set up closure to strip unwanted characters
        let characters_to_remove = ['\n'];
        let remove_fn = |&c: &char| !characters_to_remove.contains(&c);
        if cmd_output.status.success() {
            let out: String = String::from_utf8(cmd_output.stdout)?;
            // strip unwanted characters
            let out_escaped: Vec<_> = out.chars().filter(remove_fn).collect();
            let out_to_string: String = out_escaped.iter().collect();
            if out_to_string.eq("true") {
                Ok(true)
            } else {
                Ok(false)
            }
        } else {
            Ok(false)
        }
    };
    let grafana_running = get_container_state(grafana_out)?;
    let prometheus_running = get_container_state(prometheus_out)?;
    let nginx_running = get_container_state(nginx_out)?;

    let mut json = serde_json::Map::new();
    json.insert("grafana".to_string(), Value::Bool(grafana_running));
    json.insert("prometheus".to_string(), Value::Bool(prometheus_running));
    json.insert("nginx".to_string(), Value::Bool(nginx_running));

    Ok(json)
}

/// Change the state of the resource monitoring stack.
pub async fn toggle_resource_monitoring_state() {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    // These tests need sudo, so will disable these from the general test suite.
    // Otherwise, they will likely always fail.

    // #[tokio::test]
    // async fn test_get_qemu_conf_user_and_group() -> anyhow::Result<()> {
    //     let res = get_qemu_conf_user_and_group().await?;
    //     println!("{res:?}");
    //     Ok(())
    // }

}
