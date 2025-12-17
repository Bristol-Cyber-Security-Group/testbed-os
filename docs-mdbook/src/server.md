# TestbedOS Server

## Starting the TestbedOS Server

There are three ways to start the server.
You can either use the server in daemon mode by running `sudo systemctl start testbedos-server.service`.
You can also directly run the server from the CLI with `sudo testbedos-server main`.
Or you can run via cargo, if you are in the testbedos-server project folder in the source code with `sudo -E bash -c  'cargo run -- main' $USER`.
Once you have successfully run the server once in main mode, you do not need to specify `main` unless you edit the `mode.json`.

You are now ready to use the testbed, you can either use an example in the ``examples/`` folder or roll your own.
Refer to the examples on how to build a ``kvm-compose.yaml`` file.

The basic syntax is to be in a folder with a ``kvm-compose.yaml`` defined and run ``kvm-compose generate-artefacts`` to generate config.
See :ref:`orchestration <orchestration/index:orchestration>` for more information on how to deploy a test case.

You should not need to use sudo with the command, unless you are using a resource (such as an existing disk, file to push into vm with cloud-init) that your user does not have permission for.