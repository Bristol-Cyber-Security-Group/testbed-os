Host JSON
=========

The host JSON contains basic information about how the testbed can use the network interfaces of the host for the guests it will create.
This is also used for the OVN configuration.

The following is an example `host.json`:

.. code-block:: json

    {
      "ip": "10.50.0.1",
      "user": "ubuntu",
      "identity_file": "/home/ubuntu/.ssh/id_ed25519",
      "testbed_nic": "eth0",
      "main_interface": "wlo1",
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

These are generally good defaults you can use in your own testbed, apart from the username and identity_file path.

The IP will be the IP this testbed is accessible at for other testbed hosts, if only using one testbed this can be `127.0.0.1`.

The `testbed_nic` is also the interface the testbed can be accessible to other testbeds, if only using one testbed this can be `127.0.0.1`.

The `main_interface` is the interface that the testbed will use to allow guests to access the internet.
In this example, we use the wireless interface of the host as a possible interface - say you are running the testbed on a laptop connected to WiFi.

The OVN section of this JSON file is used to configure OVN.
You can use this example yourself, for a testbed in main mode.
As long as you are OK with the NAT ip addresses of guests leaving the logical network in the `172.16.1.1/24` subnet.
The `encap_ip` here should usually be the same as the host's IP, as this is the IP OVS uses to create the overlay network to other testbed hosts.
If you are using the testbed in client mode, you will need to set `client_ovn_remote` to the IP of the main i.e. `tcp:10.50.0.1:6642`.
You need to specify the protocol and port to the main's OVN server - so just replace the ip in this example to the IP of your main.

Note that when in client mode, the main will only need some of this whole configuration but it will require this JSON to be valid before it accepts the client join request.


Cluster Mode
------------

The above example shows you how to set up the main testbed server.
To set up the client testbed server(s), you must edit the `host.json` slightly.

Note that, we recommend having the testbed servers communicate via a dedicated LAN that is separate to the main internet connection.
IT is currently unsupported using the same network interface for main internet connection and for OVN/testbed networking.

If the main testbed server has the IP `10.50.0.1` and the client we are configuring has ip `10.50.0.2`, you can do the following.
Note that the `chassis_name` must be unique in your cluster.

.. code-block:: json

    {
      "ip": "10.50.0.2",
      "user": "ubuntu",
      "identity_file": "/home/ubuntu/.ssh/id_ed25519",
      "testbed_nic": "eth0",
      "main_interface": "wlo1",
      "is_main_host": true,
      "ovn": {
        "chassis_name": "client1",
        "bridge": "br-int",
        "encap_type": "geneve",
        "encap_ip": "10.50.0.2",
        "main_ovn_remote": "tcp:10.50.0.1:6642",
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

Once that is done, you can then run:

.. code-block:: shell

    sudo testbedos-server client -m 10.50.0.1 -t eth0

Then you can check with OVN on the main testbed host to see the client chassis appear with:

.. code-block:: shell

    sudo ovn-sbctl show

And you will see each chassis listed.

