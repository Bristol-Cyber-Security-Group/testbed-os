============
kvm-compose
============

``kvm-compose`` is a cli tool used to generate the configuration files and artefacts for |orchestration|, to deploy arbitrary network topologies and virtual machine configurations across any number of hosts.
It uses a yaml file |kvm-compose.yaml| as the source of configuration that describes a test case to be deployed on the testbed.
This tool will enumerate all the `machine` and network components and generate a series of scripts or artefacts that describe how to deploy them on the testbed.

There are two main artefacts, the state JSON file and the artefacts folder.
The state JSON file contains a complete description of the testbed components, ranging from the testbed hosts that will be used, testbed guests to be deployed and the various openvswitch bridges and tunnels to be created.
The artefacts folder will contain the various scripts and config files to be used to express the configuration in the state JSON file and therefore the original |kvm-compose.yaml| file.

This tool also offers the ability to create the |kvm-compose-config.json|, which needs to be enumerated with the testbed host information.
The testbed host information is used in the configuration generation to load balance the testbed guests and openvswitch bridges across the total distributed testbed.

The tool also offers the use of analysis tools for security under `analysis-tools` sub command, see the analysis tools section for more information.

.. toctree::
    :maxdepth: 2
    :caption: Contents:

    architecture
    kvm-compose-yaml/index
    analysis-tools
    usage

.. seealso::

    * Tool |orchestration|
    * Yaml Schema |kvm-compose.yaml|
    * Server Config Schema |kvm-compose-config.json|

.. |kvm-compose.yaml| replace:: :ref:`kvm-compose/kvm-compose-yaml/index:kvm-compose Yaml`
.. |kvm-compose-config.json| replace:: :ref:`testbed-config/index:Testbed Config`
.. |orchestration| replace:: :ref:`orchestration/index:orchestration`
