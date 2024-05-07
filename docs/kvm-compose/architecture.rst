============
Architecture
============

`kvm-compose` is a binary CLI tool written in rust.
It uses the serde library to deserialise and parse the |kvm-compose.yaml| file to start building a state representation in memory.
The state representation, once enumerated with details from the |kvm-compose.yaml| and |kvm-compose-config.json|, will be also serialised with serde into the state JSON file to be used with |orchestration|.
Along with the state JSON, the artefacts are also generated.
Note: this CLI tool uses the APIs on the |testbed server| to execute the testbed processes.

Note: the testbed has been designed in this way to expose the artefacts generated before running the testbed orchestration to allow the user to inspect the artefacts.
This also allows you to further customise the deployment if you wish or if the testbed tooling does not yet support a specific feature you are looking for.
Additionally, since these are mostly text files, it allows you to version control your test case (don't forget to ignore the large binary files in your .gitignore if you use git).
As long as you keep to the convention of the artefacts generated, the orchestration tool will just push and execute these scripts/config/images to the correct locations.
The orchestration tool "doesn't care" what is inside these, as long as the generated |state JSON| file is valid.

`kvm-compose` needs access to the libvirt daemon to access information about existing libvirt networks.

See the |networking| section for more information on the architecture for generating the distributed bridges and tunnels configuration.

Load Balancing
--------------

Given an arbitrary network topology and machine definitions in the |kvm-compose.yaml| file, these will be distributed over the testbed hosts listed in the |kvm-compose-config.json| file.

The following are the current possible load balancing algorithms with heuristics that can be used with the testbed:

:round robin: The topology is distributed based on the bridges defined across the testbed hosts in a round robin allocation.
    Starting on the first host in the |kvm-compose-config.json| file, each bridge is allocated until all bridges allocated.
    The machines that have that bridge as an interface will then also be allocated to that testbed host.
    This is a simple implementation with no consideration for resource usage and minimising potential number of tunnels between testbed hosts.
    Note: if a machine has multiple bridges as interfaces and the bridges are on different hosts, it will not work as there is no check for this.

Artefacts
---------

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
    artefacts/machine3-cloud-init.iso


State JSON
----------

The state JSON file is a direct serialisation of the desired state from the |kvm-compose.yaml| file in memory.
This data structure is used to enumerate all the artefacts, so is computed before writing any artefacts to disk.

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


Log Streaming
-------------

The kvm-compose CLI tool will use the various APIs on the testbed server to run commands and control deployments.
Since these commands can be long running, the CLI also uses a websocket to stream the logs of the command from the server.
Currently the websocket is solely one directional, from the server to the client.

See the |testbed server| section for details on the architecture of the log streaming.

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


.. |kvm-compose.yaml| replace:: :ref:`kvm-compose-yaml/index:kvm-compose Yaml`
.. |kvm-compose-config.json| replace:: :ref:`testbed-config/index:Testbed Config`
.. |orchestration| replace:: :ref:`orchestration/index:orchestration`
.. |networking| replace:: :ref:`networking/index:Networking`
.. |state JSON| replace:: :ref:`state JSON <kvm-compose/architecture:State JSON>`
.. |testbed server| replace:: :ref:`testbed server <testbedos-server/index:TestbedOS Server>`
