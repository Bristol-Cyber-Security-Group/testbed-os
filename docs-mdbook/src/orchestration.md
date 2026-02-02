# TestbedOS Orchestration

The TestbedOS orchestration is a process to break down and execute the tasks required for the successful execution of a deployment command as entered by a user for a TestbedOS project. The orchestration is executed by the [the TestbedOS server](server.md) and it depends on the deployment command as entered by the user to bring up, interact with, or shut down the deployment. For example, the orchestration will download the required libvirt virtual machine images for the deployment command `kvm-compose generate-artefacts`.

## Orchestration Overview

The orchestration will occur between the TestbedOS client and [the TestbedOS server](server.md).
The TestbedOS client will receive the deployment commands as entered by the users, e.g., `kvm-compose up`, and the TestbedOS server will execute the necessary *orchestration instructions* for the deployment command. 
Please see [the orchestration protocol](#orchestration-protocol) for more information on the client-server protocol orchestration.

We batch the orchestration instructions against the different services used by TestbedOS, grouped per stage as each stage is dependent on the previous stage. 
So all resources in a stage, such as turning on guests, will be batched together. 
More details on this in the [Deployment Stages](#deployment-stages) section.

All functionalities based on [the `kvm-compose.yaml` file](schema.md) are generally based on the orchestration pipeline.
This means they all require the state file to have been created (via generate-artefacts), and also usually for the guests to be running depending on the command (some commands like snapshot will turn them off). We discuss on [the state configuration file in the next section](#state-configuration-file).


## State Configuration File

The TestbedOS orchestration works off of a TestbedOS project's state configuration file `<project name>-state.json`.
The state configuration file is generated from [the project's `kvm-compose.yaml` file](schema.md) after the initial `kvm-compose generate-artefacts` command, and it includes the configurations declared in the [`kvm-compose.yaml`](schema.md) file. 
In fact, the state configuration file is a direct serialisation of the desired state from the [`kvm-compose.yaml`](schema.md) file in memory.
Please refer to the [`Minimal Working Example`](welcome.md#minimal-working-example) to see the command in action.


## Deployment Stages

There are several stages defined for each deployment stage in the orchestration process for bringing up the deployment for a TestbedOS project.
This specifically happens after the user executes the `kvm-compose up` command.
Please see the [Minimal Working Example](welcome.md#minimal-working-example) on the usage of `kvm-compose up` in a deployment.
Each stage, where possible, executes the deployment tasks on all target host or guests in parallel.
These stages occur in sequence, as follows:

1) Ensure that the state configuration file and `artefacts` folder exists and deserialise the state configuration file.
2) Check if all TestbedOS hosts to be used in the deployment are running.
3) Create project folders on client TestbedOS hosts (if being used).
4) Network Stage
    1) Deploy the [libvirt network configuration](networking_guest_ovn.md.md) on the main TestbedOS host.
    2) Deploy the [OVS bridges](networking_guest_ovn.md) across all TestbedOS hosts.
    3) Deploy the [OVS tunnels](networking_guest_ovn.md) across all TestbedOS hosts.
5) If clones are defined in [the `kvm-compose.yaml` file](schema.md) via the `scaling` parameter,
    1) Provision the golden image if a shared install script is supplied in [the `kvm-compose.yaml` file](schema.md) via the `shared-setup` parameter.
    2) Create the .qcow2 linked clones from the golden image.
6) Guest Deployment Stage
    1) Distribute artefacts for all TestbedOS guests to the respective TestbedOS hosts.
    2) Rebase any linked clone images that are on remote TestbedOS hosts
    3) Switch on all guests.
7) Guest Setup Stage
    1) Execute the setup scripts for all testbed guests if specified in the [the `kvm-compose.yaml` file](schema.md) via the `setup_script` parameter. 

<!-- ## Artefacts

The artefacts generated are placed in the `artefacts` folder inside the project folder, that contains the |kvm-compose.yaml| file.

Example for three testbed hosts, each allocated with one machine definition.

