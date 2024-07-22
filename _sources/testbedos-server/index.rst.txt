================
TestbedOS Server
================

``TestbedOS Server`` is a server that runs in the background as a daemon to keep state of the testbed deployments.
It offers a REST API to control the testbed.
This API is used by the `kvm-compose` CLI tool and for the web interface.

The server is solely a wrapper around the `kvm-compose` library, which can be invoked without the server - see the `kvm-compose` usage document.
Note that when not using the server you lose the ability to deal with deployments and just work from the current project folder.

The objective of the server is to keep track of state in the testbed, this means tracking any existing deployments if you have multiple and check to see the up/down state.
The server also allows inspecting and modifying the configuration of the `kvm-compose-config` through the API.

.. toctree::
    :maxdepth: 2
    :caption: Contents:

    architecture
    api

