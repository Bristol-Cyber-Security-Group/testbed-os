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

sudo rm -rf /var/lib/testbedos/tools/*
echo "testbed tooling has been removed from the data directory"

# delete testbed data directory with all deployment information etc
read -p "Do you want to also clear the whole TestbedOS data directory, including all deployment information?"
answer=${answer,,}
if [[ "$answer" == "y" ]]; then
    sudo rm -rf /var/lib/testbedos/
    echo "testbed data directory cleared"
elif [[ "$answer" == "n" ]]; then
    echo "Will not delete the data directory"
else
    echo "Please enter 'y' or 'n'. Exiting ..."
    exit 1
fi


echo "done."
