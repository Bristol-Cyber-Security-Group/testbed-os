TestbedOS Networking Architecture
##################################

This document describe the components of the TestbedOS networking system and its components.

Background
**********

The testbed provides a way to describe a network topology in the |kvm-compose.yaml| file to emulate real world networks.
This network is a `Software Defined Network <https://en.wikipedia.org/wiki/Software-defined_networking>`_ (SDN) powered by `Open Virtual Networks <https://www.ovn.org/en/>`_ (OVN).
Via OVN, the network is constructed with components such as switches and routers, following familiar concepts in networking.
OVN is used in `OpenStack <https://www.openstack.org/>`_ and is a capable networking tool for cloud scenarios, but it is also capable in the context of the testbed.
`OpenvSwitch <https://www.openvswitch.org/>`_ (OVS) underpins OVN, operating at level 2 in the OSI model.
The testbed will be creating and configuring both OVN and OVS to deploy the network, but once configured, OVN will be controlling the network behaviours.

OVN is a powerful for networking as it allows creating a cluster of many hosts running OVN, while providing a single API to control the network behaviours over all the hosts.
This means you can scale out the testbed to provide more resources to host more virtual machines, without needing to worry about how you configure a distributed network.
Furthermore, the physical location of these virtual machines is now not important since their vision of the network is defined in the logical network.
For example, you have two hosts A and B, and on host A you have a virtual machine X and on host B you have a virtual machine Z.
In your logical network, you place both virtual machines X and Z on the same logical switch.
In the background, OVN will tunnel the network traffic between hosts A and B such that the virtual machines X and Z have no perception of the underlying physical network being over two hosts.
This not only simplifies the network definition on the user's side, it also simplifies the technical backend that the testbed needs to configure as this is all handled by OVN.
Additionally, scaling out the testbed is as simple as configuring the OVN daemon on the new host to point to the main OVN host, which is also managed by the testbed software for you.

Theoretically, you could create a large cluster of hosts running the testbed with virtual machines load balanced across all hosts.
These virtual machines could be configured in any way on the logical network level, in addition to many isolated logical networks in the same or different user deployments.
This is essentially how cloud infrastructures work with the various tenancies sharing the underlying hypervisors for their workloads.

OVN Background
**************

The following text will give a high level description of how we use OVN and OVS, but see the OVN `architecture man page <https://www.ovn.org/support/dist-docs/ovn-architecture.7.html>`_ for more details.
To break down how the network is configured, we will first discuss how a network is defined in OVN, then how we configure OVS and then how the configuration in OVN will control OVS to provide the SDN.
It is important to note that OVN is also the controller for the SDN, where the OVS bridges will be configured to use the OVN controller for the flow rules.

OVN lets the user work at the "logical network" level, which is then converted into flow rules for the OVS bridges.
The logical network has a more complex and flexible abstraction over a network, when compared to flow rules.
OVN will convert the logical network definition into complex flow rules to create the behaviours and constraints of the logical network.
For example, you can define a logical switch that is connected to a logical router via logical ports.
These logical constructs are converted into various flow rules to achieve this logical network.
The OVS bridges in the network will then apply these flow rules on the network traffic.

OVS bridges are configured to use the OVN controller to obtain the flow rules.
These bridges are where you attach the network interfaces of guests, such as a libvirt virtual machine.
On each host that is part of the OVN cluster, OVN creates a single OVS bridge called "br-int" short for integration bridge.
There is another bridge called "br-ex" which is short for external bridge, this is covered below in the external networking section.
Each host running OVN only needs one integration bridge, and all virtual machines will be attached to this bridge.
Even if the virtual machines are part of separate logical networks, or on different logical switches, it does not matter as the network isolation is handled with flow rules.
You do not need to create an OVS bridge per logical network or per logical switch, and this is why OVN is so powerful.

To finally make the association between the OVS bridges and the OVN logical network, the port IDs on the OVS bridge for the virtual machine interfaces are specified in the OVN logical ports.
Logical ports in logical switches and logical routers.
Specifically for virtual machines, we can define logical ports on the logical switches.
These logical ports can be given an ID of the port on any of the integration bridges that are part of the OVN cluster.
This association allows OVN to route the network traffic to and from the port of the virtual machine's interface on the OVN network.

Testbed OVN Implementation and Limitations
******************************************

While the testbed uses OVN, there is a small constraint on what APIs available from OVN are exposed to the user via the |kvm-compose.yaml|.
The basic networking components are available:

- switches
- switch ports
- routers
- router ports
- DHCP
- NAT

