---
title: SCHEMA-NETWORKING-TOOLS
section: 1
---

# Guest Networking Tools

## tcpdump in OVN

As the traffic in SDNs are not like classic networks, it can be a bit more awkward to observe the traffic due to all the flow rules.
While it is possible to run `ovs-tcpdump` on the OVS bridges, you may not find what you expect i.e. you see all the traffic.
Note that `ovs-tcpdump` is a specific version of `tcpdump` for OVS bridges.

We include our own version of `tcpdump` to address some of the [limitations](schema_network_limitations.md).

OVN also provides ways to virtually test traffic from two endpoints, to test if your network works as intended.
Please see the documentation on [ovn-trace](https://www.ovn.org/support/dist-docs/ovn-trace.8.html).
