Testbed Mode
============

This is a simple file read on startup to dictate the mode, either master or client.

Master
------

The contents of the JSON file in master mode is simply the string "Master":

.. code-block:: json

    "Master"

Client
------

The contents of the JSON file in client mode requires two options.
This file can be manually created or will be made by the `testbedos-server` binary when running in client mode with these two arguments.

.. code-block:: json

    {
        "Client": {
            "master_ip": "10.50.0.1",
            "testbed_interface": "eth0"
        }
    }
