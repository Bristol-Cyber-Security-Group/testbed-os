# OVN and OVS Brief Background

As previously mentioned in [TestbedOS Networking](networking.md), [Open Virtual Networks (OVN)](https://www.ovn.org/en/), underpinned by [OpenvSwitch (OVS)](https://www.openvswitch.org/), is the provider for TestbedOS's Software-Defined Networking (SDN) capability. 

OVN is used in [OpenStack](https://www.openstack.org) and it is a capable networking tool for cloud infrastructure scenarios, but it is also capable in the more lightweight context of TestbedOS.
TestbedOS creates and configures both OVN and OVS to deploy the network, but once configured, OVN will be controlling the network behaviours.

## OVN as a Networking Abstraction Overlay Layer for Host Clusters

OVN is a powerful for networking as it allows creating a cluster of many hosts running OVN, while providing a single API to control the network behaviours over all the hosts.
This means you can scale out the testbed to provide more resources to host more virtual machines, without needing to worry about how you configure a distributed network.
Furthermore, the physical location of these virtual machines is now not important since their vision of the network is defined in the logical network.

For example, you have two hosts A and B, and on host A you have a virtual machine X and on host B you have a virtual machine Z.
In your logical network, you place both virtual machines X and Z on the same logical switch.
In the background, OVN will tunnel the network traffic between hosts A and B such that the virtual machines X and Z have no perception of the underlying physical network being over two hosts.

This not only simplifies the network definition on the user's side, it also simplifies the technical backend that TestbedOS needs to configure as this is all handled by OVN.
Additionally, scaling out a deployment in TestbedOS is as simple as configuring the OVN daemon on the new host to point to the main OVN host, which is also managed by TestbedOS for you.

Theoretically, you could create a large cluster of hosts running a TestbedOS deployment with virtual machines load balanced across all hosts.
These virtual machines could be configured in any way on the logical network level, in addition to many isolated logical networks in the same or different user deployments.
This is essentially how cloud infrastructures work with the various tenancies sharing the underlying hypervisors for their workloads.

## OVN and OVS TestbedOS Internals

We will give a high level description of how we use OVN and OVS, but see the OVN [architecture man page](https://www.ovn.org/support/dist-docs/ovn-architecture.7.html) for more details.
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
On each host that is part of the OVN cluster, OVN creates a single OVS bridge called `br-int` short for integration bridge.
There is another bridge called `br-ex` which is short for external bridge, this is covered in the [External Guest IP Addresses](networking.md#external-guest-ip-addresses).
Each host running OVN only needs one integration bridge, and all virtual machines will be attached to this bridge.
Even if the virtual machines are part of separate logical networks, or on different logical switches, it does not matter as the network isolation is handled with flow rules.
You do not need to create an OVS bridge per logical network or per logical switch, and this is why OVN is so powerful.

To finally make the association between the OVS bridges and the OVN logical network, the port IDs on the OVS bridge for the virtual machine interfaces are specified in the OVN logical ports.
Logical ports in logical switches and logical routers.
Specifically for virtual machines, we can define logical ports on the logical switches.
These logical ports can be given an ID of the port on any of the integration bridges that are part of the OVN cluster.
This association allows OVN to route the network traffic to and from the port of the virtual machine's interface on the OVN network.