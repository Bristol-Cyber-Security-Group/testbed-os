============
Architecture
============

The orchestration in testbed works off the |state JSON|, where the elements in this state will have a corresponding way to be provisioned in the testbed.
The state is created through the process of creating a logical testbed from the |kvm-compose.yaml|.

Background
----------

Orchestration is run on the server, the clients (the CLI, TUI and GUI) will send commands that will be processed by the server.
The orchestration code uses the rust `async` syntax but rather than running the orchestration tasks in parallel, they are executed concurrently.
The slowest parts of the orchestration are io based, so networking and the filesystem - everything else is extremely quick.
We batch the commands against the different services used by the testbed, grouped per stage as each stage is dependant on the previous stage.
So all resources in a stage, such as turning on guests, will be batched together.

All commands and functionality based on the yaml file are generally based on the orchestration pipeline.
This means they all require the state file to have been created (via generate-artefacts), and also usually for the guests to be running depending on the command (some commands like snapshot will turn them off).
Every command is built together as a series of instructions, as mentioned before, each instruction has an order and can batch on multiple resources.
For example, all commands start with the `Init` instruction as an initial check of the system, then followed by the series of instruction(s) to complete the command, completed with the `End` instruction.

GUI Technical Detail
--------------------

The GUI works slightly differently to the CLI and TUI, where the command generation occurs on the server.
Since the GUI does not have filesystem access, the GUI will make one extra call before command running to ask the server to read the state file and generate the commands on it's behalf.
Once generated, the server will send the command list in order to the GUI.
Then, the GUI will continue with the command running in the same way as the CLI and TUI.

Command Running
---------------
The core functionality of this tool is to take a mapping from a hostname, which can either be a testbed guest or a testbed host and run a script/command or push a file to them.
This enables the deployment across the distributed testbed using SSH.
In this document, we will describe the execution of a command or script or pushing a file as a "task".

There are four different scenarios for command running of tasks:

:local testbed host: This is running a task on the current (master) testbed host.
    Note: there is no need for SSH as it will be a local command.
    Note: we do not need to push files since the artefacts will already be in this host's file system.

:local testbed guest: This is running a task on a testbed guest that is on the current (master) testbed host.
    This uses SSH.

:remote testbed host: This is running a task on a remote testbed host.
    This uses SSH.

:remote testbed guest: This is running a task on a testbed guest that is on a remote testbed host.
    This uses a proxied SSH, where we SSH onto the testbed host first then SSH onto the testbed guest.

Stages
------
There are several stages defined for each deployment stage of the testbed.
Each stage, where possible will execute tasks on all target host/guest in parallel.
These stages do have a dependency, as follows:

1) Check for Artefacts Stage (ensures the state JSON and artefacts folder exist and deserialise the state JSON)
2) Check if all testbeds to be used in the deployment are running
3) Create project folders on client testbed hosts (if being used)
4) Network Stage
    1) deploy the libvirt network on the master testbed host
    2) deploy the openvswitch bridges across all testbed hosts
    3) deploy the openvswitch tunnels across all testbed hosts
5) If clones are defined
    1) provision the golden image if shared install script supplied
    2) create .qcow2 linked clones from golden image
6) Guest Deploy Stage
    1) distribute artefacts for all testbed guest to the respective testbed hosts
    2) rebase any linked clone images that are on remote testbed hosts
    3) start all guests
7) Guest Setup Stage
    1) execute (if specified in the |kvm-compose.yaml| file) the setup scripts for all testbed guests

Cancellation
------------

The orchestration and command running supports cancellation of commands.
This means you are able to cancel or interrupt a command, while it is running on the server and have a graceful exist.
For the CLI and TUI, there is a `ctrl+c` handler, which when triggered will cancel the command running and tell the server to stop.
For the GUI, there is a cancel button on the command running page, which will do the same thing.

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

Technical Architecture
======================

The orchestration and command running are executed over a websocket from the client.
The client will send a series of instructions that make up the orchestration command one by one until completion.
For the CLI and TUI, the command generation happens locally as they have filesystem access.
For the GUI, there is an extra step where the GUI will ask the server to generate the commands and send them to the GUI, before the GUI can then send the commands.
This means the GUI, once it has the series of commands, will then open the websocket to the server's orchestration endpoint to run commands like the CLI and TUI.

Command generation has been designed to decouple the sending of the instructions generated to the sending of the instruction to the server.
So that we can re-use this code for both the CLI/TUI and the GUI, we utilise channels to message from the functions that generate the command back to a listener.
When called by the CLI/TUI, the instruction is immediately relayed to the server to be executed.
For the GUI, when the server is running the generation code, it relays it to the GUI so that it can store them all before stating the orchestration.

Once the CLI/TUI or GUI have the instructions ready, they will send them to the server for processing over a websocket.
For this to work, the server needs to concurrently handle the orchestration websocket connection from the client, and the execution of the commands send over this websocket.
While the orchestration websocket is open on the server, there is:

1) the send and receive websocket connection to the client
2) cancellation listener
    - for the GUI this comes over the websocket
    - for the CLI/TUI this is another async task listening for `ctrl+c`

The protocol between the client and server is as follows:

1) client sends instruction
2) server sends acknowledgement that instruction is valid
3) server sends result of the instruction execution, success or fail with some relevant message string

This protocol is followed for all instructions, minus the `End` instruction.

During orchestration and command running, there might be the need to also send further logging to the client.
For example, the analysis tooling will have some output that is to be displayed to the user.
This logging would be outside of the protocol outlined above.
To manage this, during the run of each instruction, the server also has a logging specific channel that is sent to the instruction processing code.
If any logging events are emitted from the instruction, the serer will emit this log to the client through the websocket, and it will be handled concurrently to the protocol.





.. |state JSON| replace:: :ref:`state JSON <kvm-compose/architecture:State JSON>`
.. |artefacts| replace:: :ref:`artefacts <kvm-compose/architecture:artefacts>`
.. |kvm-compose.yaml| replace:: :ref:`kvm-compose/kvm-compose-yaml/index:kvm-compose Yaml`
