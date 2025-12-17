# TestbedOS Clustering Mode

## Testbed Cluster

It is possible to create a cluster of testbed hosts to increase the resource capability of your testbed.
The testbed hosts must be accessible i.e. on the same local network.
You will still need to individually configure each host's `host.json`.
You will then need to start the non main testbed hosts in client mode.
This is similar to the main mode commands, but instead you can use the following methods:

- ``sudo testbedos-server client -m <ip of main testbed host> -t <interface visible to main host on local network>```
- ``sudo -E bash -c  'cargo run -- client -m <ip of main testbed host> -t <interface visible to main host on local network>' $USER```
- If you are using the ``systemctl``` method, you must make sure the `mode.json` in ``/var/lib/testbedos/config/`` has been configured with the client configuration

Similar to the main mode, once you have successfully run the server in the client mode, you do not have to specify the client with arguments as this will be read from the `mode.json`.
Please see the testbed server |Cluster Management| for more information.

Note that, we recommend having the testbed servers of the client hosts communicate via a dedicated LAN that is separate to the `"Main"` host's internet connection.
Currently, using the same network interface for both the `"Main"` host's internet connection and OVN or TestbedOS networking is unsupported.

### Limitations

Be aware that if you do use sudo, the files created may required elevated permissions to use so you will there-on need to continue to use sudo unless you manually edit the owner (`chown`) or permissions (`chmod`).

If you use kvm-compose up with or without sudo, if you are using cloud-init images, then be aware that the images downloaded will either go to ``/root/.kvm-compose/`` if you use sudo or ``/home/<your home folder/.kvm-compose/`` if you do not.
This means that you may end up downloading the images twice, once in each folder if you interchange the use of sudo.