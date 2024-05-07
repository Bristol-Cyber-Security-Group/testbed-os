// use anyhow::{bail, Context};
// use async_trait::async_trait;
// use virt::connect::Connect;
// use virt::network::Network;
// use crate::orchestration::{OrchestrationCommon, OrchestrationTask, run_subprocess_command, run_subprocess_command_allow_fail, run_testbed_orchestration_command, run_testbed_orchestration_command_allow_fail};
// use crate::state::{StateOvsNetwork};
// use crate::state::load_balancing::BridgeConnection;

// TODO - leaving this as we may need it again once we reimplement OVS network backend

// // this creates the base libvirt network
// #[async_trait]
// impl OrchestrationTask for StateOvsNetwork {
//     async fn create_action(&self, common: OrchestrationCommon) -> anyhow::Result<()> {
//         tracing::info!("deploying libvirt network");
//
//         // scope the libvirt connection so that it is dropped when exited scopr as it is not thread safe
//         let create_libvirt_network = async {
//             let conn = Connect::open("qemu:///system")
//                 .context("Connecting to qemu:///system").expect("Connecting to libvirt");
//             let res = Network::define_xml(&conn, &self.libvirt_network_xml);
//             match res {
//                 Ok(_) => Ok(()),
//                 Err(err) => {
//                     tracing::warn!("error in defining network, probably already exists, error: {:#}", err);
//                     let expected_err = "already exists with uuid".to_string();
//                     if !err.to_string().trim().contains(&expected_err) {
//                         // error in joining blocking thread
//                         Err(err)
//                     } else {
//                         Ok(())
//                     }
//
//                 }
//             }
//         }.await;
//         match create_libvirt_network {
//             Ok(..) => {}
//             Err(e) => {
//                 // skip error if already defined
//                 let expected_err = "already exists with uuid".to_string();
//
//                 if !e.to_string().trim().contains(&expected_err) {
//                     // error in joining blocking thread
//                     tracing::error!("{:#}", e);
//                     bail!("could not define network in libvirt, quitting")
//                 }
//             }
//         }
//
//         let project_name = &common.project_name;
//         let libvirt_network_name = format!("{project_name}-network");
//         let br_name = &self.livbirt_network_bridge_name;
//         let veth_peer_left = format!("{project_name}-veth0");
//         let veth_peer_right = format!("{project_name}-veth1");
//         // run the commands
//         let cmd = vec!["virsh", "net-start", &libvirt_network_name];
//         run_subprocess_command_allow_fail(
//             "sudo",
//             cmd,
//             false).await?;
//         let cmd = vec!["virsh", "net-autostart", &libvirt_network_name];
//         run_subprocess_command(
//             "sudo",
//             cmd,
//             false).await?;
//         // might exist
//         let cmd = vec!["ip", "link", "add", &veth_peer_left, "type", "veth", "peer", "name", &veth_peer_right];
//         run_subprocess_command_allow_fail(
//             "sudo",
//             cmd,
//             false).await?;
//         let cmd = vec!["ip", "link", "set", "up", "dev", &veth_peer_left];
//         run_subprocess_command_allow_fail(
//             "sudo",
//             cmd,
//             false).await?;
//         let cmd = vec!["ip", "link", "set", "up", "dev", &veth_peer_right];
//         run_subprocess_command_allow_fail(
//             "sudo",
//             cmd,
//             false).await?;
//         // continue
//         let cmd = vec!["ip", "link", "set", &veth_peer_right, "master", &br_name];
//         run_subprocess_command_allow_fail(
//             "sudo",
//             cmd,
//             false).await?;
//
//         tracing::info!("finished deploying libvirt network");
//
//         Ok(())
//
//     }
//
//     async fn destroy_action(&self, common: OrchestrationCommon) -> anyhow::Result<()> {
//
//         tracing::info!("destroying libvirt network");
//
//         let project_name = &common.project_name;
//         let libvirt_network_name = format!("{project_name}-network");
//         let veth_peer_left = format!("{project_name}-veth0");
//         // run delete commands for libvirt network
//         let cmd = vec!["ip", "link", "delete", &veth_peer_left];
//         run_subprocess_command_allow_fail(
//             "sudo",
//             cmd,
//             false).await?;
//         let cmd = vec!["virsh", "net-destroy", &libvirt_network_name];
//         run_subprocess_command_allow_fail(
//             "sudo",
//             cmd,
//             false).await?;
//         let cmd = vec!["virsh", "net-undefine", &libvirt_network_name];
//         run_subprocess_command_allow_fail(
//             "sudo",
//             cmd,
//             false).await?;
//
//         tracing::info!("finished destroying libvirt network");
//
//         Ok(())
//     }
// }


