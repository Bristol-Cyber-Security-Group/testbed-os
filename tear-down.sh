#!/bin/bash

# remove kvm-compose binary
KVMCOMPOSELOC=$(which kvm-compose)
sudo rm "$KVMCOMPOSELOC"
echo "kvm-compose removed $KVMCOMPOSELOC"

# stop and remove the server service
sudo systemctl stop testbedos-server.service
sudo systemctl disable testbedos-server.service
sudo rm /etc/systemd/system/testbedos-server.service
sudo systemctl daemon-reload

# remove the server
SERVERLOC=$(which testbedos-server)
sudo rm "$SERVERLOC"
echo "testbedos-server removed $SERVERLOC"


# kvm-ui-tui
KVMUILOC_SYMLINK=$(which kvm-ui-tui)
# get the parent folder twice as the UI folder holds the multiple UI related entrypoints
KVMTUILOC_ABSOLUTE=$(dirname $(readlink -f $(which kvm-ui-tui)))


sudo rm "$KVMTUILOC_SYMLINK"
echo "kvm-ui-tui symlink removed $KVMTUILOC_SYMLINK"
sudo rm -rf "$KVMTUILOC_ABSOLUTE"
echo "kvm-ui-tui installed dir removed $KVMTUILOC_ABSOLUTE"

# remove man pages from system TODO

# delete testbed configuration folder in /var/lib/testbedos/ TODO


echo "done."
