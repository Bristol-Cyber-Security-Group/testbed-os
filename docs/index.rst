====================================
Welcome to Testbed-OS documentation!
====================================

``Testbed-OS`` is a testbed for launching virtual machines on abstract topologies to support research and testing.
In addition to the tooling in the testbed, we aim to deliver a variety of example test cases that showcase the capability of the testbed as a reference to build your own test cases.
The testbed uses yaml files to describe the testbed components to be deployed.
The testbed will parse the yaml file and create the artefacts in the yaml file, create the network described in the yaml and then deploy the various virtual machines on this network.
These steps are described further in the rest of the documentation.

Please see |getting-started| to do the initial configuration of the testbed server before running anything on the testbed.

For guidance on the different topics:

- |kvm-compose.yaml| section for the schema of the yaml file
- |orchestration| section for the deployment approach
- |networking| section for information on the network architecture of the testbed
- |installation|, |host.json| sections for initial setup before using yaml files to create testbed deployments

To get started with your first test case, see the |examples| topic which will walk you through a minimal test case building up a |kvm-compose.yaml| file.

.. toctree::
    :maxdepth: 3
    :caption: Contents:

    getting-started/index
    installation/index
    kvm-compose/index
    kvm-compose-yaml/index
    testbed-config/index
    orchestration/index
    exec/index
    guest-types/index
    gui/index
    testbedos-server/index
    networking/index
    resource-monitoring/index
    test-harness/index
    examples/index

.. |kvm-compose.yaml| replace:: :ref:`kvm-compose-yaml/index:kvm-compose Yaml`
.. |orchestration| replace:: :ref:`orchestration/index:orchestration`
.. |networking| replace:: :ref:`networking/index:Networking`
.. |installation| replace:: :ref:`installation/index:Testbed OS Installation`
.. |examples| replace:: :ref:`examples/index:Examples`
.. |host.json| replace:: :ref:`testbed-config/index:Testbed Config`
.. |getting-started| replace:: :ref:`getting-started/index:Getting Started`
