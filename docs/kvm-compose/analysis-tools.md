Analysis Tools
---------------

The analysis tools subcommand for kvm-compose is available to be used against the network or the guests on a running testbed deployment.
The following are the available tools and some information on how they work.
In general the command line tooling will be a wrapper around an existing tool with some checks to make sure the arguments are appropriate for the testbed and to work on multiple testbed hosts.

tcpdump
_______

Tcpdump and ovs-tcpdump are supported in the testbed, with the constraint of limiting you to only be able to work on bridges that have been created by the testbed.
Additionally, the command will also work on bridges that have been assigned to other testbed hosts that are part of the testbed (non-main testbed hosts).
The outputs, if any, will be pulled from the remote testbed hosts into the current working directory, which will be the testbed project directory you are working from.

Both tcpdump and ovs-tcpdump take similar command line arguments, which can be given to the kvm-compose tool and they will be passed on to the respective tool.
Essentially the tcpdump analysis tool is just a wrapper around tcpdump and ovs-tcpdump, with the added checks and pulling of files from the remote.
The tool also supports the expected behaviour of ctrl-c to gracefully cancel a packet capture.

