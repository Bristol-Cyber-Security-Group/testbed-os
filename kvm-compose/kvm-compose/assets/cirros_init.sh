
mkdir /home/cirros/.ssh
cirros-query get public_ssh_key > /home/cirros/.ssh/authorized_keys
chmod 600 /home/cirros/.ssh/authorized_keys
chown cirros /home/cirros/.ssh/authorized_keys

mkdir -p /etc/nocloud/context
cirros-query get run_script > /etc/nocloud/run_script.sh
chmod +x /etc/nocloud/run_script.sh

mkdir /nocloudtmp
mount /dev/sr0 /nocloudtmp
tar -xf /nocloudtmp/context.tar -C /etc/nocloud/context
umount /nocloudtmp
rm -rf /nocloudtmp

sh /etc/nocloud/run_script.sh > /etc/nocloud/run_script_out.txt 2>/etc/nocloud/run_script_err.txt