// // this creates the OVS bridges
// #[async_trait]
// impl OrchestrationTask for StateTestbedInterface {
//     async fn create_action(&self, common: OrchestrationCommon) -> anyhow::Result<()> {
//
//         let bridge_name = &self.name;
//         let project_name = &common.project_name;
//         let ovs_bridge_name = format!("{project_name}-{bridge_name}");
//         let testbed_host = self.testbed_host.as_ref().unwrap();
//         let testbed_host_ip = common.testbed_hosts.get(testbed_host)
//             .expect("could not find testbed host config for bridge")
//             .ip.clone();
//         let controller = format!("tcp:{testbed_host_ip}:6653");
//
//         // create bridge
//         let cmd = vec!["ovs-vsctl", "--may-exist", "add-br", &ovs_bridge_name];
//         run_testbed_orchestration_command(
//             &common,
//             testbed_host,
//             "sudo",
//             cmd,
//             false).await?;
//         // set openflow protocol
//         let cmd = vec!["ovs-vsctl", "set", "bridge", &ovs_bridge_name, "protocol=OpenFlow13"];
//         run_testbed_orchestration_command(
//             &common,
//             testbed_host,
//             "sudo",
//             cmd,
//             false).await?;
//         // set controller
//         let cmd = vec!["ovs-vsctl", "set-controller", &ovs_bridge_name, &controller];
//         run_testbed_orchestration_command(
//             &common,
//             testbed_host,
//             "sudo",
//             cmd,
//             false).await?;
//
//         Ok(())
//     }
//
//     async fn destroy_action(&self, common: OrchestrationCommon) -> anyhow::Result<()> {
//
//         // let bridge_name = &self.name;
//         // let project_name = &common.project_name;
//         // let ovs_bridge_name = format!("{project_name}-{bridge_name}");
//         // let testbed_host = self.testbed_host.as_ref().unwrap();
//         //
//         // if self.name.eq(&common.external_bridge) {
//         //     let veth_peer_left = format!("{project_name}-veth0");
//         //
//         //     let cmd = vec!["ovs-vsctl", "del-port", &ovs_bridge_name, &veth_peer_left];
//         //     run_testbed_orchestration_command_allow_fail(
//         //         &common,
//         //         testbed_host,
//         //         "sudo",
//         //         cmd,
//         //         false).await?;
//         // }
//         //
//         // let cmd = vec!["ovs-vsctl", "del-br", &ovs_bridge_name];
//         // run_testbed_orchestration_command_allow_fail(
//         //     &common,
//         //     testbed_host,
//         //     "sudo",
//         //     cmd,
//         //     false).await?;
//
//         Ok(())
//     }
// }
//
// // this creates the connections between OVS bridges, if across testbed hosts
// #[async_trait]
// impl OrchestrationTask for PhysicalBridgeConnections {
//     async fn create_action(&self, common: OrchestrationCommon) -> anyhow::Result<()> {
//
//         // loop through all the testbed hosts
//         for (logical_host, testbed_host_data) in common.testbed_hosts.iter() {
//             let testbed_host = logical_host;
//             let testbed_host_nic = &testbed_host_data.testbed_nic;
//
//             // count geneve interfaces per host to make sure all interfaces on the ovs tunnel with the geneve tunnel are unique
//             let mut geneve_interface_counter = 0;
//
//             // each connection may or may not be a tunnel
//             for (_, connection_info) in self.0.as_ref().unwrap().iter() {
//                 match connection_info {
//                     BridgeConnection::Ovs { source_br, target_br, source_veth, target_veth, testbed_host, .. } => {
//                         let cmd = vec!["ip", "link", "add", &source_veth, "type", "veth", "peer", "name", &target_veth];
//                         run_testbed_orchestration_command(
//                             &common,
//                             testbed_host,
//                             "sudo",
//                             cmd,
//                             false).await?;
//
//                         // bring up bridges
//                         let cmd = vec!["ip", "link", "set", "up", "dev", &source_veth];
//                         run_testbed_orchestration_command(
//                             &common,
//                             testbed_host,
//                             "sudo",
//                             cmd,
//                             false).await?;
//                         let cmd = vec!["ip", "link", "set", "up", "dev", &target_veth];
//                         run_testbed_orchestration_command(
//                             &common,
//                             testbed_host,
//                             "sudo",
//                             cmd,
//                             false).await?;
//                         // add ports to bridges
//                         let cmd = vec!["ovs-vsctl", "--may-exist", "add-port", &source_br, &source_veth];
//                         run_testbed_orchestration_command(
//                             &common,
//                             testbed_host,
//                             "sudo",
//                             cmd,
//                             false).await?;
//
//                         let cmd = vec!["ovs-vsctl", "--may-exist", "add-port", &target_br, &target_veth];
//                         run_testbed_orchestration_command(
//                             &common,
//                             testbed_host,
//                             "sudo",
//                             cmd,
//                             false).await?;
//                     }
//                     BridgeConnection::Tunnel { source_br, target_br, source_remote_ip, target_remote_ip, key, testbed_host_source, testbed_host_target, .. } => {
//                         let key_res = key.as_ref().unwrap();
//                         if testbed_host_source.eq(testbed_host) {
//                             let src_remote_ip = source_remote_ip.as_ref().unwrap();
//                             let src_nic = &testbed_host_nic;
//                             // target remote ip here is the local (to the host in iteration) unique tunnel ip
//                             let local_tunnel_ip = target_remote_ip.as_ref().unwrap();
//                             let local_tunnel_ip_wmask = format!("{local_tunnel_ip}/24");
//
//                             let cmd = vec!["ip", "addr", "add", &local_tunnel_ip_wmask, "dev", &src_nic];
//                             let ip_add_res = run_testbed_orchestration_command(
//                                 &common,
//                                 testbed_host,
//                                 "sudo",
//                                 cmd,
//                                 false).await;
//                             if ip_add_res.is_err() {
//                                 let err = ip_add_res.unwrap_err();
//                                 let expected_err = "RTNETLINK answers: File exists".to_string();
//                                 if !err.to_string().trim().contains(&expected_err) {
//                                     // if not this expected error
//                                     tracing::error!("{}", err.to_string());
//                                     bail!("could not set ip on NIC")
//                                 } else {
//                                     tracing::warn!("ignoring ip address add error (RTNETLINK answers: File exists)")
//                                 }
//                             }
//
//                             let geneve_geneve_interface_counter = format!("geneve{geneve_interface_counter}");
//                             let options_remote_ip = format!("options:remote_ip={src_remote_ip}");
//                             let options_key = format!("options:key={key_res}");
//                             let cmd = vec!["ovs-vsctl", "--may-exist", "add-port", &source_br, &geneve_geneve_interface_counter, "--", "set", "interface", &geneve_geneve_interface_counter, "type=geneve", &options_remote_ip, &options_key];
//                             run_testbed_orchestration_command(
//                                 &common,
//                                 testbed_host,
//                                 "sudo",
//                                 cmd,
//                                 false).await?;
//
//                             let cmd = vec!["ip", "link", "set", &src_nic, "mtu", "2000"];
//                             run_testbed_orchestration_command(
//                                 &common,
//                                 testbed_host,
//                                 "sudo",
//                                 cmd,
//                                 false).await?;
//                             geneve_interface_counter += 1;
//                         }
//                         // configure "right" of tunnel
//                         if testbed_host_target.eq(testbed_host) {
//                             let tgt_remote_ip = target_remote_ip.as_ref().unwrap();
//                             let tgt_nic = &testbed_host_nic;
//                             // source remote ip here is the local (to the host in iteration) unique tunnel ip
//                             let local_tunnel_ip = source_remote_ip.as_ref().unwrap();
//
//                             let local_tunnel_ip_wmask = format!("{local_tunnel_ip}/24");
//                             let cmd = vec!["ip", "addr", "add", &local_tunnel_ip_wmask, "dev", &tgt_nic];
//                             let ip_add_res = run_testbed_orchestration_command(
//                                 &common,
//                                 testbed_host,
//                                 "sudo",
//                                 cmd,
//                                 false).await;
//                             if ip_add_res.is_err() {
//                                 let err = ip_add_res.unwrap_err();
//                                 let expected_err = "RTNETLINK answers: File exists".to_string();
//                                 if !err.to_string().trim().contains(&expected_err) {
//                                     // if not this expected error
//                                     tracing::error!("{}", err.to_string());
//                                     bail!("could not set ip on NIC")
//                                 } else {
//                                     tracing::warn!("ignoring ip address add error (RTNETLINK answers: File exists)")
//                                 }
//                             }
//
//                             let geneve_geneve_interface_counter = format!("geneve{geneve_interface_counter}");
//                             let options_remote_ip = format!("options:remote_ip={tgt_remote_ip}");
//                             let options_key = format!("options:key={key_res}");
//                             let cmd = vec!["ovs-vsctl", "--may-exist", "add-port", &target_br, &geneve_geneve_interface_counter, "--", "set", "interface", &geneve_geneve_interface_counter, "type=geneve", &options_remote_ip, &options_key];
//                             run_testbed_orchestration_command(
//                                 &common,
//                                 testbed_host,
//                                 "sudo",
//                                 cmd,
//                                 false).await?;
//
//                             let cmd = vec!["ip", "link", "set", &tgt_nic, "mtu", "2000"];
//                             run_testbed_orchestration_command(
//                                 &common,
//                                 testbed_host,
//                                 "sudo",
//                                 cmd,
//                                 false).await?;
//
//                             geneve_interface_counter += 1;
//                         }
//                     }
//                 }
//             }
//         }
//
//         Ok(())
//     }
//
//     async fn destroy_action(&self, common: OrchestrationCommon) -> anyhow::Result<()> {
//         // loop through all the testbed hosts
//         for (logical_host, testbed_host_data) in common.testbed_hosts.iter() {
//             let testbed_host = logical_host;
//             let testbed_host_nic = &testbed_host_data.testbed_nic;
//
//
//             // count geneve interfaces per host to make sure all interfaces on the ovs tunnel with the geneve tunnel are unique
//             let mut geneve_interface_counter = 0;
//             for (_, connection_info) in self.0.as_ref().unwrap().iter() {
//                 match connection_info {
//                     BridgeConnection::Ovs {
//                         name: _,
//                         source_br,
//                         target_br,
//                         source_veth,
//                         target_veth,
//                         ip: _,
//                         testbed_host,
//                     } => {
//                         // match the bridge connection to the current testbed host variable in the loop
//                         if testbed_host.eq(testbed_host) {
//                             // writeln!(ovs_create_file, "{}", format!("# pair = {name}").as_str())?;
//                             // connect the bridges to the veths
//
//                             let cmd = vec!["ip", "link", "delete", &source_veth];
//                             run_testbed_orchestration_command_allow_fail(
//                                 &common,
//                                 testbed_host,
//                                 "sudo",
//                                 cmd,
//                                 false).await?;
//                             // bring up bridges
//
//                             let cmd = vec!["ovs-vsctl", "del-port", &source_br, &source_veth];
//                             run_testbed_orchestration_command_allow_fail(
//                                 &common,
//                                 testbed_host,
//                                 "sudo",
//                                 cmd,
//                                 false).await?;
//
//                             let cmd = vec!["ovs-vsctl", "del-port", &target_br, &target_veth];
//                             run_testbed_orchestration_command_allow_fail(
//                                 &common,
//                                 testbed_host,
//                                 "sudo",
//                                 cmd,
//                                 false).await?;
//
//                         }
//                     }
//                     BridgeConnection::Tunnel {
//                         source_br,
//                         target_br,
//                         source_remote_ip,
//                         target_remote_ip,
//                         key: _,
//                         source_br_ip: _,
//                         target_br_ip: _,
//                         testbed_host_source,
//                         testbed_host_target,
//                     } => {
//                         // configure "left" of tunnel
//                         if testbed_host_source.eq(testbed_host) {
//                             let src_nic = &testbed_host_nic;
//                             // target remote ip here is the local (to the host in iteration) unique tunnel ip
//                             let local_tunnel_ip = target_remote_ip.as_ref().unwrap();
//
//                             let local_tunnel_ip_wmask = format!("{local_tunnel_ip}/24");
//                             let cmd = vec!["ip", "addr", "del", &local_tunnel_ip_wmask, "dev", &src_nic];
//                             run_testbed_orchestration_command_allow_fail(
//                                 &common,
//                                 testbed_host,
//                                 "sudo",
//                                 cmd,
//                                 false).await?;
//
//                             let geneve_geneve_interface_counter = format!("geneve{geneve_interface_counter}");
//                             let cmd = vec!["ovs-vsctl", "del-port", &source_br, &geneve_geneve_interface_counter];
//                             run_testbed_orchestration_command_allow_fail(
//                                 &common,
//                                 testbed_host,
//                                 "sudo",
//                                 cmd,
//                                 false).await?;
//
//
//                             geneve_interface_counter += 1;
//                         }
//                         // configure "right" of tunnel
//                         if testbed_host_target.eq(testbed_host) {
//                             let tgt_nic = &testbed_host_nic;
//                             // source remote ip here is the local (to the host in iteration) unique tunnel ip
//                             let local_tunnel_ip = source_remote_ip.as_ref().unwrap();
//
//                             let local_tunnel_ip_wmask = format!("{local_tunnel_ip}/24");
//                             let cmd = vec!["ip", "addr", "del", &local_tunnel_ip_wmask, "dev", &tgt_nic];
//                             run_testbed_orchestration_command_allow_fail(
//                                 &common,
//                                 testbed_host,
//                                 "sudo",
//                                 cmd,
//                                 false).await?;
//
//                             let geneve_geneve_interface_counter = format!("geneve{geneve_interface_counter}");
//                             let cmd = vec!["ovs-vsctl", "del-port", &target_br, &geneve_geneve_interface_counter];
//                             run_testbed_orchestration_command_allow_fail(
//                                 &common,
//                                 testbed_host,
//                                 "sudo",
//                                 cmd,
//                                 false).await?;
//
//
//                             geneve_interface_counter += 1;
//                         }
//                     }
//                 }
//             }
//         }
//         Ok(())
//     }
// }
