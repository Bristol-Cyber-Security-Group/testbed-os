use std::collections::HashSet;
use std::path::PathBuf;
use anyhow::{bail, Context};
use async_trait::async_trait;
use reqwest::Client;
use tokio::sync::mpsc::{Sender};
use kvm_compose_schemas::deployment_models::{Deployment, DeploymentCommand};
use crate::components::LogicalTestbed;
use crate::components::network::LogicalNetwork;
use crate::orchestration::{OrchestrationCommon, OrchestrationTask, read_previous_state_request, run_subprocess_command, run_subprocess_command_allow_fail, run_testbed_orchestration_command, run_testbed_orchestration_command_allow_fail};
use crate::orchestration::api::*;
use crate::orchestration::websocket::{send_orchestration_instruction_over_channel};
use crate::ovn::OvnCommand;
use crate::state::{State, StateNetwork};

/// This is a pre-prepared command running function inside the Fn closure that is sent to each OVN
/// component's `OvnCommand` implementation. It takes in the command string from `OvnCommand` and
/// passes it into the pre-prepared function. If `remote_config` is Some, then the command can be
/// executed on a remote testbed host.
pub async fn ovn_run_cmd(
    cmd: Vec<String>,
    remote_config: (Option<String>, OrchestrationCommon),
) -> anyhow::Result<String> {
    // convert input to Vec<&str>
    let cmd = cmd.iter()
        .map(|x| x.as_str())
        .collect();
    if let Some(testbed_name) = remote_config.0 {
        let res = run_testbed_orchestration_command(
            &remote_config.1,
            &testbed_name,
            "sudo",
            cmd,
            false,
            None,
        ).await;
        res
    } else {
        let res = run_subprocess_command(
            "sudo",
            cmd,
            false,
            None,
        ).await;
        res
    }
}

/// This is a pre-prepared command running function inside the Fn closure that is sent to each OVN
/// component's `OvnCommand` implementation. It takes in the command string from `OvnCommand` and
/// passes it into the pre-prepared function. If `remote_config` is Some, then the command can be
/// executed on a remote testbed host. Allow failures version of `ovn_run_cmd`.
pub async fn ovn_run_cmd_allow_fail(
    cmd: Vec<String>,
    remote_config: (Option<String>, OrchestrationCommon),
) -> anyhow::Result<String> {
    // convert input to Vec<&str>
    let cmd = cmd.iter()
        .map(|x| x.as_str())
        .collect();
    // allow fail as consecutive up/down could mean we try to do something twice
    if let Some(testbed_name) = remote_config.0 {
        let res = run_testbed_orchestration_command_allow_fail(
            &remote_config.1,
            &testbed_name,
            "sudo",
            cmd,
            false,
            None,
        ).await;
        res
    } else {
        let res = run_subprocess_command_allow_fail(
            "sudo",
            cmd,
            false,
            None,
        ).await;
        res
    }
}

#[async_trait]
impl OrchestrationTask for StateNetwork {
    async fn create_action(&self, common: &OrchestrationCommon) -> anyhow::Result<()> {
        tracing::info!("deploying OVN components");

        // create all OVN resources
        match &self {
            StateNetwork::Ovn(ovn_state) => {
                for (_, ls_data) in &ovn_state.switches {
                    ls_data.create_command(&ovn_run_cmd, (None, common.clone())).await?;
                }
                for (_, lsp_data) in &ovn_state.switch_ports {
                    lsp_data.create_command(&ovn_run_cmd, (None, common.clone())).await?;
                }
                for (_, lr_data) in &ovn_state.routers {
                    lr_data.create_command(&ovn_run_cmd, (None, common.clone())).await?;
                }
                for (_, lrp_data) in &ovn_state.router_ports {
                    lrp_data.create_command(&ovn_run_cmd, (None, common.clone())).await?;
                }
                for (_, lrp_data) in &ovn_state.ovs_ports {
                    // OVS ports may need to be created on remote testbed hosts
                    lrp_data.create_command(
                        &ovn_run_cmd,
                        (
                            Some(chassis_to_tb_host(&lrp_data.chassis, &common)?),
                            common.clone(),
                        )
                    ).await?;
                }
                for (_, router) in &ovn_state.routers {
                    for (_, route) in &router.routing.0 {
                        route.create_command(&ovn_run_cmd, (None, common.clone())).await?;
                    }
                }
                for (_, router) in &ovn_state.routers {
                    for (_, ext) in &router.external_gateway.0 {
                        ext.create_command(&ovn_run_cmd, (None, common.clone())).await?;
                    }
                }
                for (_, router) in &ovn_state.routers {
                    for (_, nat) in &router.nat.0 {
                        nat.create_command(&ovn_run_cmd, (None, common.clone())).await?;
                    }
                }
                for dhcp in &ovn_state.dhcp_options {
                    // we want to send common which has the OVN data structure, will repurpose the
                    // "remote_config" for this - should probably consider renaming it
                    dhcp.create_command(&ovn_run_cmd, (None, common.clone())).await?;
                }

            }
            StateNetwork::Ovs(_) => unimplemented!(),
        }

        Ok(())
    }

