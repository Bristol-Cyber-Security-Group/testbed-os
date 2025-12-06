# Packet Capture

This packet capture tool is to be used in the background from the testbed server.
The packet capture will work directly on the OVS ports.

This is primarily aimed to be used as a crate in the rest of the testbed, but this can optionally be used as a CLI tool.
Usage as a CLI tool is currently experimental and not supported, only use for testing.

You can build the CLI with `cargo build --bin testbedos-tcpdump --features cli-binary`.
You can then find the binary in `kvm-compose/target/debug/testbedos-tcpdump`.
To use this you must specify an interface and an output file location, and optionally a BPF filter syntax.
For example:
```shell
# assuming you are in the rust target debug folder where the binary has been built ...

sudo ./testbedos-tcpdump -i vm-ovn00 -o /home/debian/capture.pcap dst host 10.0.0.21

# this will choose OVS port vm-ovn00
# this will output to a pcap file called capture.pcap in /home/debian
# the BPF syntax 'dst host 10.0.0.21' is placed always at the end, but it is optional - see tcpdump docs for syntax info

```
