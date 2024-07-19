use std::future::Future;
use std::net::IpAddr;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use crate::orchestration::api::{OrchestrationResource, OrchestrationResourceNetwork, OrchestrationResourceNetworkType};
use crate::orchestration::OrchestrationCommon;
use crate::ovn::components::{MacAddress, OvnIpAddr};
use crate::ovn::{OvnCommand};
use crate::vec_of_strings;

/// This represents an OVN logical router port
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LogicalRouterPort {
    pub name: String,
    pub parent_router: String,
    pub mac_address: MacAddress,
    pub ip: OvnIpAddr, // mut be ip with mask
    pub chassis_name: Option<String>, // set chassis to make this a gateway router
}

impl LogicalRouterPort {
    pub fn new(
        name: String,
        parent_router: String,
        mac_address: MacAddress,
        ip: IpAddr,
        mask: u16,
        chassis_name: Option<String>,
    ) -> Self {
        Self {
            name,
            parent_router,
            mac_address,
            ip: OvnIpAddr::Subnet {
                ip,
                mask,
            },
            chassis_name,
        }
    }

    pub fn to_orchestration_resource(
        &self,
    ) -> OrchestrationResource {
        OrchestrationResource::Network(OrchestrationResourceNetworkType::Ovn(OrchestrationResourceNetwork::RouterPort(self.clone())))
    }

    // pub fn set_gateway_chassis_command(
    //     &self,
    // ) -> Option<Vec<String>> {
    //     match &self.chassis_name {
    //         None => None,
    //         Some(chassis_name) => {
    //             // TODO - priority for port rather than hardcode 20 or no priority
    //             Some(vec_of_strings!["ovn-nbctl", "lrp-set-gateway-chassis", &self.name, chassis_name, "20"])
    //         }
    //     }
    // }
    //
    // pub fn del_gateway_chassis_command(
    //     &self,
    // ) -> Option<Vec<String>> {
    //     match &self.chassis_name {
    //         None => None,
    //         Some(chassis_name) => {
    //             Some(vec_of_strings!["ovn-nbctl", "lrp-del-gateway-chassis", &self.name, chassis_name])
    //         }
    //     }
    // }
}

#[async_trait]
impl OvnCommand for LogicalRouterPort {
    async fn create_command<F>(&self, f: impl Fn(Vec<String>, (Option<String>, OrchestrationCommon)) -> F + Send + Sync, config: (Option<String>, OrchestrationCommon)) -> anyhow::Result<String>
        where
            F: Future<Output=anyhow::Result<String>> + Send
    {
        tracing::info!("creating LRP {}", &self.name);
        f(vec_of_strings![
            "ovn-nbctl", "--may-exist", "lrp-add", &self.parent_router, &self.name, self.mac_address.get_string(), self.ip.to_string()
        ], config).await
    }

    async fn destroy_command<F>(&self, f: impl Fn(Vec<String>, (Option<String>, OrchestrationCommon)) -> F + Send + Sync, config: (Option<String>, OrchestrationCommon)) -> anyhow::Result<String>
        where
            F: Future<Output=anyhow::Result<String>> + Send
    {
        tracing::info!("destroying LRP {}", &self.name);
        f(vec_of_strings!["ovn-nbctl", "lrp-del", &self.name], config).await
    }
}

#[cfg(test)]
mod tests {
    use std::net::Ipv4Addr;
    use crate::ovn::test_ovn_run_cmd;
    use super::*;

    #[tokio::test]
    async fn test_logical_router_port() {
        let lrp0 = LogicalRouterPort::new(
            "lr0-port0".into(),
            "lr0".into(),
            MacAddress::new("00:00:00:00:ff:01".into()).unwrap(),
            IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)),
            24,
            None,
        );
        let create_cmd = lrp0.create_command(test_ovn_run_cmd, (None, OrchestrationCommon::default())).await.unwrap();
        let expected_cmd = vec_of_strings![
            "ovn-nbctl", "--may-exist", "lrp-add", "lr0", "lr0-port0", "00:00:00:00:ff:01", "10.0.0.1/24"
        ].join(" ");
        assert_eq!(create_cmd, expected_cmd);

    }

}