.. code-block:: sh

    kvm-compose.yaml
    state.json # see State JSON section
    artefacts/machine1-domain.xml
    artefacts/machine1-cloud-init.img
    artefacts/machine1-cloud-init.iso
    artefacts/machine2-domain.xml
    artefacts/machine2-cloud-init.img
    artefacts/machine2-cloud-init.iso
    artefacts/machine3-domain.xml
    artefacts/machine3-cloud-init.img
    artefacts/machine3-cloud-init.iso -->

## Orchestration Protocol

After a user enters a TestbedOS deployment command, e.g., `kvm-compose up`, the TestbedOS clients send the deployment commands which is then converted into a series of *orchestration instructions*. In the source code, this is essentially a vector of strings defined by us and it is represented by the [`OrchestrationInstruction enum` in the source code](https://github.com/Bristol-Cyber-Security-Group/testbed-os/blob/develop/kvm-compose/kvm-compose/src/orchestration/api.rs#L77).

Each orchestration instruction is then serialised (by Rust's `serde` crate) and sent to the TestbedOS server over a websocket and they are processed by the server with the server executing the corresponding shell commands as required for the successful execution of the deployment command. The client will send each orchestration instruction in the series for a deployment command one by one to the server until completion. 

The orchestration protocol between the client and server is as follows:

1) The TestbedOS client receives the deployment command from the user and converts them into a series of orchestration instructions. For example, every series of orchestration instructions start with `Init` and ends with `End`.
2) The client sends each orchestration instruction in the series to the TestbedOS server until completion. 
3) The server then sends acknowledgement that the orchestration instruction is valid and execute the shell commands for the corresponding orchestration instruction, except for the `End` instruction that signals the end of the orchestration instruction.
4) The server sends the result of the instruction execution, success or fail with the relevant message.

This protocol is followed for all instructions, minus the `End` instruction.

There might be a need to also send further logging to the client.
For example, the [analysis tooling](user_interface_exec_tool.md) will have some output that is to be displayed to the user.
This logging would be outside of the protocol outlined above.
To manage this, during the run of each orchestration instruction, the server also has a logging specific channel that is sent to the instruction processing code.
If any logging events are emitted from the instruction, the server will emit this log to the client through the websocket, and it will be handled concurrently to the protocol.


## Interaction with [TestbedOS User Interfaces](user_interface.md)