    async fn destroy_action(&self, common: &OrchestrationCommon) -> anyhow::Result<()> {
        tracing::info!("destroying OVN components");

        // destroy all OVN resources
        match &self {
            StateNetwork::Ovn(ovn_state) => {
                for dhcp in &ovn_state.dhcp_options {
                    dhcp.destroy_command(&ovn_run_cmd_allow_fail, (None, common.clone())).await?;
                }
                for (_, router) in &ovn_state.routers {
                    for (_, route) in &router.routing.0 {
                        route.destroy_command(&ovn_run_cmd_allow_fail, (None, common.clone())).await?;
                    }
                }
                for (_, router) in &ovn_state.routers {
                    for (_, ext) in &router.external_gateway.0 {
                        ext.destroy_command(&ovn_run_cmd_allow_fail, (None, common.clone())).await?;
                    }
                }
                for (_, router) in &ovn_state.routers {
                    for (_, nat) in &router.nat.0 {
                        nat.destroy_command(&ovn_run_cmd_allow_fail, (None, common.clone())).await?;
                    }
                }
                for (_, lsp_data) in &ovn_state.switch_ports {
                    lsp_data.destroy_command(&ovn_run_cmd_allow_fail, (None, common.clone())).await?;
                }
                for (_, lrp_data) in &ovn_state.router_ports {
                    lrp_data.destroy_command(&ovn_run_cmd_allow_fail, (None, common.clone())).await?;
                }
                for (_, sw_data) in &ovn_state.switches {
                    sw_data.destroy_command(&ovn_run_cmd_allow_fail, (None, common.clone())).await?;
                }
                for (_, lr_data) in &ovn_state.routers {
                    lr_data.destroy_command(&ovn_run_cmd_allow_fail, (None, common.clone())).await?;
                }
                for (_, lrp_data) in &ovn_state.ovs_ports {
                    // OVS ports may need to be destroyed on remote testbed hosts
                    lrp_data.destroy_command(
                        &ovn_run_cmd_allow_fail,
                        (
                            Some(chassis_to_tb_host(&lrp_data.chassis, &common)?),
                            common.clone(),
                        )).await?;
                }
            }
            StateNetwork::Ovs(_) => unimplemented!(),
        }

        Ok(())
    }

