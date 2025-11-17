# Packet Capture

This packet capture tool is to be used in the background from the testbed server.
The packet capture will work directly on the OVS ports or bridges.

TODOs:
- need to create and clean up the mirror ports
- figure out how we deal with the situation where the VM is on a remote host
- how this works as a producer and speaks X database input
- testing
  - since we need privilege to run against networking components and a live environment then need integration tests
- in terms of threading, we don't want to run the risk of starving the main testbed server when lots of packets
  - should we create a separate thread rather than use one of the (four) threads on the server
- 

