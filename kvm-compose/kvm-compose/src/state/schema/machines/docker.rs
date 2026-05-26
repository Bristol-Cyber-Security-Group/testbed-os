use std::collections::{BTreeMap, HashMap};
use serde::{Deserialize, Serialize};
use validator::Validate;
use kvm_compose_schemas::kvm_compose_yaml::machines::docker::{ConfigDockerMachine, DockerScaling, Volume};
use crate::state::schema::machines::StateScalingInterface;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct StateDockerMachine {
    pub image: String,
    pub command: Option<String>,
    pub entrypoint: Option<String>,
    pub environment: Option<BTreeMap<String, String>>,
    pub env_file: Option<String>,
    pub volumes: Option<Vec<Volume>>,
    pub privileged: Option<bool>,
    pub scaling: Option<StateDockerScaling>,
    pub user: Option<String>,
    pub device: Option<Vec<String>>,
    pub static_ip: Option<String>,
}

impl From<ConfigDockerMachine> for StateDockerMachine {
    fn from(docker_machine: ConfigDockerMachine) -> Self {
        Self {
            image: docker_machine.image,
            command: docker_machine.command,
            entrypoint: docker_machine.entrypoint,
            environment: docker_machine.environment,
            env_file: docker_machine.env_file,
            volumes: docker_machine.volumes,
            privileged: docker_machine.privileged,
            scaling: docker_machine.scaling.map(|s| s.into()),
            user: docker_machine.user,
            device: docker_machine.device,
            static_ip: docker_machine.static_ip,
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Validate, Clone)]
pub struct StateDockerScaling {
    #[validate(range(min = 1))]
    pub count: u32,
    #[validate(length(min = 1))]
    pub interfaces: HashMap<String, StateScalingInterface>,
}

impl From<DockerScaling> for StateDockerScaling {
    fn from(scaling: DockerScaling) -> Self {
        Self {
            count: scaling.count,
            interfaces: scaling.interfaces.into_iter()
                .map(|(k, v)| (k, v.into()))
                .collect(),
        }
    }
}
