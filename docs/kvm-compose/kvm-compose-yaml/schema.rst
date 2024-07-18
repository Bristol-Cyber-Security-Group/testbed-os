======
Schema
======

The top level of the schema has four main sections:

:machines: a list of machine definitions that specify the guest type and further specialisations
:network: the network definition, including switch and topology definition
:tooling: a list of tools and specialisation to be used with the testbed
:testbed_options: a list of options to customise the testbed that doesn't fall under the above sections

These sections are explained further below.

Machines
--------

The machine section allows you to define different guests on the testbed.
These definitions require you to specify the guest type such as libvirt, docker or avd.

The schema for machines have a few levels of hierarchy to help you define and specialise the guest.
At the top level, you can specify the following:

:name: the unique name of the machine
:network: optional, a list of interface definitions for the guest
:machine type: this is the first level of specialisation, you must pick one of the supported guest types

For example, the following are snippets of the relevant parts possible machine definitions:

.. code-block:: yaml

    - name: libvirt-guest
      network:
        - switch: sw0
          gateway: 10.0.0.1
          mac: "00:00:00:00:00:01"
          ip: "10.0.0.10"
      libvirt:
        ...

.. code-block:: yaml

    - name: docker-guest
      network:
        - switch: sw0
          gateway: 10.0.0.1
          mac: "00:00:00:00:00:01"
          ip: "10.0.0.10"
      docker:
        ...

.. code-block:: yaml

    - name: avd-guest
      network:
        - switch: sw0
          gateway: 10.0.0.1
          mac: "00:00:00:00:00:01"
          ip: "10.0.0.10"
      avd:
        ...

Machines - Libvirt
------------------
The libvirt subsection of the schema offers the following libvirt specific options:

:cpus: number of virtual cpus to assign the guest
:memory_mb: amount of memory in megabytes to assign the guest
:libvirt_type: a further specialisation for the different types of libvirt guests, see |libvirt machines| for details
:scaling: optional, allows you to scale out from one machine definition and create clones see |scaling| section for more information

The `libvirt_type` section (see |libvirt machines|) requires you to pick from the supported types such as cloud image, existing disk or iso guest.
For example, the following are snippets of the relevant parts possible libvirt machine definitions:

.. code-block:: yaml

    - name: cloud-image-guest
      libvirt:
        libvirt_type:
          cloud_image:
            name: ubuntu_20_04

.. code-block:: yaml

    - name: existing-disk-guest
      libvirt:
        libvirt_type:
          existing_disk:
            path: /path/to/prebuilt/image.img

.. code-block:: yaml

    - name: iso-guest
      libvirt:
        libvirt_type:
          iso_guest:
            path: /path/to/install/iso.iso

The `scaling` option has restrictions on what it can be used with.
It can only be used with `cloud_image` and `existing_disk`.
It cannot exist at the same time as the higher level `interfaces` section, as it has its own options for this.
The following options are supported:

:count: the number of clones to be made
:interfaces: optional, a list of bridges and the clone guests assigned to that bridge, each bridge can take a list of clone ids
:shared_setup: optional, a script that will be executed on the backing image before clones are made
:clone_setup: optional, a list of scripts and assigned clones that will be executed on the clones
:clone_run: optional, a list of scripts and assigned clones that will be executed on the clones at the end of orchestration

For example, the following are snippets of the relevant parts possible scaling definitions:

.. code-block:: yaml

    # 1) this snippet will create 2 clones from a cloud image definition (ommited with ...)
    # 2) the clones are assigned to separate switches
    # 3) the clones will be cloned from an image created with the shared script already executed
    # 4) the clones will have a setup script executed on them when they are setup, they both have the same script

    - name: scaling-guest
      libvirt:
        libvirt_type:
          cloud_image:
            ... # 1)
        scaling:
          count: 2
          interfaces: # 2)
            sw0:
              clones: [0]
              gateway: "10.0.0.1"
              ip_type: dynamic
              mac_range:
                from: "00:00:00:00:00:01"
                to: "00:00:00:00:00:01"
            sw1:
              clones: [1]
              gateway: "10.0.0.1"
              ip_type: dynamic
              mac_range:
                from: "00:00:00:00:00:02"
                to: "00:00:00:00:00:02"
          shared_setup: shared.sh # 3)
          clone_setup:
            - script: install.sh
              clones: [0, 1] # 4)

For further information on the `libvirt_type` sub-schema, see |libvirt machines|.

Machines - Docker
-----------------

The docker subsection of the schema offers the following docker specific options:

:image: the container image to be used
:command: a command that will override the image's CMD, if it exists
:entrypoint: an entrypoint that will override the image's ENTRYPOINT, if it exists
:environment: a key value map of environment variables to give the container
:env_file: a file containing environment variables, similar to the environment option
:volumes: a list of mount points from the host to container
:privileged: option to allow the container to be run as privileged
:device: a list of device mount points, similar to volumes

These are 1-1 replication from the docker run schema, for more information on the behaviours of these see https://docs.docker.com/engine/reference/commandline/run/

For example, the following are snippets of the relevant parts possible docker machine definitions:

