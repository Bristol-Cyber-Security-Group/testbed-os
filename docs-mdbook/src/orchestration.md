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
    - for the CLI this is another `async` task listening for interrupt signal by the user, i.e., the `Ctrl+C` keypress.

Cancellation
------------

The orchestration and command running supports cancellation of commands.
This means you are able to cancel or interrupt a command, while it is running on the server and have a graceful exist.
For the CLI and TUI, there is a `ctrl+c` handler, which when triggered will cancel the command running and tell the server to stop.
For the GUI, there is a cancel button on the command running page, which will do the same thing.

## Architecture from `kvm-compose`

Note: the testbed has been designed in this way to expose the artefacts generated before running the testbed orchestration to allow the user to inspect the artefacts.
This also allows you to further customise the deployment if you wish or if the testbed tooling does not yet support a specific feature you are looking for.
Additionally, since these are mostly text files, it allows you to version control your test case (don't forget to ignore the large binary files in your .gitignore if you use git).
As long as you keep to the convention of the artefacts generated, the orchestration tool will just push and execute these scripts/config/images to the correct locations.
The orchestration tool "doesn't care" what is inside these, as long as the generated |state JSON| file is valid.

`kvm-compose` needs access to the libvirt daemon to access information about existing libvirt networks.

Command Running
---------------
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
    This uses a proxied SSH, where we SSH onto the testbed host first then SSH onto the testbed guest.

Scaling
-------

To speed up the provisioning of guests that share a common install, the scaling features utilises the linked clone functionality offered by .qcow2 image file tipe.
Qcow2 stands for qemu copy on write, which means the clone disks will only contain the difference from the original image we call 'golden image'.
This offers disk usage optimisation, for example without linked clones if we have 3 guests that share the same install and take up 10GB of space each then we use a total of 30GB of space.
With clones (3 to match the example), the golden image would take 10GB of space and the 3 guests would start with a few kilobytes in disk space used and only grow as the guest creates/edits files.
Furthermore, if the common install was bandwidth or CPU intensive, using clones we only need to do this once rather than 3 times concurrently which is likely to compete for resources and take more time.

We implement this scaling feature by offering the `scaling` option in the kvm-compose.yaml file schema, see the schema doc for the limitation of the syntax.
In the artefact generation stage, the clone guests have artefacts prepped ready for when the clone .qcow2 images are prepared in the orchestration stage.
The cloned guests are treated like any other guest in the testbed, it is only their provisioning steps  (creating a clone from the golden image) that is different.
Note that the golden image is also a guest but it will be turned off for the duration of the testbed test case as it's disk must not have a write lock, such that the guests may copy on write as they need.

Scaling
-------

Scaling is a feature in the testbed backed by the qemu linked clones functionality.
The backing image for the clones will become .qcow2 images (qemu copy on write) so that they are efficient with space.
Therefore the state in the backing image will be available to clones.
Some state will be overwritten by cloud-init (if using cloud-init) such as the hostname and any other post install scripts or any other cloud-init functionality.

Given the use of the scaling parameter (see kvm-compose.yaml schema and kvm-compose scaling architecture for more info) in the kvm-compose.yaml file, the orchestration of clones is similar to other non clone guests.
There is an extra stage executed, if clones are present in the state.json to provision the golden image (backing image) and then create linked clones from it.
The process to create the linked clones:

1) start the golden image
2) wait for it to be available
3) execute share install script
4) turn off guest
5) wait for guest to be shut down for write lock to be removed
6) create n number of clones as specified in the kvm-compose.yaml file

Snapshots
---------

The testbed also supports snapshots of libvirt guests.
It supports multiple testbed hosts.
You can create/restore/delete/list snapshots through the `kvm-compose` CLI.

The snapshots are stored on the respective testbed hosts the guests are created on.
The CLI is merely a wrapper around the libvirt snapshot API, so if you create a snapshot outside of the testbed tools the snapshot will be available to the testbed.

Existing Disk
^^^^^^^^^^^^^

