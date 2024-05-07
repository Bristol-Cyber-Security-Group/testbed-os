#!/bin/bash

### setup one testbed host

# create a copy of the source cloud init image, delete any existing which may have state
echo "Creating a copy of the cloud init image to work from if it does not already exist, removing any old copies if they exist."
sudo virsh destroy testbed-host-base
sudo virsh undefine testbed-host-base
rm ../artefacts/testbedhost_base.img || true
cp ../artefacts/focal-server-cloudimg-amd64.img ../artefacts/testbedhost_base.img

# TODO - check if there already are linked clones causing a write lock on this image from subsequent runs
# resize the image
qemu-img resize ../artefacts/testbedhost_base.img +20G

cd ../assets/ || exit

# create a customised cloudinit metadata for this host
mkdir -p ../artefacts/testbedhost_base/
cp ../assets/iso/meta-data ../artefacts/testbedhost_base/meta-data
cp ../assets/iso/user-data ../artefacts/testbedhost_base/user-data
cp ../assets/iso/network-config ../artefacts/testbedhost_base/network-config
sed -i 's/testbed-server/testbedhost-base/g' ../artefacts/testbedhost_base/meta-data

# create kvm
sudo virt-install \
  --name testbed-host-base \
  --memory 2048 \
  --vcpus 2 \
  --disk ../artefacts/testbedhost_base.img \
  --import \
  --os-variant ubuntufocal \
  --network network=test-harness-network \
  --cloud-init user-data="../artefacts/testbedhost_base/user-data",meta-data="../artefacts/testbedhost_base/meta-data",network-config="../artefacts/testbedhost_base/network-config" \
  --graphics none \
  --noautoconsole \
  --noreboot

# wait for host to be up
echo "Waiting for the VM to come up and accept SSH connections."
for ii in {1..6}
  do
    if ssh -i ssh_key/id_ed25519 -o "StrictHostKeyChecking no" -o "UserKnownHostsFile /dev/null" nocloud@testbedhost-base true; then
      echo "testbed host up"
      break
    else
      echo "testbed host not up, waiting 10s"
      sleep 10
    fi
    if [ $ii -eq 6 ]; then
      echo "testbed host was never reachable"
      exit
    fi
done

# push current release codebase
rsync -avr -e "ssh -i ssh_key/id_ed25519 -o 'StrictHostKeyChecking no' -o 'UserKnownHostsFile /dev/null'"  \
  --exclude "test-harness/" \
  --exclude "artefacts/*" \
  --exclude "target/*" \
  --exclude ".git/*" \
  ../../../testbed-os \
  nocloud@testbedhost-base:/home/nocloud/

# install testbedos tools
ssh -i ssh_key/id_ed25519 -o "StrictHostKeyChecking no" -o "UserKnownHostsFile /dev/null" nocloud@testbedhost-base "cd /home/nocloud/testbed-os/ && ./pre-req-setup.sh" || exit 1
ssh -i ssh_key/id_ed25519 -o "StrictHostKeyChecking no" -o "UserKnownHostsFile /dev/null" nocloud@testbedhost-base "cd /home/nocloud/testbed-os/ && bash -l ./setup.sh" || exit 1

# set the libvirt user (so we can work with guest images in the home folder, not just in /var/lib/libvirt/images/)
ssh -i ssh_key/id_ed25519 -o "StrictHostKeyChecking no" -o "UserKnownHostsFile /dev/null" nocloud@testbedhost-base "sudo sed -i 's/#user = \"root\"/user = \"root\"/g' /etc/libvirt/qemu.conf"
ssh -i ssh_key/id_ed25519 -o "StrictHostKeyChecking no" -o "UserKnownHostsFile /dev/null" nocloud@testbedhost-base "sudo sed -i 's/#group = \"root\"/group = \"root\"/g' /etc/libvirt/qemu.conf"
ssh -i ssh_key/id_ed25519 -o "StrictHostKeyChecking no" -o "UserKnownHostsFile /dev/null" nocloud@testbedhost-base "sudo systemctl restart libvirtd"