    async fn request_create_action(&self, _common: &OrchestrationCommon, sender: &mut Sender<OrchestrationProtocol>) -> anyhow::Result<()> {
        tracing::info!("requesting to deploy OVN components");
        // no need to batch these as OVN is quick to create resources

        // create all OVN resources
        match &self {
            StateNetwork::Ovn(ovn_state) => {
                for (_, ls_data) in &ovn_state.switches {
                    send_orchestration_instruction_over_channel(
                        sender,
                        // receiver,
                        OrchestrationInstruction::Deploy(vec![ls_data.to_orchestration_resource()]),
                    ).await.context("requesting the creation of logical switch")?;

                }
                for (_, lsp_data) in &ovn_state.switch_ports {
                    send_orchestration_instruction_over_channel(
                        sender,
                        // receiver,
                        OrchestrationInstruction::Deploy(vec![lsp_data.to_orchestration_resource()]),
                    ).await.context("requesting the creation of logical switch port")?;
                }
                for (_, lr_data) in &ovn_state.routers {
                    send_orchestration_instruction_over_channel(
                        sender,
                        // receiver,
                        OrchestrationInstruction::Deploy(vec![lr_data.to_orchestration_resource()]),
                    ).await.context("requesting the creation of logical router")?;
                }
                for (_, lrp_data) in &ovn_state.router_ports {
                    send_orchestration_instruction_over_channel(
                        sender,
                        // receiver,
                        OrchestrationInstruction::Deploy(vec![lrp_data.to_orchestration_resource()]),
                    ).await.context("requesting the creation of logical router port")?;
                }
                for (_, ovs_data) in &ovn_state.ovs_ports {
                    send_orchestration_instruction_over_channel(
                        sender,
                        // receiver,
                        OrchestrationInstruction::Deploy(vec![ovs_data.to_orchestration_resource()]),
                    ).await.context("requesting the creation of ovs port")?;
                }
                for (_, router) in &ovn_state.routers {
                    for (_, route) in &router.routing.0 {
                        send_orchestration_instruction_over_channel(
                            sender,
                            // receiver,
                            OrchestrationInstruction::Deploy(vec![route.to_orchestration_resource()]),
                        ).await.context("requesting the creation of static route")?;
                    }
                }
                for (_, router) in &ovn_state.routers {
                    for (_, ext) in &router.external_gateway.0 {
                        send_orchestration_instruction_over_channel(
                            sender,
                            // receiver,
                            OrchestrationInstruction::Deploy(vec![ext.to_orchestration_resource()]),
                        ).await.context("requesting the creation of external gateway")?;
                    }
                }
                for (_, router) in &ovn_state.routers {
                    for (_, nat) in &router.nat.0 {
                        send_orchestration_instruction_over_channel(
                            sender,
                            // receiver,
                            OrchestrationInstruction::Deploy(vec![nat.to_orchestration_resource()]),
                        ).await.context("requesting the creation of nat rule")?;
                    }
                }
                for dhcp in &ovn_state.dhcp_options {
                    send_orchestration_instruction_over_channel(
                        sender,
                        // receiver,
                        OrchestrationInstruction::Deploy(vec![dhcp.to_orchestration_resource()]),
                    ).await.context("requesting the creation of dhcp rule")?;
                }
                for (_, acl_record) in &ovn_state.acl {
                    send_orchestration_instruction_over_channel(
                        sender,
                        OrchestrationInstruction::Deploy(vec![acl_record.to_orchestration_resource()]),
                    ).await.context("requesting the creation of ACL")?;
                }
            }
            StateNetwork::Ovs(_) => unimplemented!(),
        }

        Ok(())
    }

