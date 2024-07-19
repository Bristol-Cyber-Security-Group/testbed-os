use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::future::Future;
use async_trait::async_trait;
use crate::orchestration::api::{OrchestrationResource, OrchestrationResourceNetwork, OrchestrationResourceNetworkType};
use crate::orchestration::OrchestrationCommon;
use crate::ovn::{OvnCommand};
use crate::ovn::configuration::external_gateway::OvnExternalGateway;
use crate::ovn::configuration::nat::OvnNat;
use crate::ovn::configuration::route::OvnRoute;
use crate::vec_of_strings;

// serde implementations for these wrapper structs in ovn_serde.rs
#[derive(Debug, Clone)]
pub struct RoutingMap(pub HashMap<(String, String, String), OvnRoute>);

#[derive(Debug, Clone)]
pub struct ExternalGatewayMap(pub HashMap<(String, String), OvnExternalGateway>);

#[derive(Debug, Clone)]
pub struct NatMap(pub HashMap<(String, String, String), OvnNat>);

/// This represents an OVN logical router. The router can optionally contain routes, external
/// gateways and NAT configuration. The implementation for these are in the `configuration` folder
/// in this `ovn` crate.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LogicalRouter {
    pub name: String,
    pub routing: RoutingMap,
    pub external_gateway: ExternalGatewayMap,
    pub nat: NatMap,
}

impl LogicalRouter {
    pub fn new(
        name: String,
    ) -> Self {
        Self {
            name,
            routing: RoutingMap(HashMap::new()),
            external_gateway: ExternalGatewayMap(HashMap::new()),
            nat: NatMap(HashMap::new()),
        }
    }

    pub fn to_orchestration_resource(
        &self,
    ) -> OrchestrationResource {
        OrchestrationResource::Network(OrchestrationResourceNetworkType::Ovn(OrchestrationResourceNetwork::Router(self.clone())))
    }
}

#[async_trait]
impl OvnCommand for LogicalRouter {
    async fn create_command<F>(&self, f: impl Fn(Vec<String>, (Option<String>, OrchestrationCommon)) -> F + Send + Sync, config: (Option<String>, OrchestrationCommon)) -> anyhow::Result<String>
        where
            F: Future<Output=anyhow::Result<String>> + Send
    {
        tracing::info!("creating LR {}", &self.name);
        f(vec_of_strings!["ovn-nbctl", "--may-exist", "lr-add", &self.name], config).await
    }

    async fn destroy_command<F>(&self, f: impl Fn(Vec<String>, (Option<String>, OrchestrationCommon)) -> F + Send + Sync, config: (Option<String>, OrchestrationCommon)) -> anyhow::Result<String>
        where
            F: Future<Output=anyhow::Result<String>> + Send
    {
        tracing::info!("destroying LR {}", &self.name);
        f(vec_of_strings!["ovn-nbctl", "lr-del", &self.name], config).await
    }
}

#[cfg(test)]
mod tests {
    use crate::ovn::test_ovn_run_cmd;
    use super::*;

    #[tokio::test]
    async fn test_logical_router() {
        let lr0 = LogicalRouter::new("lr0".into());
        let create_cmd = lr0.create_command(test_ovn_run_cmd, (None, OrchestrationCommon::default())).await.unwrap();
        let expected_cmd = vec_of_strings!["ovn-nbctl", "--may-exist", "lr-add", "lr0"].join(" ");
        assert_eq!(create_cmd, expected_cmd);
        let destroy_cmd = lr0.destroy_command(test_ovn_run_cmd, (None, OrchestrationCommon::default())).await.unwrap();
        let expected_cmd = vec_of_strings!["ovn-nbctl", "lr-del", "lr0"].join(" ");
        assert_eq!(destroy_cmd, expected_cmd);
    }
}