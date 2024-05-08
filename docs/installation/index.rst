=======================
Testbed OS Installation
=======================

There are various dependencies needed to be installed with some based configuration needed to get the testbed ready to deploy test cases.
This installation guide will outline each component needed.

The following dependency install has been packaged together into the ``pre-req-setup.sh`` script in the root of the repo (for ubuntu/debian based distros, replace ``apt`` with your distributions package manager i.e. ``pacman``).
Note that this script will try to edit your bash profile such as ``~/.bashrc``, please check the script and comment out anything you do not want to be executed.
This script is used in the projects test quite (see test-harness folder in the root of the repo), so will be working on a fresh ubuntu install.

You can either follow the instructions in this document, or run ``./pre-req-setup.sh`` script in the root of the repo automate the pre-requisite dependencies installation.

Host Dependencies
-----------------

:Rust: https://rustup.rs/
    Default configuration, when asked, is fine.
:Poetry: https://python-poetry.org/docs/#installation
    Make sure you have python `^3.10`, consider using pyenv to manage python installs (see |poetry_docs|_ ).
    Also make sure you have pip3 installed for this python version, for ubuntu install `python3-pip`
:PyEnv: https://github.com/pyenv/pyenv
    Python version manager, asks you to manually add it to your shell profile once installed.

Poetry has been used to manage the python virtual environments for this project.
While it is possible to use others, you will need to manually replace the use of ``poetry run`` for example with your own virtual environment management.
The use of ``poetry run`` for ad-hoc use of the python environment to remove the need to load the virtual environment in the current session.


Compile-time
^^^^^^^^^^^^

``sudo apt install libvirt-dev libssl-dev gcc make zlib1g zlib1g-dev libssl-dev libbz2-dev libsqlite3-dev libffi-dev libncurses5 libncurses5-dev libncursesw5 libreadline-dev lzma liblzma-dev libbz2-dev libtool autoconf``

For building python with pyenv

``sudo apt install libffi-dev libncurses5 libncurses5-dev libncursesw5 libreadline-dev lzma liblzma-dev libbz2-dev``

Runtime
^^^^^^^

``sudo apt install qemu-kvm libvirt-daemon-system libvirt-clients libnss-libvirt genisoimage``

OVN and OVS
^^^^^^^^^^^

We need to build OVN and OVS from source, we use the OVS submodule so that we match the OVN to OVS version.

.. code-block:: shell

    git clone https://github.com/ovn-org/ovn.git
    cd ovn
    git checkout v23.03.0
    ./boot.sh
    git submodule update --init
    cd ovs
    ./boot.sh
    ./configure
    make
    sudo make install
    cd ..
    ./configure
    make
    sudo make install

Docker
^^^^^^

Docker is used to allow the user to deploy containers on the testbed, the installation steps for docker are the same as in `https://docs.docker.com/desktop/install/linux-install/`.

Android Emulator
^^^^^^^^^^^^^^^^

Android Virtual Device is used to deploy Android emulators in the testbed, we need to install the following CLI tools.
The command line tools must also be downloaded from Google at https://developer.android.com/studio#command-tools


.. code-block:: shell

    sudo apt-get -y install openjdk-17-jdk
    sudo apt-get -y install android-tools-adb
    sudo mkdir -p /opt/android-sdk/cmdline-tools/
    sudo chown $USER: -R /opt/android-sdk/
    CMDLINETOOLS_URL=https://dl.google.com/android/repository/commandlinetools-linux-10406996_latest.zip
    wget $CMDLINETOOLS_URL -O /opt/android-sdk/cmdline-tools/tools.zip
    unzip /opt/android-sdk/cmdline-tools/tools.zip -d /opt/android-sdk/cmdline-tools/
    mv /opt/android-sdk/cmdline-tools/cmdline-tools /opt/android-sdk/cmdline-tools/latest
    rm /opt/android-sdk/cmdline-tools/tools.zip


You will need to add the following to your environment assuming you are using bash, do this once:

.. code-block:: shell

    # add to ~/.bashrc
    export PATH=$PATH:/opt/android-sdk/cmdline-tools/latest/bin
    export ANDROID_HOME=/opt/android-sdk/
    export ANDROID_SDK_ROOT=/opt/android-sdk/


You will need to run `source ~/.bashrc` to load these new variables.

You must accept the licenses with `sdkmanager --licenses` or `yes | sdkmanager --licenses` to auto accept.

Then you will need to install the emulator with `sdkmanager --install "emulator" "platform-tools"`.

Optional
^^^^^^^^

``sudo apt install virt-manager``

Virtual Machine manager is a useful GUI for libvirt, which allows you to inspect the network and guest configuration.
It also allows you to open a graphical window to the guest which will either be a terminal or the graphical desktop if installed.

Consider using ``sudo virsh console <guest name>`` to open a TTY to the guest as the graphical window may not support copy paste etc without guest tools installed.

Setup Testbed
-------------

Clone the testbed git repo into your desired location then navigate to the root directory.
Execute::

    ./setup.sh