    async fn request_destroy_action(&self, _common: &OrchestrationCommon, sender: &mut Sender<OrchestrationProtocol>) -> anyhow::Result<()> {
        tracing::info!("requesting to destroy OVN components");
        // no need to batch these as OVN is quick to create resources
        match &self {
            StateNetwork::Ovn(ovn_state) => {
                for (_, acl_record) in &ovn_state.acl {
                    send_orchestration_instruction_over_channel(
                        sender,
                        OrchestrationInstruction::Destroy(vec![acl_record.to_orchestration_resource()]),
                    ).await.context("requesting the destruction of ACL")?;
                }
                for (_, ls_data) in &ovn_state.switches {
                    send_orchestration_instruction_over_channel(
                        sender,
                        // receiver,
                        OrchestrationInstruction::Destroy(vec![ls_data.to_orchestration_resource()]),
                    ).await.context("requesting the destruction of logical switch")?;

                }
                for (_, lsp_data) in &ovn_state.switch_ports {
                    send_orchestration_instruction_over_channel(
                        sender,
                        // receiver,
                        OrchestrationInstruction::Destroy(vec![lsp_data.to_orchestration_resource()]),
                    ).await.context("requesting the destruction of logical switch port")?;
                }
                for (_, lr_data) in &ovn_state.routers {
                    send_orchestration_instruction_over_channel(
                        sender,
                        // receiver,
                        OrchestrationInstruction::Destroy(vec![lr_data.to_orchestration_resource()]),
                    ).await.context("requesting the destruction of logical router")?;
                }
                for (_, lrp_data) in &ovn_state.router_ports {
                    send_orchestration_instruction_over_channel(
                        sender,
                        // receiver,
                        OrchestrationInstruction::Destroy(vec![lrp_data.to_orchestration_resource()]),
                    ).await.context("requesting the destruction of logical router port")?;
                }
                for (_, router) in &ovn_state.routers {
                    for (_, route) in &router.routing.0 {
                        send_orchestration_instruction_over_channel(
                            sender,
                            // receiver,
                            OrchestrationInstruction::Destroy(vec![route.to_orchestration_resource()]),
                        ).await.context("requesting the destruction of static route")?;
                    }
                }
                for (_, router) in &ovn_state.routers {
                    for (_, ext) in &router.external_gateway.0 {
                        send_orchestration_instruction_over_channel(
                            sender,
                            // receiver,
                            OrchestrationInstruction::Destroy(vec![ext.to_orchestration_resource()]),
                        ).await.context("requesting the destruction of external gateway")?;
                    }
                }
                for (_, router) in &ovn_state.routers {
                    for (_, nat) in &router.nat.0 {
                        send_orchestration_instruction_over_channel(
                            sender,
                            // receiver,
                            OrchestrationInstruction::Destroy(vec![nat.to_orchestration_resource()]),
                        ).await.context("requesting the destruction of nat rule")?;
                    }
                }
                for dhcp in &ovn_state.dhcp_options {
                    send_orchestration_instruction_over_channel(
                        sender,
                        // receiver,
                        OrchestrationInstruction::Destroy(vec![dhcp.to_orchestration_resource()]),
                    ).await.context("requesting the destruction of dhcp rule")?;
                }
                for (_, ovs_data) in &ovn_state.ovs_ports {
                    send_orchestration_instruction_over_channel(
                        sender,
                        // receiver,
                        OrchestrationInstruction::Destroy(vec![ovs_data.to_orchestration_resource()]),
                    ).await.context("requesting the destruction of ovs port")?;
                }

            }
            StateNetwork::Ovs(_) => unimplemented!(),
        }

        Ok(())
    }
}

pub async fn reapply_acl_action(
    current_state: &State,
    logical_testbed: LogicalTestbed,
    sender: &mut Sender<OrchestrationProtocol>,
    deployment: Deployment,
    command: DeploymentCommand,
    project_name: String,
    project_location: PathBuf,
    http_client: &Client,
    server_conn: &String,
) -> anyhow::Result<()> {
    // TODO - There is an assumption that only the ACL rules were changed, this needs to be
    //  handled properly when implementing delta changes

    // get ovn logical representation from state
    let current_network = match &current_state.network {
        StateNetwork::Ovn(n) => n,
        StateNetwork::Ovs(_) => bail!("acl not implemented for ovs"), // will need to refactor if implementing ovs
    };

    // get ovn logical representation from yaml, this has to be OVN for now
    let Some(LogicalNetwork::Ovn(ref new_network)) = logical_testbed.network else {bail!("no OVN network defined in yaml")};

    // we compare the existing switch list, and the new yaml acl switch list so that the new acl
    // list doesn't try to place rules on switches that don't already exist
    // TODO - what do we compare here to make sure the network is still the same, just the switch names under ACL section?
    let current_switch_list: HashSet<_> = current_network.switches
        .iter()
        .map(|(sw, _)| {
            sw
        })
        .collect();
    let yaml_acl_switch_list: HashSet<_> = new_network.acl
        .iter()
        .map(|(_, acl)| {
            &acl.entity_name
        })
        .collect();

    ensure_yaml_acl_switches_already_exist(current_switch_list, yaml_acl_switch_list)?;

    send_orchestration_instruction_over_channel(
        sender,
        OrchestrationInstruction::Init {
            deployment: deployment.clone(),
            deployment_command: command.clone(),
        },
    ).await.context("sending Init request to server")?;

    // if Ok, then destroy current acl rules, then create the new ones
    match &current_state.network {
        StateNetwork::Ovn(ovn_state) => {
            for (_, acl_record) in &ovn_state.acl {
                send_orchestration_instruction_over_channel(
                    sender,
                    OrchestrationInstruction::Destroy(vec![acl_record.to_orchestration_resource()]),
                ).await.context("requesting the destruction of ACL")?;
            }
        }
        StateNetwork::Ovs(_) => unimplemented!(),
    }

    // create a new state
    let new_state = State::new(&logical_testbed)
        .context("Creating state from logical testbed")?;
    match &new_state.network {
        StateNetwork::Ovn(ovn_state) => {
            for (_, acl_record) in &ovn_state.acl {
                send_orchestration_instruction_over_channel(
                    sender,
                    OrchestrationInstruction::Deploy(vec![acl_record.to_orchestration_resource()]),
                ).await.context("requesting the creation of ACL")?;
            }
        }
        StateNetwork::Ovs(_) => unimplemented!(),
    }

    // if successful update the current state, only the ACL part, then save to disk
    let mut previous_state = read_previous_state_request(&http_client, &server_conn, &project_name).await?;
    previous_state.network = new_state.network;
    previous_state
        .write(&project_name, &project_location)
        .await
        .context("Writing the state json file.")?;

    Ok(())
}

