# Tool

TestbedOS additionally offers tools to run against the deployment environment that is not necessarily specific to a guest and this is done through the `tool` subcommand. The structure of the `tool` subcommand is as follows.

``` bash
kvm-compose tool <tool name> <tool options> [-h|--help]
```

The `help` documentation is available through the `-h` or the `--help` option for tool-specific documentation.

## Packet Capture

One of such `tool`s provided by TestbedOS is to allow the capture of packets from OVN, the Software-Defined Networking (SDN) provider of TestbedOS, via a wrapper around `tcp-dump`. It works in a similar way to `ovs-tcpdump` in that it manages the lifecycle of creating and destroying the mirror port.

The structure of the command is as follows.

``` bash
kvm-compose tool tcp-dump -i <interface> -f <pcap file output location>
```

### Options

`-i, --interface <interface>`
: Name of the interface of the guest, i.e., the port name on the OVN integration bridge `br-int` of the guest's interface. Note: the guest interface and the port name are the same.

`-f, --file-output <output filename>`
: Path of the output PCAP file.