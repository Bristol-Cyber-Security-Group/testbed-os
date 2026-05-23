use serde::{Deserialize, Serialize};
use kvm_compose_schemas::kvm_compose_yaml::{Machine, MachineNetwork};
use kvm_compose_schemas::kvm_compose_yaml::machines::GuestType;
use crate::state::schema::machines::avd::StateAVDMachine;
use crate::state::schema::machines::docker::StateDockerMachine;
use crate::state::schema::machines::libvirt::StateLibvirtMachine;

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub struct StateTestbedGuest {
    #[serde(flatten)]
    pub guest_type: StateMachine,
    pub testbed_host: Option<String>,
    pub is_golden_image: bool,
    /// this is a unique identifier for the guest in the state
    pub guest_id: u32,
    pub extra_info: StateTestbedGuestExtraInfo,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct StateMachine {
    pub name: String,
    pub network: Option<Vec<StateMachineNetwork>>,
    // flatten means we don't need to specify "guest_type" and directly specify the GuestType variant
    #[serde(flatten)]
    pub guest_type: StateGuestType,
}

impl TryFrom<Machine> for StateMachine {

    type Error = anyhow::Error;

    fn try_from(machine: Machine) -> anyhow::Result<Self> {
        Ok(Self {
            name: machine.name,
            // we need to re-compose the optional vec from inner type MachineNetwork to StateMachineNetwork
            network: machine.network.map(|vec| {
                vec.iter().map(Into::into).collect()
            }),
            guest_type: machine.guest_type.try_into()?,
        })
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct StateMachineNetwork {
    pub switch: String,
    pub gateway: Option<String>,
    pub mac: String,
    pub ip: String,
    pub network_name: Option<String>,
}

impl From<&MachineNetwork> for StateMachineNetwork {
    fn from(machine: &MachineNetwork) -> Self {
        Self {
            switch: machine.switch.clone(),
            gateway: machine.gateway.clone(),
            mac: machine.mac.clone(),
            ip: machine.ip.clone(),
            network_name: machine.network_name.clone(),
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum StateGuestType {
    Libvirt(StateLibvirtMachine),
    Docker(StateDockerMachine),
    Android(StateAVDMachine),
}

impl StateGuestType {
    pub fn name(&self) -> String {
        match self {
            Self::Libvirt(_) => "Libvirt".into(),
            Self::Docker(_) => "Docker".into(),
            Self::Android(_) => "Android".into(),
        }
    }
}

impl TryFrom<GuestType> for StateGuestType {

    type Error = anyhow::Error;

    fn try_from(guest_type: GuestType) -> anyhow::Result<Self> {
        let res = match guest_type {
            GuestType::Libvirt(libvirt) => StateGuestType::Libvirt(libvirt.try_into()?),
            GuestType::Docker(docker) => StateGuestType::Docker(docker.into()),
            GuestType::Android(android) => StateGuestType::Android(android.into()),
        };
        Ok(res)
    }
}

/// This contains extra information on the guest that is not captured by the yaml, but is computed from a combination of
/// the yaml and the testbed environment, making it unique to a testbed
#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub struct StateTestbedGuestExtraInfo {
    /// This is used when a guest is based off another resource i.e. an image for existing disk, or an iso for iso guest
    pub reference_image: Option<String>,
}
