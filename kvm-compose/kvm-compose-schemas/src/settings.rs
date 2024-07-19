use std::collections::HashMap;
use std::fmt::Formatter;
use tokio::fs::File;
use std::path::PathBuf;
use std::fmt;
use anyhow::bail;
use serde::{Deserialize, Serialize};
use tokio::io::AsyncWriteExt;
use crate::TESTBED_SETTINGS_FOLDER;

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
#[serde(rename_all = "snake_case")]
pub struct TestbedClusterConfig {
    /// this is a collection of all hosts in the cluster
    pub testbed_host_ssh_config: HashMap<String, SshConfig>,
    /// this is the ssh private key for guests, defaults to the insecure key
    #[serde(skip_deserializing)]
    pub ssh_public_key_location: String,
    /// this is the ssh public key for guests, defaults to the insecure key
    #[serde(skip_deserializing)]
    pub ssh_private_key_location: String,

}

/// This is the hosts configuration, it has to be filled in based on the host's environment. There
/// is not much possibility to automate filling in any of these values since it will be unique to
/// the user's intended configuration of the testbed, especially if using multiple testbed hosts.
#[derive(Deserialize, Serialize, Debug, Clone, Default)]
#[serde(rename_all = "snake_case")]
pub struct SshConfig {
    /// ip of the host in the local network, if using multiple testbed hosts
    pub ip: String,
    /// username for ssh
    pub user: String, // TODO - remove
    /// for multiple testbed hosts, this is the location of the private key for this host but on the
    /// main testbed, not locally - so the main knows how to connect to this host
    pub identity_file: String, // TODO - remove
    /// this is the local interface that can see the other testbed hosts in the cluster
    pub testbed_nic: String,
    /// this is the main interface with internet connectivity to allow guests to access the internet
    pub main_interface: String,
    /// must be set to Some(true) if this is the main tetsbed, otherwise will be regarded as a client host
    // TODO - with cluster management, we can default this value based on the mode.json
    pub is_main_host: Option<bool>,
    /// this is the OVN configuration for this host
    #[serde(default)]
    pub ovn: OvnConfig, // default settings are for main testbed
}

impl fmt::Display for SshConfig {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(&serde_json::to_string_pretty(&self).unwrap())
            .expect("SshConfig to json via serde failed");
        Ok(())
    }
}

impl SshConfig {
    pub async fn write(&self) -> anyhow::Result<()> {
        let mut output =
            File::create(format!("{TESTBED_SETTINGS_FOLDER}/config/host.json")).await?;
        output.write_all(format!("{self}").as_bytes()).await?;
        Ok(())
    }

    pub async fn read() -> anyhow::Result<SshConfig> {
        let name: PathBuf =
            PathBuf::from(format!("{TESTBED_SETTINGS_FOLDER}/config/host.json"));
        tracing::trace!("expected kvm compose config json location: {:?}", name);
        if name.is_file() {
            let text = tokio::fs::read_to_string(name).await?;
            let config: SshConfig = serde_json::from_str(&text)?;
            Ok(config)
        } else {
            bail!("could not read host.json - this needs to be set in /var/lib/testbedos/config/host.json")
        }
    }
}

/// This is the OVN config for the testbed host. All but the chassis name can have default values
/// that will work for main mode. When in client mode, the client_ovn_remote will be filled by
/// the main testbed as a response from joining the cluster. Therefore you only need to fill the
/// chassis_name to a unique value as a minimum configuration.
#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub struct OvnConfig {
    /// unique name for this host for OVN to know which host is what
    pub chassis_name: String,
    /// the integration bridge for OVN
    #[serde(default = "default_bridge")]
    pub bridge: String,
    /// the type of packet encapsulation for tunnels between chassis
    #[serde(default = "default_encap_type")]
    pub encap_type: String,
    /// the local ip endpoint for tunnel termination, must be the same as ip of the local network
    /// if host is a client in a cluster
    #[serde(default = "default_encap_ip")]
    pub encap_ip: String,
    /// this is the ip/socket to the local OVN southbound database
    #[serde(default = "default_main_ovn_remote")]
    pub main_ovn_remote: String,
    /// this is the ip of the remote OVN southbound database, to be used when this host is a client
    /// in a cluster
    #[serde(default)]
    pub client_ovn_remote: Option<String>, // for connecting to a main testbed ovn cluster
    /// this is a list of network to external bridge and the ip of external bridge
    /// i.e. public:br-ex where br-ex has ip 172.16.1.1/24
    #[serde(default = "default_bridge_mappings")]
    pub bridge_mappings: Vec<(String, String, String)>,
}

impl Default for OvnConfig {
    fn default() -> Self {
        Self {
            chassis_name: default_chassis_name(),
            bridge: default_bridge(),
            encap_type: default_encap_type(),
            encap_ip: default_encap_ip(),
            main_ovn_remote: default_main_ovn_remote(),
            client_ovn_remote: None,
            bridge_mappings: default_bridge_mappings(),
        }
    }
}

fn default_chassis_name() -> String {"ovn".to_string()}
fn default_bridge() -> String {"br-int".to_string()}
fn default_encap_type() -> String {"geneve".to_string()}
fn default_encap_ip() -> String {"127.0.0.1".to_string()}
fn default_main_ovn_remote() -> String {"unix:/usr/local/var/run/ovn/ovnsb_db.sock".to_string()}
fn default_bridge_mappings() -> Vec<(String, String, String)> {vec![("public".into(), "br-ex".into(), "172.16.1.1/24".into())]}

impl fmt::Display for TestbedClusterConfig {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(&serde_json::to_string_pretty(&self).unwrap())
            .expect("state to json via serde failed");
        Ok(())
    }
}

impl TestbedClusterConfig {
    pub async fn write(&self) -> anyhow::Result<()> {
        let mut output =
            File::create(format!("{TESTBED_SETTINGS_FOLDER}/config/kvm-compose-config.json")).await?;
        output.write_all(format!("{self}").as_bytes()).await?;
        Ok(())
    }

    pub async fn read() -> anyhow::Result<TestbedClusterConfig> {
        let name: PathBuf =
            PathBuf::from(format!("{TESTBED_SETTINGS_FOLDER}/config/kvm-compose-config.json"));
        tracing::trace!("expected kvm compose config json location: {:?}", name);
        if name.is_file() {
            let text = tokio::fs::read_to_string(name).await?;
            let mut config: TestbedClusterConfig = serde_json::from_str(&text)?;
            // temporarily inject the default keys, in the future we allow a different mechanism
            // for the user to specify their own guest SSH keys
            TestbedClusterConfig::insert_default_values(&mut config);
            Ok(config)
        } else {
            bail!("could not read kvm-compose-config.json")
        }
    }

    pub fn insert_default_values(tbcc: &mut TestbedClusterConfig) {
        tbcc.ssh_private_key_location = format!("{TESTBED_SETTINGS_FOLDER}/keys/id_ed25519_testbed_insecure_key");
        tbcc.ssh_public_key_location = format!("{TESTBED_SETTINGS_FOLDER}/keys/id_ed25519_testbed_insecure_key.pub");
    }
}
