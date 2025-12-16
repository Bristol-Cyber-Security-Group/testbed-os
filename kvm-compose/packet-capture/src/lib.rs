use std::path::PathBuf;
use anyhow::{bail, Context};
use rand::distr::Distribution;
use crate::interface::OVSConfig;

mod interface;
mod capture;

pub struct TestbedPacketCapture;

impl TestbedPacketCapture {
    pub async fn capture(
        &self,
        config: TCPDumpConfig,
        stop_rx: tokio::sync::oneshot::Receiver<()>,
    ) -> anyhow::Result<()> {
        match capture::packet_capture(&config, stop_rx).await {
            Ok(_) => {}
            Err(err) => {
                tracing::warn!("forcing cleanup due to setup error");
                OVSConfig::teardown(&config).await
                    .context("tearing down mirror port infrastructure due to error in setup")?;
                anyhow::bail!(err)
            }
        }
        Ok(())
    }
}

pub struct TCPDumpConfig {

    /// OVS port or interface to capture on
    pub interface: String,

    /// Whether to mirror all traffic on the bridge
    pub span: bool,

    /// Optional arguments to be passed to `tcpdump`
    pub dump_args: Vec<String>,

    /// Generated OS interface to receive mirrored traffic
    pub mirror_interface: String,

    /// Generated OVS mirror port name
    pub mirror_name: String,

    /// Name of OVS bridge
    pub ovs_bridge: String,

    /// Specify which consumer type the captured packet data should go to
    pub consumer: TCPDumpConsumer,
}

impl TCPDumpConfig {

    pub async fn new(
        interface: String,
        span: bool,
        dump_args: Vec<String>,
        consumer: TCPDumpConsumer,
    ) -> anyhow::Result<Self> {
        let mirror_interface = Self::generate_interface_name(&interface)
            .await?;
        let mirror_name = Self::generate_mirror_name(&interface, &mirror_interface);
        let new = Self {
            interface,
            span,
            dump_args,
            mirror_interface,
            mirror_name,
            ovs_bridge: "br-int".to_string(),
            consumer,
        };

        new
            .validate()
            .await?;

        Ok(new)
    }

    async fn generate_interface_name(in_name: &str) -> anyhow::Result<String> {
        // we will try to generate an interface name up to 10 times, if for some reason this
        // collides this many times, something is seriously wrong, and we should defer to the user,
        // since we should not keep trying until this works to prevent an infinite loop

        for _ in 0..10 {
            let maybe_name = generate_name(&in_name);

            // check if there was a collision, if there is, this will Err and will loop again
            if let Ok(_) = tokio::process::Command::new("sudo")
                .arg("ip")
                .arg("link")
                .arg("show")
                .arg("dev")
                .arg(&maybe_name)
                .output()
                .await
            {
                // exit loop early, the name does not collide
                return Ok(maybe_name);
            }

        }

        bail!("could not generate an interface name without a collision 10 times")
    }

    fn generate_mirror_name(in_name: &str, os_interface: &str) -> String {
        format!("mirror-{in_name}-to-{os_interface}")
    }

    async fn validate(&self) -> anyhow::Result<()> {

        // TODO - what if the interface/port is on a remote testbed

        // check interface exists, this is the ovs side, and error if it doesn't
        tokio::process::Command::new("sudo")
            .arg("ovs-vsctl")
            .arg("--id=@target")
            .arg("get")
            .arg("port")
            .arg(&self.interface)
            .output()
            .await
            .context("Getting existing interface/port")?;

        // check mirror doesn't already exist
        if tokio::process::Command::new("sudo")
            .arg("ovs-vsctl")
            .arg("--id=@target")
            .arg("get")
            .arg("mirror")
            .arg(&self.mirror_interface)
            .output()
            .await
            .context("Getting exiting mirror")?
            .status
            .success()
        {
            bail!("Mirror port {} already exists", &self.mirror_interface);
        }

        Ok(())
    }
}

/// Enum to define the different consumers to output the packet capture data to
pub enum TCPDumpConsumer {
    /// Path to create a pcap file
    File(PathBuf),

    // TODO other consumers i.e. databases over the network
}

fn generate_name(in_name: &str) -> String {
    // this is the name of the OS interface, this can only be 15 characters max
    let mut os_interface_name = format!("mi{in_name}");
    // truncate by 6 if greater than 10 characters
    if os_interface_name.len() > 10 {
        os_interface_name = os_interface_name[0..10].to_string();
    }
    // add a unique id to the end to fill the space ... do 14 instead of 15 so that we can put
    // in a hyphen between the text and the unique id
    let needed_characters = 14 - os_interface_name.len();
    let mut rng = rand::rng();
    let suffix = rand::distr::Alphanumeric
        .sample_iter(&mut rng)
        .take(needed_characters)
        .map(char::from)
        .collect::<String>();

    format!("{}-{}", os_interface_name, suffix)
}

#[cfg(test)]
mod tests {
    use crate::generate_name;

    #[test]
    fn test_generate_interface_name_length_constraint_short_input() {
        let test = generate_name("short");
        assert!(test.len() <= 15);
    }

    #[test]
    fn test_generate_interface_name_length_constraint_long_input() {
        let test = generate_name("alonginputnameoverfifteen");
        assert!(test.len() <= 15);
    }

}
