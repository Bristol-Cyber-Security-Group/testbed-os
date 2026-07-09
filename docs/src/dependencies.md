---
title: DEPENDENCIES
section: 1
---

# TestbedOS Dependencies

There are various dependencies needed to be installed with some based configuration needed to get the testbed ready to deploy test cases.
This installation guide will outline each component needed. 

## General Dependencies

[Rust](https://rustup.rs/)
: The Rust programming language. Default configuration, when prompted, is fine.

<!-- [Poetry](https://python-poetry.org/docs/#installation)
: Please make sure you have python version `3.10` and above, consider using pyenv to manage python installs (see [Poetry Documentation](https://python-poetry.org/docs/managing-environments/)).
Also make sure you have `pip3` installed for this python version, for Ubuntu install `python3-pip`.

[PyEnv](https://github.com/pyenv/pyenv)
: Python version manager, asks you to manually add it to your shell profile once installed.

Poetry has been used to manage the python virtual environments for TestbedOS.
While it is possible to use others, you will need to manually replace the use of `poetry run` for example with your own virtual environment management.
The use of `poetry run` for ad-hoc use of the python environment to remove the need to load the virtual environment in the current session. -->

## Runtime Dependencies

**`genisoimage`**
: Used to create .iso files for cloud-image guest startup configuration.

**`virt-manager`** 
: A graphical viewer for libvirt guests. 
Virtual Machine manager is a useful GUI for libvirt, which allows you to inspect the network and guest configuration.
It also allows you to open a graphical window to the guest which will either be a terminal or the graphical desktop if installed.
Consider using `sudo virsh console <project name with the guest name>`, e.g., `sudo virsh console mwe-server` for the [Minimal Working Example](welcome.md#minimal-working-example), to open a TTY to the guest as the graphical window may not support copy paste etc without guest tools installed.

## Networking Dependencies

TestbedOS uses Open Virtual Network (OVN) and Open Virtual Switch (OVS) as the underlying Software-Defined Networking (SDN) provider for the networking capabilities of the deployments.
As below, we build OVN and OVS from source and we use the OVS submodule so that we match the OVN to OVS version. 

``` bash
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

## Docker

Docker is used to allow the user to deploy containers on TestbedOS, the installation steps for Docker are the same as in [Install Docker Desktop on Linux](https://docs.docker.com/desktop/install/linux-install/).
If you are already using Docker Desktop, the TestbedOS installation reuses the existing Docker installation.

## Android Emulator

Android Virtual Device is used to deploy Android emulators on TestbedOS, we install the following CLI tools.
The command line tools must also be downloaded from Google at [Android Studio - Command line tools only](https://developer.android.com/studio#command-line-tools-only).

``` bash
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

``` bash
# added to ~/.bashrc
export PATH=$PATH:/opt/android-sdk/cmdline-tools/latest/bin
export ANDROID_HOME=/opt/android-sdk/
export ANDROID_SDK_ROOT=/opt/android-sdk/
```

The TestbedOS installation then runs the following `source ~/.bashrc` to load these new variables, accept the Android SDK license agreement, and install the Android emulator.

``` bash
source ~/.bashrc                                    # Load the new variables
yes | sdkmanager --licenses                         # Accept the licenses
sdkmanager --install "emulator" "platform-tools"    # Install the Android emulator
```

## Dependencies Installation Setup

[The Ansible installation](welcome.md#quick-installation) compiles and builds the Rust code of different components of TestbedOS. It also generates the TestbedOS documentation.