For the [CLI](user_interface.md#cli), the orchestration instructions are generated locally because the orchestration logic and the CLI source code shared the same library written in Rust that runs directly on the TestbedOS host's filesystem, which both of them have access to.

In contrast, as the [GUI](user_interface.md#gui) is implemented in JavaScript and runs in a browser, it operates in a different execution environment and does not have access to the orchestration source code, which would require the orchestration source code to be rewritten in Javascript. As a result, we solve this by having the GUI to first request the TestbedOS server to read the [state configuration file](#state-configuration-file) and generate the orchestration instructions on its behalf. Once generated, the server sends the orchestration instructions list back to the GUI. Then, the GUI will continue with the protocol in the same way as the CLI, which forwards the resulting commands to the server for execution.

<!-- The orchestration instruction generation has been designed to decouple the sending of the instructions generated with the sending of the instruction to the server.
So that we can re-use this code for both the CLI and the GUI, we utilise channels to message from the functions that generate the command back to a listener.
When called by the CLI/TUI, the instruction is immediately relayed to the server to be executed.
For the GUI, when the server is running the generation code, it relays it to the GUI so that it can store them all before stating the orchestration. -->

Once the CLI or GUI have the orchestration instructions ready, they will send them to the server for execution over a websocket.
For this to work, the server needs to concurrently handle the orchestration instructions over the websocket connection from the client, and the execution of the commands send over this websocket.
Due to the usage of Rust's `async` feature for the orchestration source code so rather than running the orchestration instructions in parallel, they are executed concurrently.
While the orchestration websocket is open on the server, there is:

1) The send and receive websocket connection to and from the client for [the orchestration protocol](#orchestration-protocol), and
2) The cancellation listener
    - For the GUI this comes over the websocket
    - For the CLI this is another `async` task listening for interrupt signal by the user, i.e., the `Ctrl+C` keypress.

### Cancellation

The orchestration supports the cancellation of commands.
This means the users are able to cancel or interrupt a command while it is running on the server and have a graceful exit.
For the CLI, there is a handler for the interrupt keypress by the user, i.e., `Ctrl+C`, which when triggered will cancel the command running and tell the TestbedOS server to stop.
For the GUI, there is a cancellation button on the command running page, which will do the same thing.

<!-- ## Architecture from `kvm-compose`

Note: the testbed has been designed in this way to expose the artefacts generated before running the testbed orchestration to allow the user to inspect the artefacts.
This also allows you to further customise the deployment if you wish or if the testbed tooling does not yet support a specific feature you are looking for.
Additionally, since these are mostly text files, it allows you to version control your test case (don't forget to ignore the large binary files in your .gitignore if you use git).
As long as you keep to the convention of the artefacts generated, the orchestration tool will just push and execute these scripts/config/images to the correct locations.
The orchestration tool "doesn't care" what is inside these, as long as the generated |state JSON| file is valid. -->


<!-- ## Orchestration Tasks

The core functionality of this tool is to take a mapping from a hostname, which can either be a testbed guest or a testbed host and run a script/command or push a file to them.
This enables the deployment across the distributed testbed using SSH.
In this document, we will describe the execution of a command or script or pushing a file as a "task".

There are four different scenarios for command running of tasks:

:local testbed host: This is running a task on the current (main) testbed host.
    Note: there is no need for SSH as it will be a local command.
    Note: we do not need to push files since the artefacts will already be in this host's file system.

:local testbed guest: This is running a task on a testbed guest that is on the current (main) testbed host.
    This uses SSH.

:remote testbed host: This is running a task on a remote testbed host.
    This uses SSH.

:remote testbed guest: This is running a task on a testbed guest that is on a remote testbed host.
    This uses a proxied SSH, where we SSH onto the testbed host first then SSH onto the testbed guest. -->

## Scaling

TestbedOS provides a scaling capability for storage optimisation of libvirt guest machines by using QCOW2 linked clones through the `scaling` parameter in [the kvm-compose.yaml file schema](schema.md). 

During the orchestration stage, the libvirt guest clones is provisioned similarly to other non-clone guests and the cloned guests are treated like any other guest in TestbedOS, it is only their provisioning steps  (creating a clone from the golden image) that is different.
That is, there is an extra stage executed, if clones are present in the [state configuration file](#state-configuration-file) to first provision the golden image (backing image) and then to create linked clones from it.

Note that the golden image is also a guest but it will be turned off for the duration of the deployment as its disk must not have a write lock by the operating system, such that the clone guests may copy-on-write as they require. Expanding from Stage 5 in [deployment stages](#deployment-stages), the process to create the linked clones are as follows.

1) Start the golden image.
2) Wait for the golden image to be available.
3) Execute share install script as defined by the [`shared_setup` parameter](libvirt.md#scaling) in [the kvm-compose.yaml file](schema.md). 
4) Turn off the golden image guest.
5) Wait for the golden image guest to be shut down for the write lock to be removed by the operating system.
6) Create the number of clones as specified by [the `count` parameter](libvirt.md#scaling) in [the `kvm-compose.yaml` file](schema.md).
7) The linked clones are started in the same way as non-linked clones.

The timeout for waiting to connect to a guest is 2 minutes. This has been chosen arbitrarily with no consideration for a scaled setup where many guests are requested causing a big load on the CPU and could naturally push connection time to over 2 minutes.

