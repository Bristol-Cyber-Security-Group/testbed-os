---
title: SERVER
section: 1
---

# TestbedOS Server

The TestbedOS Server is a server that runs in the background on the [`Main` TestbedOS host](configurations.md#singleton-mode-or-the-main-host) as a daemon. 

The objective of the TestbedOS server is to keep track of the state of the deployments. This means tracking any existing deployments if you have multiple and check to see the `up` or `down` state after the corresponding `kvm-compose up` and `kvm-compose down` commands respectively. This also includes the `failed` state if there is error in interacting with the deployments via [the other `kvm-compose` commands](user_interface_kvm_compose.md).

It offers a [REST API](server_api.md) to control the deployments. This API is used by the [`kvm-compose` CLI](user_interface.md#cli) and for the [web GUI](user_interface.md#gui). The server also allows inspecting and modifying the configuration of the [configurations](configurations.md) through the API.

<!-- The server is solely a wrapper around the kvm-compose library, which can be invoked without the server - see the kvm-compose usage document. Note that when not using the server you lose the ability to deal with deployments and just work from the current project folder. -->

## Starting the TestbedOS Server for a Deployment

There are three ways to start the TestbedOS server to start or interact with a deployment:
- You can either use the server in daemon mode by running `sudo systemctl start testbedos-server.service`,
- You can also directly run the server from the CLI with `sudo /usr/local/bin/testbedos-server`, or
- You can run via `cargo`. If you navigate to the `/testbed-os/kvm-compose/testbedos-server` project folder of [the TestbedOS source code](https://github.com/Bristol-Cyber-Security-Group/testbed-os), then the server can be run with `sudo -E bash -c  'cargo run -- main' $USER`.

Once you have successfully run the server once in [`Main`] mode, you do not need to specify `Main` unless you edit the `mode.json`. Please refer to the [TestbedOS Configurations](configurations.md) for more information on this.

After the server is running, we can now start a TestbedOS deployment. Please refer to the different minimal working examples (MWEs): [Minimal Work Example](welcome.md#minimal-working-example), [AVD Minimal Working Example](avd.md#minimal-working-example), and [Docker Minimal Working Example](docker.md#minimal-working-example) to get started, or if you are already familiar with the MWEs please feel free to go ahead and deploy your own TestbedOS project.

