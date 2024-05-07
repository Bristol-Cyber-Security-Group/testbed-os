use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use validator::Validate;
use crate::kvm_compose_yaml::machines::ConfigScalingInterface;

/// This replicates a minimum set of options from docker-compose. The options here will generally
/// be copied as is, as arguments to the docker run command. There is some reliance on the user
/// knowing the format from the following docker documentation:
/// https://docs.docker.com/engine/reference/commandline/run/
/// https://docs.docker.com/compose/compose-file/05-services/
///
/// However, where there are multiple ways to supply the format in docker-compose, we only support
/// one format.
/// As more arguments are needed, they can be added here and implemented in `create_artefact` for
/// `DockerGuest`.
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ConfigDockerMachine {
    pub image: String,
    pub command: Option<String>,
    pub entrypoint: Option<String>,
    pub environment: Option<BTreeMap<String, String>>,
    pub env_file: Option<String>,
    pub volumes: Option<Vec<Volume>>,
    pub privileged: Option<bool>,
    pub scaling: Option<DockerScaling>,
    pub user: Option<String>,
    pub device: Option<Vec<String>>,
    #[serde(skip_deserializing)]
    pub hostname: String,

    // TODO depends on is a useful feature of docker-compose that users may want here, we should
    //  be able to implement this ourselves by making checks in the orchestration or if docker cli
    //  gives this as an option. Also may be useful to make depends on available to other guest type
    // pub depends_on: _

    pub static_ip: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Volume {
    // TODO - support docker notation for read only ":ro" etc, volume drivers etc
    pub source: String,
    pub target: String,
}

#[derive(Deserialize, Serialize, Debug, Validate, Clone)]
pub struct DockerScaling {
    #[validate(range(min = 1))]
    pub count: u32,
    #[validate(length(min = 1))]
    pub interfaces: HashMap<String, ConfigScalingInterface>,
}
