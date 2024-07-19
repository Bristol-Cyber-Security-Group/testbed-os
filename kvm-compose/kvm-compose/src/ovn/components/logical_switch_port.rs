use std::future::Future;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use crate::orchestration::api::{OrchestrationResource, OrchestrationResourceNetwork, OrchestrationResourceNetworkType};
use crate::orchestration::OrchestrationCommon;
use crate::ovn::components::{MacAddress, OvnIpAddr};
use crate::ovn::{OvnCommand};
use crate::vec_of_strings;

/// This represents a logical switch port that has different variants, depending on what kind of
/// port it is providing in the network
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LogicalSwitchPort {
    pub name: String,
    pub parent_switch: String,
    pub port_type: LogicalSwitchPortType,
    pub dhcp_options_uuid: Option<u64>, // this is the hash of DhcpDatabaseEntry
}

impl LogicalSwitchPort {
    pub fn new(
        name: String,
        parent_switch: String,
        port_type: LogicalSwitchPortType,
    ) -> Self {
        Self {
            name,
            parent_switch,
            port_type,
            dhcp_options_uuid: None,
        }
    }

    pub fn to_orchestration_resource(
        &self,
    ) -> OrchestrationResource {
        OrchestrationResource::Network(OrchestrationResourceNetworkType::Ovn(OrchestrationResourceNetwork::SwitchPort(self.clone())))
    }
}

/// This enum represents the different types of port, each type of port will require different data
/// to be fully functional in the network. Not all data is relevant to all port types.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum LogicalSwitchPortType {
    Internal {
        ovs_port_name: String, // TODO - this can be removed? ovs ports are made in guest orchestration now
        ip: OvnIpAddr, // mut be ip
        chassis_name: Option<String>,
        mac_address: MacAddress,
        provider_network_name: Option<String>, // TODO is this option or mandatory?
    },
    Router {
        router_port_name: String,
        mac_address: MacAddress,
    },
    LocalNet {
        provider_network_name: String,
    },
    // LocalPort,
}

impl LogicalSwitchPortType {
    pub fn new_internal(
        ovs_port_name: String,
        ip: OvnIpAddr,
        chassis_name: Option<String>,
        mac_address: MacAddress,
        provider_network_name: Option<String>,
    ) -> LogicalSwitchPortType {
        LogicalSwitchPortType::Internal {
            ovs_port_name,
            ip,
            chassis_name,
            mac_address,
            provider_network_name,
        }
    }

    pub fn new_router(
        router_port_name: String,
        mac_address: MacAddress,
    ) -> LogicalSwitchPortType {
        LogicalSwitchPortType::Router {
            router_port_name,
            mac_address,
        }
    }

    pub fn new_localnet(
        provider_network_name: String,
    ) -> LogicalSwitchPortType {
        LogicalSwitchPortType::LocalNet {
            provider_network_name,
        }
    }
}

#[async_trait]
impl OvnCommand for LogicalSwitchPort {
    async fn create_command<F>(&self, f: impl Fn(Vec<String>, (Option<String>, OrchestrationCommon)) -> F + Send + Sync, config: (Option<String>, OrchestrationCommon)) -> anyhow::Result<String>
        where
            F: Future<Output=anyhow::Result<String>> + Send
    {
        match &self.port_type {
            LogicalSwitchPortType::Internal {
                ovs_port_name: _,
                ip,
                chassis_name,
                mac_address,
                provider_network_name
            } => {
                tracing::info!("creating LSP type internal {} on LS {}", &self.name, &self.parent_switch);
                let ip = ip.to_string();
                let mac_address = mac_address.get_string();
                let mut cmd = vec_of_strings![
                    "ovn-nbctl", "--may-exist", "lsp-add", &self.parent_switch, &self.name,
                    "--", "set", "Logical_Switch_Port", &self.name,
                    format!("addresses=\"{mac_address} {ip}\"")
                ];
                let mut options = "options:".to_string();
                if let Some(network_name) = provider_network_name {
                    options.push_str(&format!("network_name={network_name},"))
                }
                if let Some(chassis_name) = chassis_name {
                    options.push_str(&format!("chassis={chassis_name}"));
                }
                cmd.push(options);
                // run command
                f(cmd, config).await
            }
            LogicalSwitchPortType::Router {
                router_port_name,
                mac_address
            } => {
                tracing::info!("creating LSP type router {} on LS {}", &self.name, &self.parent_switch);
                let mac_address = mac_address.get_string();
                // run command
                f(vec_of_strings![
                    "ovn-nbctl", "--may-exist", "lsp-add", &self.parent_switch, &self.name,
                    "--", "set", "Logical_Switch_Port", &self.name, "type=router",
                    format!("options:router-port={router_port_name}"),
                    format!("addresses=\"{mac_address}\"")
                ], config).await
            }
            LogicalSwitchPortType::LocalNet {
                provider_network_name
            } => {
                tracing::info!("creating LSP type localnet {} on LS {}", &self.name, &self.parent_switch);
                // run command
                f(vec_of_strings![
                    "ovn-nbctl", "--may-exist", "lsp-add", &self.parent_switch, &self.name,
                    "--", "set", "Logical_Switch_Port", &self.name, "type=localnet",
                    format!("options:network_name={provider_network_name}"),
                    format!("addresses=\"unknown\"")
                ], config).await
            }
        }
    }

    async fn destroy_command<F>(&self, f: impl Fn(Vec<String>, (Option<String>, OrchestrationCommon)) -> F + Send + Sync, config: (Option<String>, OrchestrationCommon)) -> anyhow::Result<String>
        where
            F: Future<Output=anyhow::Result<String>> + Send
    {
        tracing::info!("destroying LSP type {:?} {} on LS {}", self.port_type, &self.name, &self.parent_switch);
        f(vec_of_strings!["ovn-nbctl", "lsp-del", &self.name], config).await
    }
}

