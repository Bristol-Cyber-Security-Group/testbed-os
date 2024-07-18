========
Examples
========

This document will outline how to set up a test case for the testbed.
It is assumed you have installed the testbed dependencies and testbed code (see installation documentation).
It is also assumed you have configured the |kvm-compose-config.json|.

Please see the example projects in the `example/` folder in the root of the git repo.
These example projects have been created to showcase different features of the testbed.

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
        network:
          - switch: sw0
            gateway: 10.0.0.1
            mac: "00:00:00:00:00:01"
            ip: "10.0.0.10"
        libvirt:
          cpus: 1
          memory_mb: 1024
          libvirt_type:
            cloud_image:
              name: ubuntu_20_04
              expand_gigabytes: 2

    network:
      ovn:
        switches:

          sw0:
            subnet: "10.0.0.0/24"


This is a simple example, with one logical switch and one cloud-init virtual machine attached to this switch.
Please see the |kvm-compose.yaml| documentation and schema for more information on each element in the yaml.

We are now ready to deploy this minimal test case.
You can do this by first generating the artefacts:

.. code-block:: bash

    # make sure you are still inside the example folder
    kvm-compose generate-artefacts

You will see an `artefacts` folder and an `example-state.json` file have been created.
Have a look inside these to find out more at what has been created.
Once you are happy, we can now run the orchestration:

.. code-block:: bash

    # make sure you are still inside the example folder
    kvm-compose up

If you installed `virtual-manager`, you can open it up to see the guest appear shortly.
Once the orchestration has finished, you are able to access your guest through `virtual-manager`.
For virtual manager, you can just double click the guest in the list and a console window will open.


If you want to look at the networking components created, you can use:

.. code-block:: bash

    # at the host level
    ip a
    # at the openvswitch level (virtual)
    sudo ovs-vsctl show
    # at the open virtual networks level (logical)
    sudo ovn-nbctl show

You will see the different components created at each level to support the networking of the virtual machine.

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

