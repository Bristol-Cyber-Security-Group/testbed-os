use std::future::Future;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use crate::orchestration::api::{OrchestrationResource, OrchestrationResourceNetwork, OrchestrationResourceNetworkType};
use crate::orchestration::OrchestrationCommon;
use crate::ovn::{OvnCommand};
use crate::vec_of_strings;

/// This represents a logical router port configuration to set it as an external gateway port.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OvnExternalGateway {
    pub router_port_name: String,
    pub chassis_name: String,
}

impl OvnExternalGateway {
    pub fn new(
        router_port_name: String,
        chassis_name: String,
    ) -> Self {
        Self {
            router_port_name,
            chassis_name,
        }
    }

    pub fn to_orchestration_resource(
        &self,
    ) -> OrchestrationResource {
        OrchestrationResource::Network(OrchestrationResourceNetworkType::Ovn(OrchestrationResourceNetwork::ExternalGateway(self.clone())))
    }
}

#[async_trait]
impl OvnCommand for OvnExternalGateway {
    async fn create_command<F>(&self, f: impl Fn(Vec<String>, (Option<String>, OrchestrationCommon)) -> F + Send + Sync, config: (Option<String>, OrchestrationCommon)) -> anyhow::Result<String>
        where
            F: Future<Output=anyhow::Result<String>> + Send
    {
        tracing::info!("creating external gateway ({:?}, {:?}) on LRP {}", &self.router_port_name, &self.chassis_name, &self.router_port_name);
        // TODO - priority for port rather than hardcode 20 or no priority
        f(vec_of_strings!["ovn-nbctl", "--may-exist", "lrp-set-gateway-chassis", &self.router_port_name, &self.chassis_name, "20"], config).await
    }

    async fn destroy_command<F>(&self, f: impl Fn(Vec<String>, (Option<String>, OrchestrationCommon)) -> F + Send + Sync, config: (Option<String>, OrchestrationCommon)) -> anyhow::Result<String>
        where
            F: Future<Output=anyhow::Result<String>> + Send
    {
        tracing::info!("destroying external gateway ({:?}, {:?}) on LRP {}", &self.router_port_name, &self.chassis_name, &self.router_port_name);
        f(vec_of_strings!["ovn-nbctl", "lrp-del-gateway-chassis", &self.router_port_name, &self.chassis_name], config).await
    }
}

#[cfg(test)]
mod tests {
    use crate::ovn::test_ovn_run_cmd;
    use super::*;

    #[tokio::test]
    async fn test_external_gateway() {
        let gw = OvnExternalGateway::new(
            "lr0-public".into(),
            "ovn".into(),
        );
        let set_gateway = gw.create_command(test_ovn_run_cmd, (None, OrchestrationCommon::default())).await.unwrap();
        let expected_cmd = vec_of_strings!["ovn-nbctl", "--may-exist", "lrp-set-gateway-chassis", "lr0-public", "ovn", "20"].join(" ");
        assert_eq!(set_gateway, expected_cmd);
        let del_gateway = gw.destroy_command(test_ovn_run_cmd, (None, OrchestrationCommon::default())).await.unwrap();
        let expected_cmd = vec_of_strings!["ovn-nbctl", "lrp-del-gateway-chassis", "lr0-public", "ovn"].join(" ");
        assert_eq!(del_gateway, expected_cmd);
    }
}