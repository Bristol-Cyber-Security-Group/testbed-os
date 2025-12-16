# TestbedOS Dependencies

There are various dependencies needed to be installed with some based configuration needed to get the testbed ready to deploy test cases.
This installation guide will outline each component needed. 

## Host Dependencies

**[Rust](https://rustup.rs/)**
The Rust programming language. Default configuration, when prompted, is fine.

**[Poetry](https://python-poetry.org/docs/#installation)**
Please make sure you have python version `3.10` and above, consider using pyenv to manage python installs (see [Poetry Documentation](https://python-poetry.org/docs/managing-environments/)).
Also make sure you have `pip3` installed for this python version, for Ubuntu install `python3-pip`.

**[PyEnv](https://github.com/pyenv/pyenv)**
Python version manager, asks you to manually add it to your shell profile once installed.

Poetry has been used to manage the python virtual environments for TestbedOS.
While it is possible to use others, you will need to manually replace the use of `poetry run` for example with your own virtual environment management.
The use of `poetry run` for ad-hoc use of the python environment to remove the need to load the virtual environment in the current session.

### Runtime Dependencies

**`genisoimage`** is used to create .iso files for cloud-image guest startup configuration.

**`virt-manager`** is used as a graphical viewer for libvirt guests. 
Virtual Machine manager is a useful GUI for libvirt, which allows you to inspect the network and guest configuration.
It also allows you to open a graphical window to the guest which will either be a terminal or the graphical desktop if installed.
Consider using `sudo virsh console <project name with the guest name>`, e.g., `sudo virsh console mwe-server` for the MWE in [Minimal Working Example](welcome.md#minimal-working-example), to open a TTY to the guest as the graphical window may not support copy paste etc without guest tools installed.

### Networking Dependencies

TestbedOS uses Open Virtual Network (OVN) and Open Virtual Switch (OVS) as the underlying Software-Defined Networking (SDN) provider for the networking capabilities of the deployments.
As below, we build OVN and OVS from source and we use the OVS submodule so that we match the OVN to OVS version. 

```    
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
```

### Docker

Docker is used to allow the user to deploy containers on TestbedOS, the installation steps for Docker are the same as in [Install Docker Desktop on Linux](https://docs.docker.com/desktop/install/linux-install/).
If you are already using Docker Desktop, the TestbedOS installation reuses the existing Docker installation.

### Android Emulator

Android Virtual Device is used to deploy Android emulators on TestbedOS, we install the following CLI tools.
The command line tools must also be downloaded from Google at [Android Studio - Command line tools only](https://developer.android.com/studio#command-line-tools-only).

```
sudo apt-get -y install openjdk-17-jdk
sudo apt-get -y install android-tools-adb
sudo mkdir -p /opt/android-sdk/cmdline-tools/
sudo chown $USER: -R /opt/android-sdk/
CMDLINETOOLS_URL=https://dl.google.com/android/repository/commandlinetools-linux-10406996_latest.zip
wget $CMDLINETOOLS_URL -O /opt/android-sdk/cmdline-tools/tools.zip
unzip /opt/android-sdk/cmdline-tools/tools.zip -d /opt/android-sdk/cmdline-tools/
mv /opt/android-sdk/cmdline-tools/cmdline-tools /opt/android-sdk/cmdline-tools/latest
rm /opt/android-sdk/cmdline-tools/tools.zip
```

The following is added to your local environment, specifically, to the `~/.bashrc` file, assuming you are using bash:

```
# added to ~/.bashrc
export PATH=$PATH:/opt/android-sdk/cmdline-tools/latest/bin
export ANDROID_HOME=/opt/android-sdk/
export ANDROID_SDK_ROOT=/opt/android-sdk/
```

The TestbedOS installation then runs the following `source ~/.bashrc` to load these new variables, accept the Android SDK license agreement, and install the Android emulator.

```
source ~/.bashrc                                    # Load the new variables
yes | sdkmanager --licenses                         # Accept the licenses
sdkmanager --install "emulator" "platform-tools"    # Install the Android emulator
```

## Installation Setup

The `setup.sh` script is run during the installation process to compile and build the Rust code of different components of TestbedOS, set up the poetry virtual environments, and build the TestbedOS documentation.

```
./setup.sh
```

## Libvirt User Permissions Configuration

The installation process of TestbedOS will add the Linux user on your machine that will interface with the libvirt daemon to the libvirt QEMU configuration file and give it permission to use it.
Specifically, the installation will edit the ``/etc/libvirt/qemu.conf`` file in the following section:

    #       user = "+0"     # Super user (uid=0)
    #       user = "100"    # A user named "100" or a user with uid=100
    #
    #user = "root"

    # The group for QEMU processes run by the system instance. It can be
    # specified in a similar way to user.
    #group = "root"

The TestbedOS installation changes the `user` variable into the username of the current user, for example, if your username is `ubuntu`, and the `group` variable to `libvirt`, as follows. 

    #       user = "+0"     # Super user (uid=0)
    #       user = "100"    # A user named "100" or a user with uid=100
    #
    user = "ubuntu"

    # The group for QEMU processes run by the system instance. It can be
    # specified in a similar way to user.
    group = "libvirt"

Once this is changed, the TestbedOS installation restarts the libvirt daemon with the following command.

```
sudo systemctl restart libvirtd
```

If you have multiple users for libvirt or a locked down linux system, please see the libvirt documentation on how to manage this.
The target supported platform for TestbedOS currently assumes that you have administrator privileges and that you are the single user on your machine.

## TestbedOS Host Configuration

TestbedOS needs to know the hosts of a deployment and as part of the installation process, the `host.json` file and the `mode.json` file will be created. 

The `host.json` file contains the necessary details for the TestbedOS host and `main.json` determines if the current host of TestbedOS is the mode of the current host, which is an important detail if the host is part of a cluster. Please see the [clustering mode](clustering_mode.md) of TestbedOS for more information. 

The [Quick Installation](welcome.md#quick-installation) assumes that the current host is a singleton and is not part of a host cluster. In this case, the `main.json` contains the string `"Main"` and the `host.json` file contains the following key-value pairs.

```
{
  "ip": "10.50.0.1",
  "user": "wil",
  "identity_file": "/home/wil/.ssh/id_ed25519",
  "testbed_nic": "eth0",
  "main_interface": "wlp0s20f3",
  "is_main_host": true,
  "ovn": {
    "chassis_name": "main",
    "bridge": "br-int",
    "encap_type": "geneve",
    "encap_ip": "10.50.0.1",
    "main_ovn_remote": "unix:/usr/local/var/run/ovn/ovnsb_db.sock",
    "client_ovn_remote": null,
    "bridge_mappings": [
      [
        "public",
        "br-ex",
        "172.16.1.1/24"
      ]
    ]
  }
}
```

[TODO: Show how to configure the host if in a clustering mode]: #

Run Testbed
-----------

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


Testbed Cluster
---------------

It is possible to create a cluster of testbed hosts to increase the resource capability of your testbed.
The testbed hosts must be accessible i.e. on the same local network.
You will still need to individually configure each host's `host.json`.
You will then need to start the non main testbed hosts in client mode.
This is similar to the main mode commands, but instead you can use the following methods:

- ``sudo testbedos-server client -m <ip of main testbed host> -t <interface visible to main host on local network>```
- ``sudo -E bash -c  'cargo run -- client -m <ip of main testbed host> -t <interface visible to main host on local network>' $USER```
- If you are using the ``systemctl``` method, you must make sure the `mode.json` in ``/var/lib/testbedos/config/`` has been configured with the client configuration

Similar to the main mode, once you have successfully run the server in the client mode, you do not have to specify the client with arguments as this will be read from the `mode.json`.
Please see the testbed server |Cluster Management| for more information.

Limitations
^^^^^^^^^^^

Be aware that if you do use sudo, the files created may required elevated permissions to use so you will there-on need to continue to use sudo unless you manually edit the owner (`chown`) or permissions (`chmod`).

If you use kvm-compose up with or without sudo, if you are using cloud-init images, then be aware that the images downloaded will either go to ``/root/.kvm-compose/`` if you use sudo or ``/home/<your home folder/.kvm-compose/`` if you do not.
This means that you may end up downloading the images twice, once in each folder if you interchange the use of sudo.
