use anyhow::Context;
use crate::TCPDumpConfig;

pub struct OVSConfig {

}

impl OVSConfig {
    pub async fn setup(
        tcpdump_config: &TCPDumpConfig,
    ) -> anyhow::Result<String> {

        // create dummy interface
        tracing::info!("create OS dummy interface {}", tcpdump_config.mirror_interface);
        tokio::process::Command::new("sudo")
            .arg("ip")
            .arg("link")
            .arg("add")
            .arg(&tcpdump_config.mirror_interface)
            .arg("type")
            .arg("dummy")
            .output()
            .await
            .context("Creating dummy OS interface")?;

        // enable interface
        tracing::info!("set up OS dummy interface {}", tcpdump_config.mirror_interface);
        tokio::process::Command::new("sudo")
            .arg("ip")
            .arg("link")
            .arg("set")
            .arg("dev")
            .arg(&tcpdump_config.mirror_interface)
            .arg("up")
            .output()
            .await
            .context("Setting dummy OS interface up")?;

        // add a port to this new interface on the ovs bridge
        tracing::info!("set up OVS port for OS dummy interface {}", tcpdump_config.mirror_interface);
        tokio::process::Command::new("sudo")
            .arg("ovs-vsctl")
            .arg("add-port")
            .arg(&tcpdump_config.ovs_bridge)
            .arg(&tcpdump_config.mirror_interface)
            .output()
            .await
            .context("Creating OVS port")?;

        // TODO - add any filters

        // create mirror
        tracing::info!("set up OVS port mirror for OS dummy interface {}", tcpdump_config.mirror_interface);
        tokio::process::Command::new("sudo")
            .arg("ovs-vsctl")
            .arg("--")
            .arg("--id=@target")
            .arg("get")
            .arg("port")
            .arg(&tcpdump_config.interface)

            .arg("--")
            .arg("--id=@mirror_port")
            .arg("get")
            .arg("port")
            .arg(&tcpdump_config.mirror_interface)

            .arg("--")
            .arg("--id=@m")
            .arg("create")
            .arg("mirror")
            .arg(format!("name={}", tcpdump_config.mirror_name))
            .arg(format!("select-all={}", tcpdump_config.span.to_string()))
            .arg("select-src-port=@target")
            .arg("select-dst-port=@target")
            .arg("output-port=@mirror_port")

            .arg("--")
            .arg("set")
            .arg("bridge")
            .arg(&tcpdump_config.ovs_bridge)
            .arg("mirrors=@m")
            .output()
            .await
            .context("Creating OVS mirror port configuration")?;


        // TODO if using span, apply span option

        Ok(tcpdump_config.mirror_interface.to_string())
    }

    pub async fn teardown(
        tcpdump_config: &TCPDumpConfig,
    ) -> anyhow::Result<()> {

        // remove mirror
        tracing::info!("clear OVS mirror port for {}", tcpdump_config.mirror_interface);
        tokio::process::Command::new("sudo")
            .arg("ovs-vsctl")
            .arg("clear")
            .arg("bridge")
            .arg(&tcpdump_config.ovs_bridge)
            .arg("mirrors") // TODO be specific which mirror
            .output()
            .await
            .context("Clearing OVS bridge mirror")?;

        // delete port
        tracing::info!("delete OVS port for {}", tcpdump_config.mirror_interface);
        tokio::process::Command::new("sudo")
            .arg("ovs-vsctl")
            .arg("--if-exists")
            .arg("del-port")
            .arg(&tcpdump_config.ovs_bridge)
            .arg(&tcpdump_config.mirror_interface)
            .output()
            .await
            .context("Deleting OVS port")?;

        // delete dummy interface
        tracing::info!("delete OS dummy interface {}", tcpdump_config.mirror_interface);
        tokio::process::Command::new("sudo")
            .arg("ip")
            .arg("link")
            .arg("del")
            .arg(&tcpdump_config.mirror_interface)
            .output()
            .await
            .context("Deleting OS dummy interface")?;

        Ok(())
    }
}