The golden image must be present on any TestbedOS host that has a clone, if the clones are distributed over multiple TestbedOS hosts.
Therefore a copy of the golden image is pushed to any TestbedOS host that has a linked clone that needs it.
Note that the clone guests are treated as an ['existing disk' guest type](libvirt.md#existing-disk) internally by TestbedOS.


## Dynamic Deployment Changes

TestbedOS currently does not yet factor in if you have made changes to the [`kvm-compose.yaml` file](schema.md), after the orchestration of a deployment.
This means you will encounter state drift if running `kvm-compose up`, then changing the `kvm-compose.yaml` file and then running `kvm-compose up` again.
To be sure there is no state drift, make sure to first run `kvm-compose down` before making any changes to the `kvm-compose.yaml` file.
Once that is done, since the deployment is now an existing deployment and hence [a state configuration file](#state-configuration-file) exists for the deployment, you will need to run `kvm-compose up` with the `--provision` flag for the deployment to reflect the changes.
We look to improve this state drift use case in the future.

## Load Balancing

Given an arbitrary network topology and machine definitions in [the `kvm-compose.yaml` file](schema.md) for a deployment, their orchestration will be distributed over the TestbedOS hosts listed in [the `kvm-compose-config.json` file](configurations.md#main-host-server-configurations) in [clustering mode](clustering_mode.md).
The following are the current possible load balancing algorithms with heuristics that can be used with the clustering mode:

Round robin
: The topology is distributed based on the bridges defined across the TestbedOS hosts in a round robin allocation.
    Starting with the first TestbedOS host in [the `kvm-compose-config.json` file](configurations.md#main-host-server-configurations), each bridge is allocated until all bridges allocated.
    The machines that have that bridge as an interface will then also be allocated to that TestbedOS host.
    This is a simple implementation with no consideration for resource usage and minimising potential number of tunnels between the TestbedOS hosts.
    Note: if a machine has multiple bridges as interfaces and the bridges are on different hosts, it will not work as there is no check for this.

## Technical Detail and Developer Notes

The TestbedOS orchestration outlines the process taking place from [the `kvm-compose.yaml` file](schema.md) to the resulting [state configuration file](#state-configuration-file) and artefacts of the deployment as well as the process taking place when the user interacts with the deployment via [the `kvm-compose` commands](user_interface.md).

In the orchestration process, [the `kvm-compose.yaml` file](schema.md) is first deserialised and a `Config` struct is filled.
With this struct, a logical deployment ([`LogicalTestbed`](https://github.com/Bristol-Cyber-Security-Group/testbed-os/blob/develop/kvm-compose/kvm-compose/src/components/mod.rs#L149)) is constructed, which is a class encompassing the different underlying components of a deployment, with their base class modelled by [`TestbedComponent`](https://github.com/Bristol-Cyber-Security-Group/testbed-os/blob/develop/kvm-compose/kvm-compose/src/components/mod.rs#L66).

These components can be of various types, e.g., a component to model a machine guest in the deployment, i.e., [`TestbedGuestComponent`](https://github.com/Bristol-Cyber-Security-Group/testbed-os/blob/develop/kvm-compose/kvm-compose/src/components/mod.rs#L134), or a component to model a networking component in the deployment, i.e., [`TestbedNetworkInterfaceComponent`](https://github.com/Bristol-Cyber-Security-Group/testbed-os/blob/develop/kvm-compose/kvm-compose/src/components/mod.rs#L139).
The logical deployment is load balanced between the available TestbedOS hosts, based on [the load balancing algorithm](#load-balancing).
Then each `TestbedComponent` will undergo a process called specialisation through the `specialise` method for the trait, so that these components will be populated with parameters that are specific to them such as paths that are specific to the TestbedOS host they are assigned to.
A `State` object is in turn created for the deployment, which becomes [the state configuration file](#state-configuration-file) to be used by the orchestrator.

As a developer, you may want to add new components, possibly:

- Testbed Host
- Testbed Guest
- Testbed Bridge
- Testbed Network

These components must implement any of the `TestbedComponent` traits.
You will only need to implement the trait and the rest of the TestbedOS code will treat it like any other component, so you do not need to add extra code into the TestbedOS core codebase.
You will need to also create a new entry for `Config` so that the new component is part of the `kvm-compose.yaml` schema.
Our abstraction allows you to focus only in how a `TestbedComponent` is converted from a `Config` and into artefacts.