# quick test to see if the install script worked
POETRY_EXISTS=`ssh -i ssh_key/id_ed25519 -o "StrictHostKeyChecking no" -o "UserKnownHostsFile /dev/null" nocloud@testbedhost-base "cd /home/nocloud/testbed-os/ && bash -l which poetry || exit"`
PYENV_EXISTS=`ssh -i ssh_key/id_ed25519 -o "StrictHostKeyChecking no" -o "UserKnownHostsFile /dev/null" nocloud@testbedhost-base "cd /home/nocloud/testbed-os/ && bash -l which pyenv || exit"`
KVM_COMPOSE_EXISTS=`ssh -i ssh_key/id_ed25519 -o "StrictHostKeyChecking no" -o "UserKnownHostsFile /dev/null" nocloud@testbedhost-base "cd /home/nocloud/testbed-os/ && bash -l which kvm-compose || exit"`
KVM_ORCHESTRATOR_EXISTS=`ssh -i ssh_key/id_ed25519 -o "StrictHostKeyChecking no" -o "UserKnownHostsFile /dev/null" nocloud@testbedhost-base "cd /home/nocloud/testbed-os/ && bash -l which kvm-orchestrator || exit"`
KVM_UI_EXISTS=`ssh -i ssh_key/id_ed25519 -o "StrictHostKeyChecking no" -o "UserKnownHostsFile /dev/null" nocloud@testbedhost-base "cd /home/nocloud/testbed-os/ && bash -l which kvm-ui-cli || exit"`
DOCKER_EXISTS=`ssh -i ssh_key/id_ed25519 -o "StrictHostKeyChecking no" -o "UserKnownHostsFile /dev/null" nocloud@testbedhost-base "cd /home/nocloud/testbed-os/ && bash -l which docker || exit"`

[[ ! -z "$POETRY_EXISTS" ]] && echo "Poetry Successfully Installed" || (echo "Poetry Not installed" && exit 1)
[[ ! -z "$PYENV_EXISTS" ]] && echo "Pyenv Successfully Installed" || (echo "Pyenv Not installed" && exit 1)
[[ ! -z "$KVM_COMPOSE_EXISTS" ]] && echo "kvm-compose Successfully Installed" || (echo "kvm-compose Not installed" && exit 1)
[[ ! -z "$KVM_ORCHESTRATOR_EXISTS" ]] && echo "kvm-orchestrator Successfully Installed" || (echo "kvm-orchestrator Not installed" && exit 1)
[[ ! -z "$KVM_UI_EXISTS" ]] && echo "kvm-ui-cli Successfully Installed" || (echo "kvm-ui-cli Not installed" && exit 1)
[[ ! -z "$DOCKER_EXISTS" ]] && echo "docker Successfully Installed" || (echo "docker Not installed" && exit 1)

# push cloud init image for guests
ssh -i ssh_key/id_ed25519 -o "StrictHostKeyChecking no" -o "UserKnownHostsFile /dev/null" nocloud@testbedhost-base "sudo mkdir -p /var/lib/testbedos/images" || exit 1
rsync -av -e "ssh -i ssh_key/id_ed25519 -o 'StrictHostKeyChecking no' -o 'UserKnownHostsFile /dev/null'"  \
  --exclude "test-harness/" \
  --exclude "artefacts/*" \
  --exclude "target/*" \
  --exclude ".git/*" \
  ../artefacts/focal-server-cloudimg-amd64.img \
  nocloud@testbedhost-base:/home/nocloud/ || exit 1
ssh -i ssh_key/id_ed25519 -o "StrictHostKeyChecking no" -o "UserKnownHostsFile /dev/null" nocloud@testbedhost-base "sudo mv /home/nocloud/focal-server-cloudimg-amd64.img /var/lib/testbedos/images/ubuntu_20_04.img" || exit 1

# make sure ssh keys have correct permissions before being pushed
chmod 600 ssh_key/id_ed*
# push ssh keys into .ssh
rsync -av -e "ssh -i ssh_key/id_ed25519 -o 'StrictHostKeyChecking no' -o 'UserKnownHostsFile /dev/null'"  \
  ssh_key/id_ed* \
  nocloud@testbedhost-base:/home/nocloud/.ssh/

# done
