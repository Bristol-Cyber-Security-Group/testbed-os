use anyhow::{bail, Context};
use async_trait::async_trait;
use tokio::sync::mpsc::{Sender};
use crate::orchestration::{OrchestrationCommon, OrchestrationTask, run_subprocess_command, run_subprocess_command_allow_fail, run_testbed_orchestration_command, run_testbed_orchestration_command_allow_fail};
use crate::orchestration::api::*;
use crate::orchestration::websocket::{send_orchestration_instruction_over_channel};
use crate::ovn::OvnCommand;
use crate::state::StateNetwork;

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

pub fn chassis_to_tb_host(chassis: &String, orchestration_common: &OrchestrationCommon) -> anyhow::Result<String> {
    for (name, host) in &orchestration_common.kvm_compose_config.testbed_host_ssh_config {
        if chassis.eq(&host.ovn.chassis_name) {
            return Ok(name.clone())
        }
    }
    bail!("could not find chassis {chassis} in kvm-compose-config");
}
