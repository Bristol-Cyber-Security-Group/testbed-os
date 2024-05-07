cd ../assets/ || exit

echo "Attempting to create libvirt test harness network, destroying any existing"
sudo virsh net-destroy test-harness-network
sudo virsh net-undefine test-harness-network
sudo virsh net-define testbed-network.xml
sudo virsh net-start test-harness-network
