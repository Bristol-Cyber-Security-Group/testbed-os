==================
Testbed Config
==================

    The testbed requires two JSON documents before it will be able to run.
    The `host.json` contains information about the current testbed host that will take part of the testbed.
    The `mode.json` contains information on how the testbed behaves in master or client mode.
    These files must exist before executing any |kvm-compose| commands.

    The host file is located in `/var/lib/testbedos/config/host.json`.

    The  mode file located at `/var/lib/testbedos/config/mode.json`.


.. toctree::
    :maxdepth: 2
    :caption: Contents:

    host
    mode

.. |kvm-compose| replace:: :ref:`kvm-compose/index:kvm-compose`
