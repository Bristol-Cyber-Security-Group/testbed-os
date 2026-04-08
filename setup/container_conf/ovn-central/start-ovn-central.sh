#!/bin/bash
set -e

# ovn central runs the north and south DBs

# remove the sockets from the container's filesystem, create the folder from the sandbox volume
# then symlink from the volume into the container
mkdir -p /testbed_sandbox/ovn
rm -rf /var/run/ovn
ln -s /testbed_sandbox/ovn /var/run/ovn

echo "Starting OVN Central databases and northd..."
/usr/share/ovn/scripts/ovn-ctl start_northd

ovn-nbctl set-connection ptcp:6641:0.0.0.0
ovn-sbctl set-connection ptcp:6642:0.0.0.0


echo "OVN Central is running and listening on ports 6641/6642..."

tail -f /var/log/ovn/ovn-northd.log