fn ensure_yaml_acl_switches_already_exist(
    current_switch_list: HashSet<&String>,
    yaml_acl_switch_list: HashSet<&String>
) -> anyhow::Result<()> {

    let difference: HashSet<_> = yaml_acl_switch_list
        .difference(&current_switch_list)
        .collect();
    if difference.len() > 0 {
        bail!("there are switches in the ACL list that don't exist in the current state")
    }

    Ok(())
}

pub fn chassis_to_tb_host(chassis: &String, orchestration_common: &OrchestrationCommon) -> anyhow::Result<String> {
    for (name, host) in &orchestration_common.kvm_compose_config.testbed_host_ssh_config {
        if chassis.eq(&host.ovn.chassis_name) {
            return Ok(name.clone())
        }
    }
    bail!("could not find chassis {chassis} in kvm-compose-config");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_yaml_acl_switches_already_exist_use_some() -> anyhow::Result<()> {
        // acl list will use some of the existing switches
        let mut current_switches = HashSet::new();
        let sw = "sw0".to_string();
        current_switches.insert(&sw);
        let sw = "sw1".to_string();
        current_switches.insert(&sw);
        let sw = "sw2".to_string();
        current_switches.insert(&sw);
        // acl list can use any of the above
        let mut acl_list = HashSet::new();
        let sw = "sw0".to_string();
        acl_list.insert(&sw);
        ensure_yaml_acl_switches_already_exist(current_switches, acl_list)?;
        Ok(())
    }

    #[test]
    fn test_validate_yaml_acl_switches_already_exist_use_exact() -> anyhow::Result<()> {
        // acl list will use the exact number of existing switches
        let mut current_switches = HashSet::new();
        let sw = "sw0".to_string();
        current_switches.insert(&sw);
        let sw = "sw1".to_string();
        current_switches.insert(&sw);
        let sw = "sw2".to_string();
        current_switches.insert(&sw);
        // acl list can use any of the above
        let mut acl_list = HashSet::new();
        let sw = "sw0".to_string();
        acl_list.insert(&sw);
        let sw = "sw1".to_string();
        acl_list.insert(&sw);
        let sw = "sw2".to_string();
        acl_list.insert(&sw);
        ensure_yaml_acl_switches_already_exist(current_switches, acl_list)?;
        Ok(())
    }

    #[test]
    fn test_validate_yaml_acl_switches_already_exist_use_more() -> anyhow::Result<()> {
        // acl list will use switches that don't exist in the current switch list
        let mut current_switches = HashSet::new();
        let sw = "sw0".to_string();
        current_switches.insert(&sw);
        let sw = "sw1".to_string();
        current_switches.insert(&sw);
        let sw = "sw2".to_string();
        current_switches.insert(&sw);
        // acl list can use any of the above
        let mut acl_list = HashSet::new();
        let sw = "sw3".to_string();
        acl_list.insert(&sw);
        let res = ensure_yaml_acl_switches_already_exist(current_switches, acl_list);
        match res {
            Ok(_) => bail!("should not have been Ok"),
            Err(_) => Ok(()),
        }
    }

}
