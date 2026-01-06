Multiple Interfaces on Guests
=============================

In the kvm-compose.yaml file you can specify one or more interfaces for guests.
Currently, only libvirt guests support multiple interfaces.

The libvirt guests will have their domain.xml generated with the list of interfaces defined in the yaml file.
For libvirt cloud-init guests, this interface information is placed in the cloud-init network config and will boot with the interfaces configured automatically.

Guest to OVN connection
=======================

Virtual machines or any software with networking capabilities can be connected to the testbed as a guest.
As long as this guest has a port on the OVS integration bridge.
For the current supported guest types, there are a few different implementation details in how we achieve this.

Libvirt
-------

In the network definition in the libvirt `domain.xml` such as below, there is the unique name of the interface for this virtual machine.
This interface is subsequently bridged to the host's integration bridge to create a port on the bridge.
The name of this port is the name used in the logical switch port.

.. code-block:: xml

    <interface type='ethernet'>
        <mac address='00:00:00:00:00:03'/>
        <target dev='guest-interface'/>
        <model type='virtio'/>
        <mtu size='1442'/>
        <address type='pci' domain='0x0000' bus='0x00' slot='0x03' function='0x0'/>
    </interface>

Docker
------

OVS has a specialised command specifically for docker containers `ovs-docker`.
This tool will in the background, create a network interface inside the container and then also create a port on the integration bridge.
It is important to note that this way of providing network connectivity to a docker container does not follow the same rules as the standard docker or docker-compose.
We must also specify the ip address for this interface that is created and give it a DNS server - we default to 8.8.8.8.

Android Emulator
----------------

The Android Emulator (Android Virtual Device) requires special provisioning for it's network.
By itself, the emulator provisions it's own networking even if you utilise some of it's `qemu` directives to attach it to bridges etc. causing some issues.
Similar to how a docker container works, we place the emulator in it's own network namespace.
We then create a port on the integration bridge and insert it inside this emulators network namespace.
This way, we have completely isolated the emulator and force it's networking to go via the logical network.

Note that this does have implications in using the Android Debug Protocol (ADB) tooling.
The ADB server needs to be started inside the namespace, as it is listening on localhost.
Therefore ADB will be listening on the namespace's localhost, and will not be aware of other emulators in other network namespaces.

Future Guest Types
------------------

In the future we aim to add other guest types, but they will generally follow how we integrate libvirt, docker and android emulators.
For example, it is possible to place a browser inside a network namespace like the Android Emulator and have it running as a guest completely inside the logical network.
This means you do not have to put the browser inside a VM unnecessarily.
Additionally, other networks can be connected to the logical network this way such as wireless access points connected to the host via ethernet.

Guests as Routers and Firewalls
================================

A valid use case is to use a virtual machine running router software for the network.
This is something that we don't yet officially support or have tested.