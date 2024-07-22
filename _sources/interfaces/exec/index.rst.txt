Exec Commands
=============

The kvm-compose CLI offers a way to run commands and tools against the guests.
This facilitates accessing the guest for the user as the guest may be in a network namespace or on a client testbed host.
There are also built in tools that have been prepared to run analysis against the guests.

The format of the exec is similar to docker exec where the base of the command:

`kvm-compose exec <guest name>`

and the following subcommands can be added, add `--help` to see what is available.

For example, if you want to run the `ls` command on a guest called `client1` you can run the following:

`kvm-compose exec client1 shell-command ls`

and the testbed server will run this command on the guest, on whichever testbed host this guest is on.

If this is a tool packages with the testbed, for example:

`kvm-compose exec phone tool frida-setup`

this will install the frida server in the guest (android guests only).

For more information on each possible command in `exec` see:

.. toctree::
    :maxdepth: 2
    :caption: Contents:

    tool
    shell-command
    user-script

