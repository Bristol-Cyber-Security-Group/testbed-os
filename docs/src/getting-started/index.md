# Getting Started

Before you can start deploying test cases on the testbed, there are a few required setup steps.
Please follow the following sections in order, before using the testbed.

## Installation

You will need to install the testbed if you have not already done this.
The testbed git repo contains install scripts.
Please see the [installation documentation](../installation/index.md#testbed-os-installation) for more information. 
Ansible is also used for easy configuration and management. 
Please install it via `sudo apt install ansible -y`

## Configuring the Server

Once you have installed the testbed, you will need to configure the server.
This requires you to make sure the `host.json` in the testbed folder `/var/lib/testbedos/config/` folder is set up correctly.
Please see the [`kvm-compose-config`](../testbed-config/index.md#testbed-config) documentation on how to set this up. 
For quick configuration Ansible will take care of basic configuraiton, but more knowledge is reqiured for more custom and complex use-cases. 

The main testbed server also needs a `mode.json` file set to "Main".
If you used the `setup.sh` script, then this will have been placed for you with main as the default.

## Launching the Server

Once the testbed is configured, you can start the server.
Please see how to [run the testbed](../installation/index.md#run-testbed) on the various commands.

## Adding a client Testbed to the Testbed Cluster

If you are adding more testbed hosts to the cluster to provide more resource capability, the setup is very similar to the first testbed.
You must follow the same installation and configuration steps.
However, instead of running the testbed in main mode, it will need to be in client mode.
Please see the [testbed cluster](../installation/index.md#testbed-cluster) documentation for more information.

## Running Test Cases

Now you have a working testbed installation, you can now start deploying test cases.
We provide a CLI, a TUI, and a GUI to interact with the testbed.
Please see the [yaml examples](../examples/index.md#examples) to construct your own [`kvm-compose.yaml`](../kvm-compose/kvm-compose-yaml/index.md#kvm-compose-yaml).
