Getting Started
===============

Before you can start deploying test cases on the testbed, there are a few required setup steps.
Please follow the following sections in order, before using the testbed.

Installation
------------

You will need to install the testbed if you have not already done this.
The testbed git repo contains install scripts.
Please see the |installation documentation| for more information.

Configuring the Server
----------------------

Once you have installed the testbed, you will need to configure the server.
This requires you to make sure the ``host.json`` in the testbed folder ``/var/lib/testbedos/config/`` folder is set up correctly.
Please see the |kvm-compose-config| documentation on how to set this up.

The master testbed server also needs a ``mode.json`` file set to "Master".
If you used the ``setup.sh`` script, then this will have been placed for you with master as the default.

Launching the Server
--------------------

Once the testbed is configured, you can start the server.
Please see how to |run the testbed| on the various commands.

Adding a client Testbed to the Testbed Cluster
----------------------------------------------

If you are adding more testbed hosts to the cluster to provide more resource capability, the setup is very similar to the first testbed.
You must follow the same installation and configuration steps.
However, instead of running the testbed in master mode, it will need to be in client mode.
Please see the |testbed cluster| documentation for more information.

Running Test Cases
------------------

Now you have a working testbed installation, you can now start deploying test cases.
We provide a CLI, a TUI, and a GUI to interact with the testbed.
Please see the |yaml examples| to construct your own |kvm-compose.yaml|.


.. |installation documentation| replace:: :ref:`installation/index:Testbed OS Installation`
.. |kvm-compose-config| replace:: :ref:`testbed-config/index:Testbed Config`
.. |run the testbed| replace:: :ref:`installation/index:Run Testbed`
.. |testbed cluster| replace:: :ref:`installation/index:Testbed Cluster`
.. |kvm-compose.yaml| replace:: :ref:`kvm-compose-yaml/index:kvm-compose Yaml`
.. |yaml examples| replace:: :ref:`examples/index:Examples`
