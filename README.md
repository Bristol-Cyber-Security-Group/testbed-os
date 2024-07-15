# TestbedOS

The TestbedOS combines virtualisation technologies, software defined networks and tooling to provide a platform to automate provisioning and testing of software.
The testbed also supports scaling out to multiple machines in a cluster mode to utilise more CPU, RAM and disk space for large deployments.
It offers a command line interface, textual user interface and a graphical interface to control the testbed.

Documentation can be found [here on GitHub pages](https://bristol-cyber-security-group.github.io/testbed-os/), in the source code in `docs/` or on the GUI if you are running the testbed server. 

To get started, please see the documentation and installation sections below.

## Repo Contents

The `kvm-compose` folder contains the rust code for the CLI and testbed server, in addition to examples that can be run once the network and testbed has been installed.

The `test-harness` folder contains integration tests for the tools above by creating a virtual multi testbed host environment locally.

The `util` folder contains the various supporting code for the testbed such as the privacy tooling, testbed resource monitoring and the textual user interface.

The `docs` folder contains the source for the documentation for this project that can be compiled into man pages and HTML.

The `examples` folder contains pre-made examples to showcase working testbed deployments.

## Documentation

The documentation for this project uses sphynx to compile the .rst files in the `docs` folder.
We compile man pages and html pages from the same source.
However, since it is compiled this requires you to either look at the un-compiled source, successfully run the `setup.sh` installation script (see below), or you compile this yourself.

To compile this yourself before running the provided testbed install script, you will need a python environment with `sphynx` and `sphynx-click` libraries installed.
Then you can run `sphinx-build -M man docs/ build/` for man pages or `sphinx-build -M html docs/ build/` for html.

Note the man pages currently remain in the `build/man/` folder in the root of this repo.
They can be accessed with `man -l build/man/kvm-compose.1` for example.

We look to publish the compiled HTML version in GitHub pages soon.

## Installation

This testbed was built for linux based systems, if you are on windows you will have to run this in a virtual machine.
Preferably Ubuntu, which is the target platform.

There are more detailed installation instructions in the documentation in the `docs` folder, the source can be read before it is compiled by the `setup.sh` script.
Various dependencies are required to be installed for the testbed to work before executing the `setup.sh` script.

There is the `pre-req-setup.sh` file that installed the various dependencies needed to run the `setup.sh` script.
However, note that pre-requisite script has been designed for a fresh Ubuntu install for the test-harness, so it makes assumptions on where configuration files may go.
Before running it, have a look first to see if you are happy with the changes it will make, otherwise follow the installation documentation. 

Run the `setup.sh` script in this folder (without sudo), which will compile and install the various dependencies, build the source code (plus documentation) and place the binaries and scripts into the user path.
You may be asked for the administrator password.

Note that on a successful installation, the testbed daemon will be running in the background.
If you wish to develop the testbed server you will need to turn off the daemon with `sudo systemctl stop testbedos-server.service`.

Before starting, you will need to configure the testbed server settings, please see the `testbed-config` documentation for support and examples.

To see the GUI, visit `http://localhost:3355/gui`.

To uninstall, use the `tear-down.sh` script, which requires sudo privileges.

## Running the Testbed

Please see the documentation for an outline in how to use the various tools in the testbed.
There is some setup before running your first test case, specifically the `installation` and `getting-started` documentation that requires you to configure your machine.
This is especially important if you aim to use multiple testbed hosts.

## Tests

### Unit Tests
To run the unit tests for rust, run `cargo test`.

### Integration Tests
To run the test harness, enter the `test-harness` folder and run `./run_test_harness.sh` which will prompt you for your sudo password (it needs access to libvirt).

# Contributing

We welcome GitHub issues for any bugs or suggestions.
However, at this time we are currently not taking pull requests while we work on the core testbed.
We will be reviewing any issues raised and triage internally.
Watch this space for when we open this up to contributions.

# Acknowledgements

We would like to thank Jacob Halsey https://github.com/jacob-pro/individual-project for his prototype as the basis of this project.

The term testbed OS was first used by [Professor Steven Wong](https://www.singaporetech.edu.sg/directory/faculty/steven-wong).