.. code-block:: yaml

    - name: nginx
      network:
        - switch: sw0
          gateway: 10.0.0.1
          mac: "00:00:00:00:00:01"
          ip: "10.0.0.10"
      docker:
        image: nginx:stable
        env_file: docker.env
        volumes:
          - source: ${PWD}/html  # the schema allows you to reference the project dir as ${PWD}
            target: /usr/share/nginx/html

    - name: one-off
      network:
        - switch: sw0
          gateway: 10.0.0.1
          mac: "00:00:00:00:00:01"
          ip: "10.0.0.10"
      docker:
        image: busybox:latest
        command: "curl project-nginx:80"  # assuming the deployment name is "project"


There is also the `scaling` option, to allow defining multiple docker containers from one definition.
It cannot exist at the same time as the higher level `interfaces` section, as it has its own options for this.
The following options are supported:

:count: the number of clones to be made
:interfaces: optional, a list of bridges and the clone guests assigned to that bridge, each bridge can take a list of clone ids

For example, the following are snippets of the relevant parts possible scaling definitions:

.. code-block:: yaml

    - name: nginx
      docker:
        image: nginx:stable
        env_file: docker.env
        volumes:
          - source: ${PWD}/html
            target: /usr/share/nginx/html
        scaling:
          count: 2
          interfaces:
          sw0:
            clones: [0]
            gateway: "10.0.0.1"
            ip_type: dynamic
            mac_range:
              from: "00:00:00:00:00:01"
              to: "00:00:00:00:00:01"
          sw1:
            clones: [1]
            gateway: "10.0.0.1"
            ip_type: dynamic
            mac_range:
              from: "00:00:00:00:00:02"
              to: "00:00:00:00:00:02"

This example creates two nginx containers on different bridges, but they both share the same config - in this case the same set of mounted HTML.

Further notes:

- docker containers are assigned an IP address by the testbed statically, meaning their IPs will increment up from the network's gateway IP.
- mounting volumes
  - absolute paths should be used
  - but we allow `${PWD}` to refer to the project folder where the kvm-compose.yaml file exists, similar to how docker-compose.yaml works
- guests inside the network can access the containers by their name through the testbeds DNS server, but from the host you must address them by IP


Machines - AVD
--------------
Not yet implemented.

Network
-------

The network section offers the following option, currently only one network schema available:

:ovn: the main network schema

Under OVN schema the following elements are available:

:switches: these are the logical switches
:routers: these are the logical routers

Both of these have different options for creating your logical network topology.

Switches
********

You can create a basic logical switch with the following:

.. code-block:: yaml

    sw0:
      subnet: "10.0.0.0/24"

This logical switch will have the subnet defined as metadata.
The subnet doesn't limit the ip addresses you statically assign, but it is used for other features of OVN such as DHCP.

You can create special logical switches that expose the host's networking to the logical switch, via the external OVS bridge usually called `br-ex`.
For example:

.. code-block:: yaml

      public:
        subnet: "172.16.1.0/24"
        ports:
          - name: ls-public
            localnet:
              network_name: public

For this switch, you must specify a port of type `localnet` that is mapped to the bridge mapping for the `br-ex` OVS bridge.
This means, in the `host.json` file, you will have defined at least one `bridge_mappings` value such as:

.. code-block:: text

    "bridge_mappings": [
      [
        "public",
        "br-ex",
        "172.16.1.1/24"
      ]
    ]

You must use the name `public`.

Note, to connect switches together, you must use logical routers.

Routers
*******

Logical routers allow traffic to flow between logical switches.
This can be achieved by creating router ports that will be connected to logical switches.
You can also then set static routes to send traffic to specific ports, such as routing traffic to other logical switches or to the internet.
These routers are also responsible for providing network address translation (NAT) and dynamic host configuration protocol (DHCP).

A basic router with a port on a logical switch can be defined with:

.. code-block:: yaml

    routers:

      lr0:
        ports:

          - name: lr0-sw0
            mac: "00:00:00:00:ff:01"
            gateway_ip: "10.0.0.1/24"
            switch: sw0

This definition will create a router port connecting logical router `lr0` to logical switch `sw0`.
The port needs a mac address and an IP address.
These are important as guests need to know the gateway.

If the router port is to be connected to a switch that is exposing the logical network to a testbed host's network.
This means an extra element is required, similar to the logical switch example above on exposing host networking:

.. code-block:: yaml

    - name: lr0-public
      mac: "00:00:20:20:12:13"
      gateway_ip: "172.16.1.200/24"
      switch: public
      set_gateway_chassis: main

You must use the same chassis name for the testbed host that you want to expose the network on.
In your `host.json` file, if the testbed host is named `main`, you must place `main` here as well.

Tooling
-------
No tooling options via the yaml are implemented at the moment.

Testbed Options
---------------
No options options via the yaml are implemented at the moment.


.. |kvm-compose| replace:: :ref:`kvm-compose/index:kvm-compose`
.. |cloud-images| replace:: :ref:`kvm-compose/usage:subcommands`
.. |networking| replace:: :ref:`networking/index:Networking`
.. |scaling| replace:: :ref:`kvm-compose/architecture:Scaling`
.. |libvirt machines| replace:: :ref:`kvm-compose/kvm-compose-yaml/libvirt:Libvirt Type`
