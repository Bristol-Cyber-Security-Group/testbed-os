# Welcome to TestbedOS documentation!

`TestbedOS` [^1] is a testbed for launching virtual machines on abstract topologies to support research and testing.

GitHub repository: [`https://github.com/Bristol-Cyber-Security-Group/testbed-os`](https://github.com/Bristol-Cyber-Security-Group/testbed-os).

In addition to the tooling in the testbed, we aim to deliver a variety of example test cases that showcase the capability of the testbed as a reference to build your own test cases.
The testbed uses yaml files to describe the testbed components to be deployed.
The testbed will parse the yaml file and create the artefacts in the yaml file, create the network described in the yaml and then deploy the various virtual machines on this network.
These steps are described further in the rest of the documentation.

Please see [getting-started](./getting-started/index.md#getting-started) to do the initial configuration of the testbed server before running anything on the testbed.

For guidance on the different topics:

- [kvm-compose.yaml](./kvm-compose/kvm-compose-yaml/index.md#kvm-compose-yaml) section for the schema of the yaml file
- [orchestration](./orchestration/index.md#orchestration) section for the deployment approach
- [networking](./networking/index.md#networking) section for information on the network architecture of the testbed
- [installation](./installation/index.md#testbed-os-installation), [`host.json`](./testbed-config/index.md#testbed-config) sections for initial setup before using yaml files to create testbed deployments

To get started with your first test case, see the [examples](./examples/index.md#examples) topic which will walk you through a minimal test case building up a [kvm-compose.yaml](./kvm-compose/kvm-compose-yaml/index.md#kvm-compose-yaml) file.

[^1]: The term testbed operating system was being used by Professor Steve Wong at the Singapore Institute of Technology.
