#!/bin/bash
set -e

#mkdir -p /testbed_sandbox/libvirt
#rm -rf /run/libvirt
#ln -s /testbed_sandbox/libvirt /run/libvirt

echo "Starting Libvirt daemon..."

mkdir -p /var/run/libvirt

# TODO - each as its own docker service?
virtlogd -d
virtlockd -d
exec libvirtd
