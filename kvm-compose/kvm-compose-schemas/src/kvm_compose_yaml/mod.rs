pub mod machines;
pub mod network_old;
pub mod testbed_options;
pub mod tooling;
pub mod network;

use crate::kvm_compose_yaml::machines::*;
use crate::kvm_compose_yaml::network::*;
use crate::kvm_compose_yaml::testbed_options::*;
use crate::kvm_compose_yaml::tooling::*;
use anyhow::{Result, Context, Error};
use serde::{Deserialize, Serialize};
use tracing::{info, error};
use std::fmt;
use std::fmt::Formatter;
use std::path::Path;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Config {
    pub machines: Option<Vec<Machine>>,
    // #[serde(flatten)]
    pub network: NetworkBackend,
    pub tooling: Option<Tooling>,
    #[serde(default)]
    pub testbed_options: TestbedOptions,
}

impl Config {
    pub async fn load_from_file<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let text = tokio::fs::read_to_string(path).await.with_context(|| "Reading Config file")?;
        let value: Self = serde_yaml::from_str(&text).with_context(|| "Parsing Config YAML")?;
        // // TODO validate i.e. needs cpu and memory etc .. create validation code in other file
        value.validate().with_context(|| "Validating Config semantics")?;
        Ok(value)
    }

    pub fn validate(&self) -> Result<()> {
        info!("Running validation for Config");
        
        if let Some(machines) = &self.machines {
            for machine in machines {
                match &machine.guest_type {
                    GuestType::Libvirt(libvirt) => {
                        if libvirt.cpus.is_none() {
                            return Err(Error::msg("Machine is missing 'cpus' in libvirt configuration"));
                        }
                        if libvirt.memory_mb.is_none() {
                            return Err(Error::msg("Machine is missing 'memory_mb' in libvirt configuration"));
                        }
                    },
                    // Add validation for other guest types if needed, skip for now
                    _ => {},
                }
            }
        }

        Ok(())
    }

    pub async fn save_to<P: AsRef<Path>>(&self, path: P) -> anyhow::Result<()> {
        let mut file = File::create(path).await?;
        let to_string = serde_yaml::to_string(&self)?;
        file.write_all(&to_string.into_bytes()).await?;
        Ok(())
    }
}

impl fmt::Display for Config {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(&serde_json::to_string_pretty(&self).unwrap())
            .expect("state to json via serde failed");
        Ok(())
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Machine {
    pub name: String,
    pub network: Option<MachineNetwork>,
    // flatten means we dont need to specify "guest_type" and directly specify the GuestType variant
    #[serde(flatten)]
    pub guest_type: GuestType,
}

// #[derive(Deserialize, Serialize, Debug, Clone)]
// pub struct ConfigInterface {
//     pub bridge: String,
// }

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct MachineNetwork {
    pub switch: String,
    pub gateway: Option<String>,
    pub mac: String,
    pub ip: String,
    pub network_name: Option<String>, // provider network
}
