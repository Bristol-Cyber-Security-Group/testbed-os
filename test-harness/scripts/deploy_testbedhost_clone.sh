#!/bin/bash

#[[ ! -z "$1" ]] && echo "building a clone called $1" || (echo "No clone name specified as an argument to this script" && exit 1)

CLONE_NAME="testbed-host-$1"
CLONE_MAC_ADDRESS="$2"

# create linked clone
cd ../artefacts/ || exit
sudo virsh destroy $CLONE_NAME
sudo virsh undefine $CLONE_NAME
rm $CLONE_NAME.qcow2 || true
qemu-img create -f qcow2 -F qcow2 -b testbedhost_base.img $CLONE_NAME.qcow2

# create each linked clones specific metadata mount
mkdir -p ../artefacts/$CLONE_NAME/
cp ../assets/iso/meta-data ../artefacts/$CLONE_NAME/meta-data
cp ../assets/iso/user-data ../artefacts/$CLONE_NAME/user-data
cp ../assets/iso/network-config ../artefacts/$CLONE_NAME/network-config
sed -i "s/testbed-server/$CLONE_NAME/g" ../artefacts/$CLONE_NAME/meta-data

sudo virt-install \
  --name $CLONE_NAME \
  --memory 5020 \
  --vcpus 2 \
  --disk $CLONE_NAME.qcow2 \
  --import \
  --os-variant ubuntufocal \
  --network network=test-harness-network \
  --mac $CLONE_MAC_ADDRESS \
  --cloud-init user-data="../artefacts/$CLONE_NAME/user-data",meta-data="../artefacts/$CLONE_NAME/meta-data",network-config="../artefacts/$CLONE_NAME/network-config" \
  --graphics none \
  --noautoconsole \
  --noreboot

## phase 2: test testbedos test cases
# TODO - make sure linked clone has ip address

# TODO - push a predetermined kvm-compose-config
