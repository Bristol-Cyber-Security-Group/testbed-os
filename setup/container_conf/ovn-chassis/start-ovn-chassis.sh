#!/bin/bash
set -e

mkdir -p /testbed_sandbox/openvswitch
rm -rf /var/run/openvswitch
ln -s /testbed_sandbox/openvswitch /var/run/openvswitch

modprobe openvswitch || echo "Warning: Could not load openvswitch kernel module..."

if [ ! -f /etc/openvswitch/conf.db ]; then
    echo "Creating OVS database..."
    ovsdb-tool create /etc/openvswitch/conf.db /usr/share/openvswitch/vswitch.ovsschema
fi

echo "Starting ovsdb-server..."
ovsdb-server --remote=punix:/var/run/openvswitch/db.sock \
             --remote=db:Open_vSwitch,Open_vSwitch,manager_options \
             --pidfile --detach

# TODO - use --system-id= to set chassis name?
ovs-vsctl --no-wait init

echo "Starting ovs-vswitchd..."
ovs-vswitchd --pidfile --detach --log-file=/var/log/openvswitch/ovs-vswitchd.log

echo "OVS is running..."

echo "Configuring Chassis..."
ovs-vsctl --may-exist add-br br-int
ovs-vsctl --may-exist add-br br-ex

ip addr add 172.16.1.1/24 dev br-ex
ip link set br-ex up
#iptables -t nat -C POSTROUTING -o eth0 -s 10.50.0.1/24 -f MASQUERADE

# TODO - we should place the ip forwarding rules for the external bridge, either here or more the br-ex creation elsewhere


# TODO - this configuration should use external IPs for clustering, see host.json
#  ... if a client running this, needs to point to the "remote"
# TODO - there were other options added, see testbed server startup code
ovs-vsctl set open_vswitch . external-ids:ovn-remote=tcp:127.0.0.1:6642
ovs-vsctl set open_vswitch . external-ids:ovn-encap-type=geneve
ovs-vsctl set open_vswitch . external-ids:ovn-encap-ip=127.0.0.1
# these came from OVN setup docs
# TODO - parameterise these
ovs-vsctl set open_vswitch . external-ids:system-id=main
ovs-vsctl set open_vswitch . external-ids:ovn-bridge=br-int
ovs-vsctl set open_vswitch . external-ids:ovn-bridge-mappings="public:br-ex"


echo "Starting OVN local controller..."
/usr/share/ovn/scripts/ovn-ctl start_controller

echo "OVN is running..."

tail -f /var/log/openvswitch/ovs-vswitchd.log
