use std::collections::hash_map::DefaultHasher;
use std::future::Future;
use std::hash::{Hash, Hasher};
use anyhow::{bail, Context};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use crate::orchestration::api::{OrchestrationResource, OrchestrationResourceNetwork, OrchestrationResourceNetworkType};
use crate::orchestration::OrchestrationCommon;
use crate::ovn::components::{MacAddress, OvnIpAddr};
use crate::ovn::OvnCommand;
use crate::state::StateNetwork;
use crate::vec_of_strings;

/// This represents DHCP options for a logical switch
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SwitchDhcpOptions {
    pub exclude_ips: String, // for now only allow one range
}

impl SwitchDhcpOptions {
    pub fn new(
        exclude_ips: String,
    ) -> Self {
        Self {
            exclude_ips,
        }
    }
}

/// This represents the DHCP option for a logical switch port. Since we don't have the UUID of the
/// table entry until we run orchestration, we need to leave a placeholder to indicate we need to
/// assign the rule to the logical switch port, which will be the hash of `DhcpDatabaseEntry`.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum SwitchPortDhcpOptions {
    DhcpV4,
}

/// This represents the DHCP table entry in OVN. The UUID of this table entry is to be assigned to
/// a logical switch port to activate the DHCP configuration for that port. The entry is based on
/// a router's configuration, as it will be processing the DHCP requests in addition to being the
/// default gateway for the guest on the port. This will be saved in the `OvnNetwork` as a
/// `HashSet`.
#[derive(Debug, Clone, Deserialize, Serialize, Eq, PartialEq, Hash)]
pub struct DhcpDatabaseEntry {
    pub cidr: OvnIpAddr, // must be a subnet
    pub lease_time: String,
    pub router: String, // ip of the router to be set as the default gateway
    pub server_id: String, // ip of the virtual dhcp server
    pub server_mac: MacAddress, // mac of the virtual dhcp server
}

impl DhcpDatabaseEntry {
    pub fn new(
        cidr: OvnIpAddr,
        lease_time: String,
        router: String,
        server_id: String,
        server_mac: MacAddress,
    ) -> Self {
        // TODO - assert cidr is a OvnIpAddr::Subnet
        Self {
            cidr,
            lease_time,
            router,
            server_id,
            server_mac,
        }
    }

    pub fn to_orchestration_resource(
        &self,
    ) -> OrchestrationResource {
        OrchestrationResource::Network(OrchestrationResourceNetworkType::Ovn(OrchestrationResourceNetwork::DhcpOption(self.clone())))
    }
}

fn get_rule_uuid_cmd(
    cidr: &String,
    lease: &String,
    router: &String,
    server_id: &String,
    mac: &String,
    project: &String,
    dns: &String,
) -> Vec<String> {
    vec_of_strings![
        "ovn-nbctl", "--bare", "--columns=_uuid", "find", "dhcp_options",
        format!("cidr=\"{cidr}\""),
        format!("options=\"lease_time\"=\"{lease}\" \"router\"=\"{router}\" \"server_id\"=\"{server_id}\" \"server_mac\"=\"{mac}\" \"dns_server\"=\"{dns}\""),
        format!("external_ids:testbedos-project={project}")
    ]
}

