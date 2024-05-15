use std::fmt;
use std::fmt::Formatter;
use std::future::Future;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use kvm_compose_schemas::kvm_compose_yaml::network::acl::{ACLAction, ACLDirection};
use crate::orchestration::api::{OrchestrationResource, OrchestrationResourceNetwork, OrchestrationResourceNetworkType};
use crate::orchestration::OrchestrationCommon;
use crate::ovn::OvnCommand;
use crate::vec_of_strings;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum ACLRecordType {
    Switch,
    // PortGroup,
}

impl fmt::Display for ACLRecordType {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let text = match self {
            ACLRecordType::Switch => "switch".to_string(),
        };
        f.write_str(&text)
            .expect("Pretty printing ACLRecordType failed");
        Ok(())
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LogicalACLRecord {
    // TODO - do we need to record the switch this is on for any reason?

    /// This would either be the switch or port group name
    pub entity_name: String,

    #[serde(rename = "type")]
    pub _type: ACLRecordType,

    // this is a re-export of `ACLRule` since we don't need anything special here

    pub direction: ACLDirection,
    /// Must be number between 0 and 32,767
    pub priority: i16,
    #[serde(rename = "match")]
    pub _match: String,
    pub action: ACLAction,

    /// Give the ACL rule a human-readable name in the OVN NB database
    pub ovn_resource_name: String,
}

impl LogicalACLRecord {
    pub fn new(
        entity_name: String,
        _type: ACLRecordType,
        direction: ACLDirection,
        priority: i16,
        _match: String,
        action: ACLAction,
        ovn_resource_name: String,
    ) -> Self {
        Self {
            entity_name,
            _type,
            direction,
            priority,
            _match,
            action,
            ovn_resource_name,
        }
    }

    pub fn to_orchestration_resource(
        &self,
    ) -> OrchestrationResource {
        OrchestrationResource::Network(OrchestrationResourceNetworkType::Ovn(OrchestrationResourceNetwork::ACL(self.clone())))
    }
}

#[async_trait]
impl OvnCommand for LogicalACLRecord {
    async fn create_command<F>(&self, f: impl Fn(Vec<String>, (Option<String>, OrchestrationCommon)) -> F + Send + Sync, config: (Option<String>, OrchestrationCommon)) -> anyhow::Result<String>
        where
            F: Future<Output=anyhow::Result<String>> + Send
    {
        tracing::info!("creating ACL on {:?}", &self.entity_name);

        let name = format!("--name={}", &self.ovn_resource_name);
        let cmd = vec_of_strings!["ovn-nbctl", "--may-exist", &name, "acl-add", &self.entity_name, &self.direction, &self.priority, &self._match, &self.action];

        f(cmd, config).await
    }

    async fn destroy_command<F>(&self, f: impl Fn(Vec<String>, (Option<String>, OrchestrationCommon)) -> F + Send + Sync, config: (Option<String>, OrchestrationCommon)) -> anyhow::Result<String>
        where
            F: Future<Output=anyhow::Result<String>> + Send
    {
        tracing::info!("destroying ACL {:?}", &self);

        let cmd = vec_of_strings!["ovn-nbctl", "acl-del",  &self.entity_name, &self.direction, &self.priority, &self._match];

        f(cmd, config).await
    }
}

#[cfg(test)]
mod tests {
    use crate::ovn::test_ovn_run_cmd;
    use super::*;

    #[tokio::test]
    async fn test_logical_acl_record() {
        let record = LogicalACLRecord::new(
            "ovn-sw0".to_string(),
            ACLRecordType::Switch,
            ACLDirection::ToLport,
            10,
            "match".to_string(),
            ACLAction::Drop,
            "resource_name".to_string(),
        );
        let expected_add = vec_of_strings!["ovn-nbctl", "--may-exist", "--name=resource_name", "acl-add", "ovn-sw0", "to-lport", "10", "match", "drop"].join(" ");
        assert_eq!(expected_add, record.create_command(&test_ovn_run_cmd, (None, OrchestrationCommon::default())).await.unwrap());
        let expected_del = vec_of_strings!["ovn-nbctl", "acl-del", "ovn-sw0", "to-lport", "10", "match", "drop"].join(" ");
        assert_eq!(expected_del, record.destroy_command(&test_ovn_run_cmd, (None, OrchestrationCommon::default())).await.unwrap());
    }
}