#[cfg(test)]
mod tests {
    use std::net::{IpAddr, Ipv4Addr};
    use crate::ovn::test_ovn_run_cmd;
    use super::*;

    #[tokio::test]
    async fn test_logical_switch_port_internal() {
        // internal
        let internal = LogicalSwitchPortType::new_internal(
            "ovs-sw0-port0".to_string(),
            OvnIpAddr::Ip(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 2))),
            Some("ovn".to_string()),
            MacAddress::new("00:00:00:00:00:01".into()).unwrap(),
            None
        );
        let lsp = LogicalSwitchPort::new(
            "sw0-port0".into(),
            "sw0".into(),
            internal,
        );
        let expected_cmd = vec_of_strings![
            "ovn-nbctl", "--may-exist", "lsp-add", "sw0", "sw0-port0",
            "--", "set", "Logical_Switch_Port", "sw0-port0",
            "addresses=\"00:00:00:00:00:01 10.0.0.2\"", "options:chassis=ovn"
        ].join(" ");
        let create_cmd = lsp.create_command(test_ovn_run_cmd, (None, OrchestrationCommon::default())).await.unwrap();
        assert_eq!(create_cmd, expected_cmd);

        // test delete
        let delete_cmd = lsp.destroy_command(test_ovn_run_cmd, (None, OrchestrationCommon::default())).await.unwrap();
        let expected_cmd = vec_of_strings!["ovn-nbctl", "lsp-del", "sw0-port0"].join(" ");
        assert_eq!(delete_cmd, expected_cmd);
    }

    #[tokio::test]
    async fn test_logical_switch_port_internal_with_provider() {
        // internal with provider network
        let internal = LogicalSwitchPortType::new_internal(
            "ovs-sw0-port0".to_string(),
            OvnIpAddr::Ip(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 2))),
            Some("ovn".to_string()),
            MacAddress::new("00:00:00:00:00:01".into()).unwrap(),
            Some("public".into())
        );
        let lsp = LogicalSwitchPort::new(
            "sw0-port0".into(),
            "sw0".into(),
            internal,
        );
        let expected_cmd = vec_of_strings![
            "ovn-nbctl", "--may-exist", "lsp-add", "sw0", "sw0-port0",
            "--", "set", "Logical_Switch_Port", "sw0-port0",
            "addresses=\"00:00:00:00:00:01 10.0.0.2\"", "options:network_name=public,chassis=ovn"
        ].join(" ");
        let create_cmd = lsp.create_command(test_ovn_run_cmd, (None, OrchestrationCommon::default())).await.unwrap();
        assert_eq!(create_cmd, expected_cmd);

        // test delete
        let delete_cmd = lsp.destroy_command(test_ovn_run_cmd, (None, OrchestrationCommon::default())).await.unwrap();
        let expected_cmd = vec_of_strings!["ovn-nbctl", "lsp-del", "sw0-port0"].join(" ");
        assert_eq!(delete_cmd, expected_cmd);
    }

    #[tokio::test]
    async fn test_logical_switch_port_router() {
        // router type
        let router = LogicalSwitchPortType::new_router(
            "lr0-port0".into(),
            MacAddress::new("00:00:00:00:ff:01".into()).unwrap(),
        );
        let lsp = LogicalSwitchPort::new(
            "sw0-port0".into(),
            "sw0".into(),
            router,
        );
        let expected_cmd = vec_of_strings![
            "ovn-nbctl", "--may-exist", "lsp-add", "sw0", "sw0-port0",
            "--", "set", "Logical_Switch_Port", "sw0-port0", "type=router",
            "options:router-port=lr0-port0", "addresses=\"00:00:00:00:ff:01\""
        ].join(" ");
        let create_cmd = lsp.create_command(test_ovn_run_cmd, (None, OrchestrationCommon::default())).await.unwrap();
        assert_eq!(create_cmd, expected_cmd);

        // test delete
        let delete_cmd = lsp.destroy_command(test_ovn_run_cmd, (None, OrchestrationCommon::default())).await.unwrap();
        let expected_cmd = vec_of_strings!["ovn-nbctl", "lsp-del", "sw0-port0"].join(" ");
        assert_eq!(delete_cmd, expected_cmd);
    }

    #[tokio::test]
    async fn test_logical_switch_port_localnet() {
        // localnet type
        let localnet = LogicalSwitchPortType::new_localnet(
            "public".into(),
        );
        let lsp = LogicalSwitchPort::new(
            "sw0-port0".into(),
            "sw0".into(),
            localnet,
        );
        let expected_cmd = vec_of_strings![
            "ovn-nbctl", "--may-exist", "lsp-add", "sw0", "sw0-port0",
            "--", "set", "Logical_Switch_Port", "sw0-port0", "type=localnet",
            "options:network_name=public", "addresses=\"unknown\""
        ].join(" ");
        let create_cmd = lsp.create_command(test_ovn_run_cmd, (None, OrchestrationCommon::default())).await.unwrap();
        assert_eq!(create_cmd, expected_cmd);

        // test delete
        let delete_cmd = lsp.destroy_command(test_ovn_run_cmd, (None, OrchestrationCommon::default())).await.unwrap();
        let expected_cmd = vec_of_strings!["ovn-nbctl", "lsp-del", "sw0-port0"].join(" ");
        assert_eq!(delete_cmd, expected_cmd);
    }

}
