use std::future::Future;
use std::net::IpAddr;
use anyhow::bail;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use crate::orchestration::api::{OrchestrationResource, OrchestrationResourceNetwork, OrchestrationResourceNetworkType};
use crate::orchestration::OrchestrationCommon;
use crate::ovn::components::OvnIpAddr;
use crate::ovn::{OvnCommand};
use crate::vec_of_strings;

/// This represents routing rules for logical routers
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OvnRoute {
    pub router_name: String,
    pub prefix: OvnIpAddr, // can be a subnet with mask or ip
    pub next_hop: OvnIpAddr, // mut be ip
    // pub port: Option<String>, // TODO - LSP or LRP? or both so for example, allow only a specific port to have the route
}

impl OvnRoute {
    pub fn new(
        router_name: String,
        prefix: OvnIpAddr,
        next_hop: IpAddr,
    ) -> anyhow::Result<Self> {
        let prefix = match prefix {
            OvnIpAddr::Ip(_) => prefix,
            OvnIpAddr::Dynamic => bail!("cannot be dynamic ip"),
            OvnIpAddr::Subnet { .. } => prefix,
        };
        Ok(Self {
            router_name,
            prefix,
            next_hop: OvnIpAddr::Ip(next_hop),
        })
    }

    pub fn to_orchestration_resource(
        &self,
    ) -> OrchestrationResource {
        OrchestrationResource::Network(OrchestrationResourceNetworkType::Ovn(OrchestrationResourceNetwork::Route(self.clone())))
    }
}

#[async_trait]
impl OvnCommand for OvnRoute {
    async fn create_command<F>(&self, f: impl Fn(Vec<String>, (Option<String>, OrchestrationCommon)) -> F + Send + Sync, config: (Option<String>, OrchestrationCommon)) -> anyhow::Result<String>
        where
            F: Future<Output=anyhow::Result<String>> + Send
    {
        tracing::info!("creating route ({:?}, {:?}) on LR {}", &self.prefix, &self.next_hop, &self.router_name);
        f(vec_of_strings!["ovn-nbctl", "--may-exist", "lr-route-add", &self.router_name, &self.prefix.to_string(), self.next_hop.to_string()], config).await
    }

    async fn destroy_command<F>(&self, f: impl Fn(Vec<String>, (Option<String>, OrchestrationCommon)) -> F + Send + Sync, config: (Option<String>, OrchestrationCommon)) -> anyhow::Result<String>
        where
            F: Future<Output=anyhow::Result<String>> + Send
    {
        tracing::info!("destroying route ({:?}, {:?}) on LR {}", &self.prefix, &self.next_hop, &self.router_name);
        f(vec_of_strings!["ovn-nbctl", "lr-route-del", &self.router_name, &self.prefix.to_string(), self.next_hop.to_string()], config).await
    }
}
