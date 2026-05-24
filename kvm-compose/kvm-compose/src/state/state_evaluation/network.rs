use async_trait::async_trait;
use crate::ovn::components::acl::LogicalACLRecord;
use crate::ovn::components::logical_router::LogicalRouter;
use crate::ovn::components::logical_router_port::LogicalRouterPort;
use crate::ovn::components::logical_switch::LogicalSwitch;
use crate::ovn::components::logical_switch_port::LogicalSwitchPort;
use crate::ovn::components::ovs::OvsPort;
use crate::ovn::configuration::dhcp::DhcpDatabaseEntry;
use crate::state::state_evaluation::EvaluateState;

#[async_trait]
impl EvaluateState for LogicalSwitch {
    async fn get_check_command(&self, _: String) -> String {
        format!(
            "ovn-nbctl --bare get Logical_Switch \"{}\" _uuid >/dev/null 2>&1 && echo \"up\" || echo \"does_not_exist\"",
            self.name,
        )
    }
}

#[async_trait]
impl EvaluateState for LogicalSwitchPort {
    async fn get_check_command(&self, _: String) -> String {
        format!(
            "STATE=$(ovn-nbctl --bare get Logical_Switch_Port \"{}\" up 2>/dev/null); if [ -z \"$STATE\" ]; then echo \"does_not_exist\"; elif [ \"$STATE\" = \"true\" ]; then echo \"up\"; else echo \"down\"; fi",
            self.name,
        )
    }
}

#[async_trait]
impl EvaluateState for LogicalRouter {
    async fn get_check_command(&self, _: String) -> String {
        format!(
            "ovn-nbctl --bare get Logical_Router \"{}\" _uuid >/dev/null 2>&1 && echo \"up\" || echo \"does_not_exist\"",
            self.name,
        )
    }
}

#[async_trait]
impl EvaluateState for LogicalRouterPort {
    async fn get_check_command(&self, _: String) -> String {
        format!(
            "ovn-nbctl --bare get Logical_Router_Port \"{}\" _uuid >/dev/null 2>&1 && echo \"up\" || echo \"does_not_exist\"",
            self.name,
        )
    }
}

#[async_trait]
impl EvaluateState for OvsPort {
    async fn get_check_command(&self, _: String) -> String {
        format!(
            "ovs-vsctl --bare get Interface \"{}\" admin_state 2>/dev/null || echo \"does_not_exist\"",
            self.name,
        )
    }
}

#[async_trait]
impl EvaluateState for LogicalACLRecord {
    async fn get_check_command(&self, _: String) -> String {
        format!(
            "ovn-nbctl --bare get ACL \"{}\" _uuid >/dev/null 2>&1 && echo \"active\" || echo \"does_not_exist\"",
            self.ovn_resource_name, // TODO - this should be the UUID
        )
    }
}

#[async_trait]
impl EvaluateState for DhcpDatabaseEntry {
    async fn get_check_command(&self, _: String) -> String {
        format!(
            "ovn-nbctl --bare get DHCP_Options \"{}\" _uuid >/dev/null 2>&1 && echo \"active\" || echo \"does_not_exist\"",
            self.lease_time, // TODO - this should be uuid?
        )
    }
}
