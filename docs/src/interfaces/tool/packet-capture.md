---
title: INTERFACES-TOOL-PACKET-CAPTURE
section: 1
---

# Packet Capture

The testbed allows you to capture packets from OVN by providing a wrapper around `tcp-dump`.
It works in a similar way to `ovs-tcpdump` in that it manages the lifecycle of creating and destroying the mirror port for you.

You can use this command with:

`kvm-compose tool tcp-dump -i <interface> -f <pcap file output location>`

Where the interface is the name of the interface of the guest.
Note that this needs to be the `port` name on the OVN integration bridge `br-int` for your guest's interface.
The guest interface and port name are the same.

File output is the location you want to place the pcap file.
