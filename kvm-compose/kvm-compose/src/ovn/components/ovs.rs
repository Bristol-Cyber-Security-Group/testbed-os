use std::future::Future;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use crate::orchestration::api::{OrchestrationResource, OrchestrationResourceNetwork, OrchestrationResourceNetworkType};
use crate::orchestration::OrchestrationCommon;
use crate::ovn::components::OvnIpAddr;
use crate::ovn::{OvnCommand};
use crate::vec_of_strings;

/// This represents the OVS settings on an OVN chassis
pub struct OvsSystem {
    pub system_id: String,
    pub ovn_encap_type: String,
    pub ovn_encap_ip: OvnIpAddr, // mut be ip
    pub ovn_remote_unix: String,
    pub ovn_bridge: String,
    // TODO this could be a list
    pub ovn_bridge_mappings: Option<String>,
}

/// This represents and OVS bridge on an OVN chassis
pub struct OvsBridge {
    pub name: String,
    pub bridge_type: OvsBridgeType,
}

/// An OVS bridge can either be a provider or integration bridge
pub enum OvsBridgeType {
    Integration,
    Provider {
        provider_network_name: String,
        ip: OvnIpAddr, // mut be ip
    },
}

/// An OVS port can be assigned to the integration bridge and will have a specific logical switch
/// port assigned via the external id metadata
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OvsPort {
    pub name: String,
    pub integration_bridge_name: String,
    pub lsp_name: String,
    pub chassis: String,
}

impl OvsPort {
    pub fn new(
        name: String,
        integration_bridge_name: String,
        lsp_name: String,
        chassis: String,
    ) -> Self {
        Self {
            name,
            integration_bridge_name,
            lsp_name,
            chassis,
        }
    }

    pub fn to_orchestration_resource(
        &self,
    ) -> OrchestrationResource {
        OrchestrationResource::Network(OrchestrationResourceNetworkType::Ovn(OrchestrationResourceNetwork::OvsPort(self.clone())))
    }
}

#[async_trait]
impl OvnCommand for OvsPort {
    async fn create_command<F>(&self, f: impl Fn(Vec<String>, (Option<String>, OrchestrationCommon)) -> F + Send + Sync, config: (Option<String>, OrchestrationCommon)) -> anyhow::Result<String>
        where
            F: Future<Output=anyhow::Result<String>> + Send
    {
        tracing::info!("creating OVS port {} on chassis {}", &self.name, &self.chassis);
        let lsp_port = &self.lsp_name;
        f(vec_of_strings![
            "ovs-vsctl", "--may-exist", "add-port", &self.integration_bridge_name, &self.name,
            "--", "set", "Interface", &self.name, "type=internal",
            "--", "set", "Interface", &self.name, format!("external_ids:iface-id={lsp_port}")
        ], config).await
    }

    async fn destroy_command<F>(&self, f: impl Fn(Vec<String>, (Option<String>, OrchestrationCommon)) -> F + Send + Sync, config: (Option<String>, OrchestrationCommon)) -> anyhow::Result<String>
        where
            F: Future<Output=anyhow::Result<String>> + Send
    {
        tracing::info!("destroying OVS port {} on chassis {}", &self.name, &self.chassis);
        f(vec_of_strings!["ovs-vsctl", "del-port", &self.integration_bridge_name, &self.name], config).await
    }
}

#[cfg(test)]
mod tests {
    use crate::ovn::test_ovn_run_cmd;
    use super::*;

    #[tokio::test]
    async fn test_ovs_port() {
        let port = OvsPort::new(
            "ovs-port".into(),
            "br-int".into(),
            "sw0-port0".into(),
            "ovn".into()
        );
        let create_cmd = port.create_command(&test_ovn_run_cmd, (None, OrchestrationCommon::default())).await.unwrap();
        let expected_cmd = vec_of_strings![
            "ovs-vsctl", "--may-exist", "add-port", "br-int", "ovs-port",
            "--", "set", "Interface", "ovs-port", "type=internal",
            "--", "set", "Interface", "ovs-port", "external_ids:iface-id=sw0-port0"
        ].join(" ");
        assert_eq!(create_cmd, expected_cmd);
        let destroy_cmd = port.destroy_command(&test_ovn_run_cmd, (None, OrchestrationCommon::default())).await.unwrap();
        let expected_cmd = vec_of_strings!["ovs-vsctl", "del-port", "br-int", "ovs-port"].join(" ");
        assert_eq!(destroy_cmd, expected_cmd);
    }
}