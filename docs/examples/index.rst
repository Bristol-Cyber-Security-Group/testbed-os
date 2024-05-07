========
Examples
========

This document will outline how to set up a test case for the testbed.
It is assumed you have installed the testbed dependencies and testbed code (see installation documentation).
It is also assumed you have configured the |kvm-compose-config.json|.

Creating a minimal test case
----------------------------

This example will show you a minimal test case with pointers at the different things that happen during setting up the testbed.

In a location in your filesystem, create a folder named after the name of your test case.
For this example, we will created the project `example` in the home directory:

.. code-block:: bash

    cd ~
    mkdir example
    cd example

Now you are in the `example` folder, we must create the yaml file.
In your text editor or IDE, create a file in the example folder called `kvm-compose.yaml`.
The contents of your file for this example:

.. code-block:: yaml

    machines:

      - name: server
        interfaces:
          - bridge: br0
        libvirt:
          cpus: 1
          memory_mb: 1024
          libvirt_type:
            cloud_image:
              name: ubuntu_20_04
              expand_gigabytes: 2

    bridges:
      - name: br0
        protocol: OpenFlow13

    external_bridge: br0

This is a simple example, with one openvswitch bridge and one cloud-init virtual machine attached to this bridge.
Please see the |kvm-compose.yaml| documentation and schema for more information on each element in the yaml.

We are now ready to deploy this minimal test case.
You can do this by first generating the artefacts:

.. code-block:: bash

    # make sure you are still inside the example folder
    kvm-compose generate-artefacts

You will see an `artefacts` folder and a `example-state.json` file have been created.
Have a look inside these to find out more at what has been created.
Once you are happy, we can now run the orchestration:

.. code-block:: bash

    # make sure you are still inside the example folder
    kvm-compose up

If you installed `virtual-manager`, you can open it up to see the guest appear shortly.
Once the orchestration has finished, you are able to access your guest through `virtual-manager` or SSH.
For virtual manager, you can just double click the guest in the list and a console window will open.
You can log in with the default credentials `nocloud` and `password`.
If you want to connect with SSH, you must use the location of the key specified in the |kvm-compose-config.json| for guests and run:

.. code-block:: bash

    ssh -i path/to/your/guest/key nocloud@example-server

Note that the hostname for the guest is {project name}-{guest name}.

If you want to look at the networking components created, you can use:

.. code-block:: bash

    ip a

and you will see `example-br0` has been created, `example-prjbr0` and some veths.
You can then also run:

.. code-block:: bash

    sudo ovs-vsctl show

and you will see information about the openvswitch bridge created (example-br0).

Once you are done, you can destroy the test case with:

.. code-block:: bash

    kvm-compose down

The guest will be destroyed and the networking components will also be destroyed.
Note that the artefacts folder will remain,
You can run an `up` again and bring back the test case without running `generate-artefacts`, but note that the libvirt guest images can retain state.

For more examples for the yaml, see the yaml |kvm-compose.yaml examples|.

.. |kvm-compose.yaml| replace:: :ref:`kvm-compose-yaml/index:kvm-compose Yaml`
.. |kvm-compose.yaml examples| replace:: :ref:`kvm-compose-yaml/schema:Schema`
.. |kvm-compose-config.json| replace:: :ref:`testbed-config/index:Testbed Config`