When you bring a pre-configured image to the testbed, we will not overwrite the original image to preserve it.
Instead, by default the testbed will create a linked clone of this image in the project artefacts folder.
This removed the need to create a deep copy of the image, saving time and space on disk.
The user can still defer to a deep copy with the `create_deep_copy` option in the existing disk yaml section.

When the existing disk linked clone is going to be placed on a remote testbed host, the testbed will need to send a full copy.
This is because we cannot use linked clones over the network, and because we don't have a distributed filesystem at the moment to support this.


General Notes
^^^^^^^^^^^^^

Once the guest deploy stage is reached, the linked clones are started in the same way as non linked clones.
Note that the golden image must be present on any testbed host that has a clone, if the clones are distributed over multiple testbed hosts.
Therefore the golden image is pushed (a copy) to any testbed host that has a linked clone that needs it.
Note that the clone guests are treated as an 'existing disk' guest type internally.

The timeout for waiting to connect to a guest is 2 minutes, this has been chosen arbitrarily with no consideration for a scaled setup where many guests are requested causing a big load on the CPU and could naturally push connection time to over 2 minutes.

Delta Change
^^^^^^^^^^^^

The testbed currently does not yet factor in if you have made changes to the |kvm-compose.yaml| file, after deploying.
This means you will encounter state drift if running `up`, then changing the yaml and then running `up` again.
To be sure there is no state drift, make sure to run `down` first.
Note that since you have already deployed something and a state file exists, you will need to run up with the `--provision` flag.

We look to improve this state drift use case in the future.

Load Balancing
--------------

Given an arbitrary network topology and machine definitions in the |kvm-compose.yaml| file, these will be distributed over the testbed hosts listed in the |kvm-compose-config.json| file.

The following are the current possible load balancing algorithms with heuristics that can be used with the testbed:

:round robin: The topology is distributed based on the bridges defined across the testbed hosts in a round robin allocation.
    Starting on the first host in the |kvm-compose-config.json| file, each bridge is allocated until all bridges allocated.
    The machines that have that bridge as an interface will then also be allocated to that testbed host.
    This is a simple implementation with no consideration for resource usage and minimising potential number of tunnels between testbed hosts.
    Note: if a machine has multiple bridges as interfaces and the bridges are on different hosts, it will not work as there is no check for this.

## Limitations

Be aware that if you do use sudo, the files created may required elevated permissions to use so you will there-on need to continue to use sudo unless you manually edit the owner (`chown`) or permissions (`chmod`).

If you use kvm-compose up with or without sudo, if you are using cloud-init images, then be aware that the images downloaded will either go to ``/root/.kvm-compose/`` if you use sudo or ``/home/<your home folder/.kvm-compose/`` if you do not.
This means that you may end up downloading the images twice, once in each folder if you interchange the use of sudo.

Technical Detail and Developer Notes
------------------------------------

This text outlines the process to go from the kvm-compose.yaml file to the resulting state json file and artefacts.
The yaml file is deserialised and a `Config` struct is filled.
With this struct, a logical testbed is started to be constructed, which works with testbed `components`.
These `components` can be of various types, i.e. a `guest component` could be a Libvirt guest or Libvirt clone.
The logical testbed is logically load balanced between the available testbed hosts, based on the load balancing algorithm.
Then specialisation occurs on the testbed `components`, so that these components will have data generated such as paths that are specific to the testbed host they are assigned to.
The `State` is created, which becomes the state json file to be used by the orchestrator.

As a developer, you may want to add new components.
The possible components:

- Testbed Host
- Testbed Guest
- Testbed Bridge
- Testbed Network

These components are traits, meaning your component must implement the trait.
You will only need to implement the trait and the rest of the code will treat it like any other component, so you don't need to add extra code in the core codebase.
You will need to also create a new entry for `Config` so that the new component is part of the yaml schema.
This abstraction allows you to focus only in how a testbed `Component` is converted from a `Config` and into artefacts.