to compile the rust code, build the poetry virtual environments and documentation for the project.


Setup local DNS
---------------

With the libnss-libvirt package, you will need to edit the following file /etc/nsswitch.conf and add libvirt to the hosts config as so (your list may differ, that is fine just add libvirt to the end)::

    hosts:          files mdns4_minimal resolve dns mymachines libvirt

This will allow you to SSH directly to the guest via their hostname which will resolve to their local IP address.
You must do this or the ``kvm-orchestrator`` tool will not work.

Configure Libvirt User Permissions
----------------------------------

You will need to add the user that will interface with the libvirt daemon and give it permission to use it.

Edit ``/etc/libvirt/qemu.conf`` file and find the following section::

    #       user = "+0"     # Super user (uid=0)
    #       user = "100"    # A user named "100" or a user with uid=100
    #
    #user = "root"

    # The group for QEMU processes run by the system instance. It can be
    # specified in a similar way to user.
    #group = "root"

change this section into (for example, if my username is ubuntu)::

    #       user = "+0"     # Super user (uid=0)
    #       user = "100"    # A user named "100" or a user with uid=100
    #
    user = "ubuntu"

    # The group for QEMU processes run by the system instance. It can be
    # specified in a similar way to user.
    group = "libvirt"

Once this is changed, make sure to restart the libvirt daemon: ``sudo systemctl restart libvirtd``.

If you have multiple users for libvirt or a locked down linux system, please see the libvirt documentation on how to manage this.
The target supported platform for the testbed currently assumes you have administrator privileges and are the single user.

Setup kvm-compose Config
------------------------

You will need to create the ``kvm-compose-config.json`` file and enumerate it with the testbed host information that will participate in the testbed.
You must do this before running the testbed or it will not know what are the testbed hosts.
See |kvm-compose-config| documentation for more information.



Run Testbed
-----------

There are three ways to start the server.
You can either use the server in daemon mode by running `sudo systemctl start testbedos-server.service`.
You can also directly run the server from the CLI with `sudo testbedos-server master`.
Or you can run via cargo, if you are in the testbedos-server project folder in the source code with `sudo -E bash -c  'cargo run -- master' $USER`.
Once you have successfully run the server once in master mode, you do not need to specify `master` unless you edit the `mode.json`.

You are now ready to use the testbed, you can either use an example in the ``examples/`` folder or roll your own.
Refer to the examples on how to build a ``kvm-compose.yaml`` file.

The basic syntax is to be in a folder with a ``kvm-compose.yaml`` defined and run ``kvm-compose generate-artefacts`` to generate config.
See :ref:`orchestration <orchestration/index:orchestration>` for more information on how to deploy a test case.

You should not need to use sudo with the command, unless you are using a resource (such as an existing disk, file to push into vm with cloud-init) that your user does not have permission for.


Testbed Cluster
---------------

It is possible to create a cluster of testbed hosts to increase the resource capability of your testbed.
The testbed hosts must be accessible i.e. on the same local network.
You will still need to individually configure each host's `host.json`.
You will then need to start the non master testbed hosts in client mode.
This is similar to the master mode commands, but instead you can use the following methods:

- ``sudo testbedos-server client -m <ip of master testbed host> -t <interface visible to master host on local network>```
- ``sudo -E bash -c  'cargo run -- client -m <ip of master testbed host> -t <interface visible to master host on local network>' $USER```
- If you are using the ``systemctl``` method, you must make sure the `mode.json` in ``/var/lib/testbedos/config/`` has been configured with the client configuration

Similar to the master mode, once you have successfully run the server in the client mode, you do not have to specify the client with arguments as this will be read from the `mode.json`.
Please see the testbed server |Cluster Management| for more information.

Limitations
^^^^^^^^^^^

Be aware that if you do use sudo, the files created may required elevated permissions to use so you will there-on need to continue to use sudo unless you manually edit the owner (`chown`) or permissions (`chmod`).

If you use kvm-compose up with or without sudo, if you are using cloud-init images, then be aware that the images downloaded will either go to ``/root/.kvm-compose/`` if you use sudo or ``/home/<your home folder/.kvm-compose/`` if you do not.
This means that you may end up downloading the images twice, once in each folder if you interchange the use of sudo.


Tear Down Testbed
-----------------

You should tear down any test cases before uninstalling the testbed, see :ref:`orchestration <orchestration/index:orchestration>` for more information on how to tear down a test case.

If you want to the testbed (assuming all vms and networking components have been destroyed), you can use the ``tear-down.sh`` script in the root of the testbed-or repo to remove the kvm-compose binary and python code+environments originally installed via setup.sh.


.. |poetry_docs| replace:: ``poetry documentation``
.. _poetry_docs: https://python-poetry.org/docs/managing-environments/
.. |kvm-compose-config| replace:: :ref:`kvm-compose-config.json <testbed-config/index:Testbed Config>`
.. |Cluster Management| replace:: :ref:`testbedos-server/architecture:Cluster Management`