In addition to applying routing rules on the routers, providing external IP addresses and basic DNS.
However, the testbed abstraction over the OVN api is somewhat opinionated in this version of the testbed.
For example, we do not expect the user to need high availability configuration in OVN.
Additionally, to avoid overly pushing the testbed opinion on a network configuration, we have not provided configurable DNS.
This is explained more further down.
For very complex networks with very specific requirements, we need to assess how this impacts the testbed's API so that it remains general.
In future versions we look to open up the API to include more configurability of the network.
All these constraints that we have applied are validated in code, so if there is something we don't support but OVN does, then that will be rejected in the yaml parsing phase.


OVN Components
==============

In the previous list, there are the OVN components that are used in the testbed.
There is significant detail in how these work and the different configuration options.
For more information on each, please see the `ovn-nbctl <https://www.ovn.org/support/dist-docs/ovn-nbctl.8.html>`_ CLI documentation for a starting point.

IPv4 and IPv6
=============

At this time, we only support IPv4.
OVN supports both, but our current implementation has IPv4 in mind.
We have added some support for IPv6 in parts of the code, but this is current untested in an end to end deployment.

Static and Dynamic Guest IP
===========================

We provide the capability of either specifying an IP address to a guest, or relying on DHCP.
OVN natively offers DHCP based on the subnet of the logical switch.
Logical ports on this logical switch with ip="dynamic" will be allocated an IP starting from the next lowest value in the subnet.

Currently, there is some incompatibility in using OVN's native DHCP and giving guests a static external IP address.
We look to resolve this in the future.

Multiple Interfaces on Guests
=============================

In the kvm-compose.yaml file you can specify one or more interfaces for guests.
Currently, only libvirt guests support multiple interfaces.

The libvirt guests will have their domain.xml generated with the list of interfaces defined in the yaml file.
For libvirt cloud-init guests, this interface information is placed in the cloud-init network config and will boot with the interfaces configured automatically.

External Networking
===================

To allow external networking from inside the logical network and out to the internet, there are a couple of OVN components that need to be configured.
First is the external bridge "br-ex".
This is the second OVS bridge that OVN manages, and this bridge will be given a static IP address - we use 172.16.1.200 as a default.
In the logical network we require a special logical switch which we name "public", which has a logical port of type "localnet".
This localnet type exposes the host's networking, so that we can push network traffic through the OVS bridge br-ex.
This works in a combination with a logical router configuration identifying an external gateway, which assigns a specific host as the "way out" of the OVN logical network.

NAT
===

It is possible to assign both "Source NAT" (snat) and "Destination NAT and Source Nat" (dnat_and_snat), where the former just allows the guest to access the internet and the latter also allows the guest to be addressed from outside the logical network.
For snat, this is compatible with guests with dynamic IP addresses.
For dnat_and_snat, this is only compatible with guests with static IP addresses.

Internal and External DNS
=========================

While OVN is comprehensive in many areas, DNS in it's current version as of writing this documentation (v23.03.0) is lacking.
For internal DNS, the OVN controller can route all DNS requests directly from the guest's port to itself to serve lookups.
However, this requires a combination of configuring the DNS entries in each logical switch and also having the guest with a dynamic IP address.
We found this to be cumbersome, in addition to being rather opinionated to potential use cases.
For example, if you want to investigate DNS traffic in your network for research purposes, say you are trying to model an old insecure network, then OVN would be obstructive in this scenario.
It is possible for the user to host a DNS agent in the network, but there would be some configuration of the guests on the user's part.

For external DNS, this will also require configuration on the user's side for the guests.
We have added 8.8.8.8 as a DNS server for guests with dynamic IP addresses as a default.
However, we are looking to generally improve the DNS story in the testbed in future updates.

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

Observing Network Traffic in OVN
********************************

As the traffic in SDNs are not like classic networks, it can be a bit more awkward to observe the traffic due to all the flow rules.
While it is possible to run `ovs-tcpdump` on the OVS bridges, you may not find what you expect i.e. you see all the traffic.
Note that `ovs-tcpdump` is a specific version of `tcp-dump` for OVS bridges, we include the python dependencies in the testbed - either through the analysis tooling or the poetry environment.

OVN also provides ways to virtually test traffic from two endpoints, to test if your network works as intended.
Please see the documentation on `ovn-trace <https://www.ovn.org/support/dist-docs/ovn-trace.8.html>`_.

.. |kvm-compose.yaml| replace:: :ref:`kvm-compose/kvm-compose-yaml/index:kvm-compose Yaml`

