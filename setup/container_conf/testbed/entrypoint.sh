#!/bin/bash
set -e

echo "Make sure that all the config files exist in the correct place..."

mkdir -p /var/lib/testbedos/config/
cp mode.json /var/lib/testbedos/config/mode.json

mkdir -p /var/lib/testbedos/deployments/

mkdir -p /var/lib/testbedos/keys/
cp ./id_ed25519_testbed_insecure_key /var/lib/testbedos/keys/
cp ./id_ed25519_testbed_insecure_key.pub /var/lib/testbedos/keys/

cp -r assets/ /var/lib/testbedos/

cat << EOF > /var/lib/testbedos/config/host.json
{
  "ip": "10.50.0.1",
  "user": "root",
  "identity_file": "/root/.ssh/id_ed25519",
  "testbed_nic": "eth0",
  "main_interface": "eth0",
  "is_main_host": true,
  "ovn": {
    "chassis_name": "main",
    "bridge": "br-int",
    "encap_type": "geneve",
    "encap_ip": "10.50.0.1",
    "main_ovn_remote": "unix:/usr/local/var/run/ovn/ovnsb_db.sock",
    "client_ovn_remote": null,
    "bridge_mappings": [
      [
        "public",
        "br-ex",
        "172.16.1.1/24"
      ]
    ]
  }
}
EOF

echo "Running the testbed server..."

# TODO - remove the ovs rundir here and either add it to the subprocess environment or mount ovs db into tbos service
OVS_RUNDIR=/testbed_sandbox/openvswitch/ /app/testbedos-server main
