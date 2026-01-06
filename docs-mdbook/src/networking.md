# TestbedOS Networking

TestbedOS provides networking capabilities for the communication between the guests in a deployment to emulate real-world networks. In TestbedOS, this is implemented as a [Software-Defined Network (SDN)](https://en.wikipedia.org/wiki/Software-defined_networking) powered by [Open Virtual Networks (OVN)](https://www.ovn.org/en/), and [OpenvSwitch](https://www.openvswitch.org/), which operates at level 2 in the OSI model, underpins OVS. Via OVN and OVS, the SDN is constructed with components such as switches and routers, following familiar concepts in networking. To learn more about OVN and OVS, as well as how they work in TestbedOS, we provide a quick background on the topic in [OVN and OVS Brief Background](networking_background.md).

## TestbedOS Networking Components

We provide a way to describe the SDN configuration and topology in the `kvm-compose.yaml` file. We have chosen the following basic networking components from OVN to be available for configuration in the `kvm-compose.yaml` file and thus for deployment. We discuss some limitations on the available (logical) networking components in [Limitations](networking_limitations.md).

- Switches
- External IP addresses
- Routers
- DHCP
- NAT
- DNS

At this time, we only support IPv4.
OVN supports both, but our current implementation has IPv4 in mind.
We have added some support for IPv6 in parts of the code, but this is current untested in an end to end deployment.

The description for the networking section in the `kvm-compose.yaml` file of a deployment starts with the `network` field with the `ovn` field under that as OVN is currently the only SDN provider in TestbedOS. Examples of the usage of the available logical networking components in the `kvm-compose.yaml` file can be seen throughout the MWEs for the [libvirt guest](welcome.md#minimal-working-example), [Docker container guest](docker.md#minimal-working-example-mwe), and for the [Android guest](avd.md#minimal-working-example). The following sections will describe the specific fields that are relevant to each of the networking components.

## Switches

You can create a basic logical switch with the following:

``` yaml
sw0:
    subnet: "10.0.0.0/24"
```

This logical switch will have the subnet defined as metadata.
The subnet doesn't limit the IP addresses you statically assign, but it is used for other features of OVN such as [DHCP](#dhcp).

## Routers

Logical routers allow traffic to flow between logical switches.
This can be achieved by creating router ports that will be connected to logical switches.
You can also then set static routes to send traffic to specific ports, such as routing traffic to other logical switches or to the internet.
These routers are also responsible for providing [network address translation (NAT)](#nat) and [dynamic host configuration protocol (DHCP)](#dhcp).

A basic router with a port on a logical switch can be defined with:

``` yaml
routers:
    lr0:
        ports:
            - name: lr0-sw0
              mac: "00:00:00:00:ff:01"
              gateway_ip: "10.0.0.1/24"
              switch: sw0
```

This definition will create a router port connecting logical router `lr0` to logical switch `sw0`.
The port needs a mac address and an IP address.
These are important as guests need to know the gateway.

## External IP Addresses

TestbedOS allows external networking from inside the logical network of a deployment and out to the internet. 
This requires a couple of OVN components that need to be configured:
1. First is the external bridge `br-ex`. This is the second OVS bridge that OVN manages, and this bridge will be given a static IP address and we use `172.16.1.200` as default.
2. In the logical network we require a special logical switch which we name  `public` and it has a logical port of type `localnet`.
This `localnet` type exposes the host's networking, so that we can push network traffic through the OVS bridge `br-ex`.
This means, in the `host.json` file (see [TestbedOS Host Configuration](configurations.md#testbedos-host-configuration)), you will have to define at least one `"bridge_mappings"` value such as:

``` json
"bridge_mappings": [
    [
        "public",
        "br-ex",
        "172.16.1.1/24"
    ]
]
```

Note that you must use the name `public` and in the [Quick Installation process](welcome.md#quick-installation) for a `Main` TestbedOS host (see [Singleton Mode or the Main Host](configurations.md#singleton-mode-or-the-main-host)), this is already included in the `host.js` file during the installation.
 
The logical router configuration in the `kvm-compose.yaml` file of a deployment should identify an external gateway, which assigns a specific host as the "way out" of the OVN logical network. 
For example:

``` yaml
public:
    subnet: "172.16.1.0/24"
    ports:
    - name: ls-public
      localnet:
      network_name: public
```

If the router port is to be connected to a switch that is exposing the logical network to a testbed host's network.
This means an extra element is required, similar to the logical switch example above on exposing host networking:

``` yaml
- name: ls-public
  mac: "00:00:20:20:12:13"
  gateway_ip: "172.16.1.200/24"
  switch: public
  set_gateway_chassis: main
```

You must use the same chassis name for the TestbedOS host that you want to expose the network on.
In your `host.json` file, if the TestbedOS host is `main`, you must place `main` here as well.

## DHCP
<!-- Static and Dynamic Guest IP -->

We provide the capability of either specifying an IP address to a guest, or relying on DHCP.
OVN natively offers DHCP based on the subnet of the logical switch.
Logical ports on this logical switch with ip="dynamic" will be allocated an IP starting from the next lowest value in the subnet.

Currently, there is some incompatibility in using OVN's native DHCP and giving guests a static external IP address.
We look to resolve this in the future.

## NAT

It is possible to assign both "Source NAT" (snat) and "Destination NAT and Source Nat" (dnat_and_snat), where the former just allows the guest to access the internet and the latter also allows the guest to be addressed from outside the logical network.
For snat, this is compatible with guests with dynamic IP addresses.
For dnat_and_snat, this is only compatible with guests with static IP addresses.

## DNS

While OVN is comprehensive in many areas, DNS in it's current version as of writing this documentation (v23.03.0) is lacking.
For internal DNS, the OVN controller can route all DNS requests directly from the guest's port to itself to serve lookups.
However, this requires a combination of configuring the DNS entries in each logical switch and also having the guest with a dynamic IP address.
We found this to be cumbersome, in addition to being rather opinionated to potential use cases.
For example, if you want to investigate DNS traffic in your network for research purposes, say you are trying to model an old insecure network, then OVN would be obstructive in this scenario.
It is possible for the user to host a DNS agent in the network, but there would be some configuration of the guests on the user's part.

For external DNS, this will also require configuration on the user's side for the guests.
We have added 8.8.8.8 as a DNS server for guests with dynamic IP addresses as a default.
However, we are looking to generally improve the DNS story in the testbed in future updates.