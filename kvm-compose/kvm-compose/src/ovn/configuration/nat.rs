use std::future::Future;
use anyhow::bail;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use crate::orchestration::api::{OrchestrationResource, OrchestrationResourceNetwork, OrchestrationResourceNetworkType};
use crate::orchestration::OrchestrationCommon;
use crate::ovn::components::{OvnIpAddr};
use crate::ovn::{OvnCommand};
use crate::vec_of_strings;

/// This represents network address translation rules for logical routers
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OvnNat {
    pub logical_router_name: String,
    pub external_ip: OvnIpAddr, // mut be ip
    pub logical_ip: OvnIpAddr, // can be a subnet with mask or an ip
    pub nat_type: OvnNatType,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum OvnNatType {
    SNat,
    DnatSNat,
}

impl OvnNatType {
    pub fn to_string(
        &self,
    ) -> String {
        match &self {
            OvnNatType::SNat => "snat".into(),
            OvnNatType::DnatSNat => "dnat_and_snat".into(),
        }
    }
}

impl OvnNat {
    pub fn new(
        logical_router_name: String,
        external_ip: OvnIpAddr,
        logical_ip: OvnIpAddr,
        nat_type: OvnNatType,
    ) -> anyhow::Result<Self> {
        let external_ip = match external_ip {
            OvnIpAddr::Ip(_) => external_ip,
            OvnIpAddr::Dynamic => bail!("cannot be dynamic ip"),
            OvnIpAddr::Subnet { .. } => bail!("cannot be subnet ip"),
        };
        let logical_ip = match logical_ip {
            OvnIpAddr::Ip(_) => logical_ip,
            OvnIpAddr::Dynamic => bail!("cannot be dynamic ip"),
            OvnIpAddr::Subnet { .. } => logical_ip,
        };
        Ok(Self {
            logical_router_name,
            external_ip,
            logical_ip,
            nat_type,
        })
    }

    pub fn to_orchestration_resource(
        &self,
    ) -> OrchestrationResource {
        OrchestrationResource::Network(OrchestrationResourceNetworkType::Ovn(OrchestrationResourceNetwork::Nat(self.clone())))
    }
}

#[async_trait]
impl OvnCommand for OvnNat {
    async fn create_command<F>(&self, f: impl Fn(Vec<String>, (Option<String>, OrchestrationCommon)) -> F + Send + Sync, config: (Option<String>, OrchestrationCommon)) -> anyhow::Result<String>
        where
            F: Future<Output=anyhow::Result<String>> + Send
    {
        tracing::info!("creating nat rule ({:?}, {:?}, {:?}) on LR {}", &self.external_ip, &self.logical_ip, &self.nat_type, &self.logical_router_name);
        f(vec_of_strings![
            "ovn-nbctl", "--may-exist", "lr-nat-add", &self.logical_router_name,
            &self.nat_type.to_string(), &self.external_ip.to_string(), &self.logical_ip
        ], config).await
    }

    async fn destroy_command<F>(&self, f: impl Fn(Vec<String>, (Option<String>, OrchestrationCommon)) -> F + Send + Sync, config: (Option<String>, OrchestrationCommon)) -> anyhow::Result<String>
        where
            F: Future<Output=anyhow::Result<String>> + Send
    {
        tracing::info!("destroying nat rule ({:?}, {:?}, {:?}) on LR {}", &self.external_ip, &self.logical_ip, &self.nat_type, &self.logical_router_name);
        f(vec_of_strings![
            "ovn-nbctl", "lr-nat-del", &self.logical_router_name,
            &self.nat_type.to_string(), &self.external_ip.to_string()
        ], config).await
    }
}
