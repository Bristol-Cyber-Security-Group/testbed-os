use std::future::Future;
use std::net::IpAddr;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use crate::orchestration::api::{OrchestrationResource, OrchestrationResourceNetwork, OrchestrationResourceNetworkType};
use crate::orchestration::OrchestrationCommon;
use crate::ovn::components::OvnIpAddr;
use crate::ovn::{OvnCommand};
use crate::ovn::configuration::dhcp::SwitchDhcpOptions;
use crate::vec_of_strings;

/// This represents an OVN logical switch
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LogicalSwitch {
    pub name: String,
    pub subnet: OvnIpAddr, // must be subnet
    pub dhcp: Option<SwitchDhcpOptions>,
}

impl LogicalSwitch {
    pub fn new(
        name: String,
        subnet: IpAddr,
        mask: u16,
    ) -> Self {
        Self {
            name,
            subnet: OvnIpAddr::Subnet {
                ip: subnet,
                mask,
            },
            dhcp: None,
        }
    }

    pub fn to_orchestration_resource(
        &self,
    ) -> OrchestrationResource {
        OrchestrationResource::Network(OrchestrationResourceNetworkType::Ovn(OrchestrationResourceNetwork::Switch(self.clone())))
    }
}

#[async_trait]
impl OvnCommand for LogicalSwitch {
    async fn create_command<F>(&self, f: impl Fn(Vec<String>, (Option<String>, OrchestrationCommon)) -> F + Send + Sync, config: (Option<String>, OrchestrationCommon)) -> anyhow::Result<String>
        where
            F: Future<Output=anyhow::Result<String>> + Send
    {
        tracing::info!("creating LS {}", &self.name);
        let other_config = format!("other_config:subnet={}", &self.subnet.to_string());
        let mut cmd = vec_of_strings!["ovn-nbctl", "--may-exist", "ls-add", &self.name, "--", "set", "Logical_Switch", &self.name, &other_config];
        if let Some(dhcp) = &self.dhcp {
            tracing::info!("adding exclude ips option on LS {} as there is a switch port with a dynamic ip address", &self.name);
            cmd.push(format!("other_config:exclude_ips={}", &dhcp.exclude_ips))
        }
        f(cmd, config).await
    }

    async fn destroy_command<F>(&self, f: impl Fn(Vec<String>, (Option<String>, OrchestrationCommon)) -> F + Send + Sync, config: (Option<String>, OrchestrationCommon)) -> anyhow::Result<String>
        where
            F: Future<Output=anyhow::Result<String>> + Send
    {
        tracing::info!("destroying LS {}", &self.name);
        f(vec_of_strings!["ovn-nbctl", "ls-del", &self.name], config).await
    }
}

#[cfg(test)]
mod tests {
    use std::net::Ipv4Addr;
    use crate::ovn::test_ovn_run_cmd;
    use super::*;

    #[tokio::test]
    async fn test_logical_switch() {
        let ls = LogicalSwitch::new(
            "sw0".into(),
            IpAddr::V4(Ipv4Addr::new(10,0,0,0)),
            24,
        );
        let expected_add = vec_of_strings!["ovn-nbctl", "--may-exist", "ls-add", "sw0", "--", "set", "Logical_Switch", "sw0", "other_config:subnet=10.0.0.0/24"].join(" ");
        assert_eq!(expected_add, ls.create_command(test_ovn_run_cmd, (None, OrchestrationCommon::default())).await.unwrap());
        let expected_del = vec_of_strings!["ovn-nbctl", "ls-del", "sw0"].join(" ");
        assert_eq!(expected_del, ls.destroy_command(test_ovn_run_cmd, (None, OrchestrationCommon::default())).await.unwrap());

    }

    #[tokio::test]
    async fn test_logical_switch_dhcp() {
        // test if it has DHCP option
        let mut ls = LogicalSwitch::new(
            "sw0".into(),
            IpAddr::V4(Ipv4Addr::new(10,0,0,0)),
            24,
        );
        ls.dhcp = Some(SwitchDhcpOptions { exclude_ips: "10.0.0.1..10.0.0.10".to_string() });
        let expected_add = vec_of_strings!["ovn-nbctl", "--may-exist", "ls-add", "sw0", "--", "set", "Logical_Switch", "sw0", "other_config:subnet=10.0.0.0/24", "other_config:exclude_ips=10.0.0.1..10.0.0.10"].join(" ");
        assert_eq!(expected_add, ls.create_command(test_ovn_run_cmd, (None, OrchestrationCommon::default())).await.unwrap());
        let expected_del = vec_of_strings!["ovn-nbctl", "ls-del", "sw0"].join(" ");
        assert_eq!(expected_del, ls.destroy_command(test_ovn_run_cmd, (None, OrchestrationCommon::default())).await.unwrap());

    }
}