#[async_trait]
impl OvnCommand for DhcpDatabaseEntry {
    async fn create_command<F>(&self, f: impl Fn(Vec<String>, (Option<String>, OrchestrationCommon)) -> F + Send + Sync, config: (Option<String>, OrchestrationCommon)) -> anyhow::Result<String>
        where
            F: Future<Output=anyhow::Result<String>> + Send
    {
        // two things..
        // 1) we need to create the DHCP rule in the database, save the OVN UUID it pops out
        // 2) we then need to take this DHCP rule, hash it, and match it to the rule we save in our
        // OVN state (the hash of the hash set entry) so we can get the right switch port(s) to
        // apply the rule to

        // we await the initial commands here but cascade any errors

        let project_name = &config.1.project_name;
        let external_ids = format!("external_ids:testbedos-project={}", &project_name);

        // lets get all the components related to this create command
        let switch_ports_hashmap = match &config
            .1 // OrchestrationCommon is in second position in tuple
            .network {
            StateNetwork::Ovn(ovn) => &ovn.switch_ports,
            StateNetwork::Ovs(_) => unimplemented!(),
        };
        // get hash of self
        let mut s = DefaultHasher::new();
        self.hash(&mut s);
        let dhcp_self_hash = s.finish();

        // get the switch ports that have this rule
        let switch_ports = {
            let mut dynamic_lsp = Vec::new();
            for (_, lsp) in switch_ports_hashmap {
                if let Some(dhcp_uuid) = lsp.dhcp_options_uuid {
                    if dhcp_uuid == dhcp_self_hash {
                        // this switch port matches the internal UUID of the database rule
                        dynamic_lsp.push(lsp);
                    }
                }
            }
            dynamic_lsp
        };

        // TODO - get uuid in case it exists, and edit that - or look at the DHCP add command that might do it for us?

        // create the rule in OVN, we also add 8.8.8.8 as the DNS server for external access
        tracing::info!("creating DHCP Options database rule cidr: {} router: {}", &self.cidr.to_string(), &self.router);
        let rule_create_res = f(vec_of_strings![
            "ovn-nbctl", "create", "dhcp_options", format!("cidr={}", self.cidr.to_string()),
            format!("options=\"lease_time\"=\"{}\" \"router\"=\"{}\" \"server_id\"=\"{}\" \"server_mac\"=\"{}\" \"dns_server\"=\"{}\"",
                &self.lease_time, &self.router, &self.server_id, &self.server_mac.address.to_string(), "{8.8.8.8}"),
            &external_ids

        ], config.clone()).await;
        // handle any errors manually
        let rule_uuid = match rule_create_res {
            Ok(ok) => ok,
            Err(err) => bail!("could not create DHCP Options database rule cidr: {} router: {}, with err: {err:#}", &self.cidr.to_string(), &self.router),
        };
        // remove newline
        let rule_uuid = if !rule_uuid.eq("") {
            rule_uuid.strip_suffix("\n")
                .context("stripping newline from DHCP options uuid")?.to_string()
        } else {
            rule_uuid
        };
        // take the rule uuid and add to every switch port
        for lsp in switch_ports {
            let cmd = vec_of_strings!["ovn-nbctl", "lsp-set-dhcpv4-options", lsp.name, rule_uuid.clone()];
            f(cmd, config.clone()).await?;
        }

        // just send a result
        Ok("done".to_string())
    }

    async fn destroy_command<F>(&self, f: impl Fn(Vec<String>, (Option<String>, OrchestrationCommon)) -> F + Send + Sync, config: (Option<String>, OrchestrationCommon)) -> anyhow::Result<String>
        where
            F: Future<Output=anyhow::Result<String>> + Send
    {

        // lookup the dhcp_options table
        let uuid_lookup_cmd = f(get_rule_uuid_cmd(
            &self.cidr.to_string(),
            &self.lease_time,
            &self.router,
            &self.server_id,
            &self.server_mac.address.to_string(),
            &config.1.project_name,
            &"{8.8.8.8}".to_string(),
        ), config.clone()).await;
        let rule_uuid = match uuid_lookup_cmd {
            Ok(ok) => ok,
            Err(err) => bail!("could not find the UUID for the DHCP cidr: {} router: {}, with err: {err:#}", &self.cidr.to_string(), &self.router)
        };

        // there could be duplicate rules..
        let rules = if rule_uuid.contains("\n\n") {
            tracing::warn!("multiple uuids returned from dhcp list, meaning there are duplicates for this subnet {} and router {} combo, will delete all", &self.cidr.to_string(), &self.router);
            let rules: Vec<_> = rule_uuid.split("\n\n")
                .map(|s| s.to_string())
                .collect();
            rules
        } else {
            vec![rule_uuid]
        };

        // check if each rule has trailing new line
        let rules: Vec<Result<&str,anyhow::Error>> = rules.iter()
            .map(|s| {
                if s.ends_with("\n") {
                    let intermediate = s.strip_suffix("\n")
                        .context("stripping newline from DHCP options uuid")?;
                    Ok(intermediate)
                } else {
                    Ok(s.as_str())
                }
            })
            .collect();

        for rule in rules {
            f(vec_of_strings!["ovn-nbctl", "destroy", "dhcp_options", rule?.to_string()], config.clone()).await?;
        }

        // remove rule
        Ok("done".into())
    }
}