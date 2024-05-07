SUDO_ASKPASS=ssh-askpass rust-gdb --cd examples/signal/ --args /home/gjp/prj/rephrain-testbed/kvm-compose/target/debug/kvm-compose -v TRACE up
(cd examples/signal/ && sudo /home/gjp/prj/rephrain-testbed/kvm-compose/target/debug/kvm-compose -v TRACE up)
(cd examples/signal/ && sudo /home/gjp/prj/rephrain-testbed/kvm-compose/target/debug/kvm-compose -v TRACE down)
sudo virsh net-dhcp-leases testbed-network
sshpass -p password ssh nocloud@192.168.222.146 ping 192.168.222.208
