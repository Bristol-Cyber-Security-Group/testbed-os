---
title: SCHEMA
section: 1
---

# TestbedOS Schema

The schema of TestbedOS refers to the componenets that TestbedOS offers for deployments. We have previously seen a [Minimal Working Example](welcome.md#minimal-working-example) where the components for a deployment described in the `kvm-compose.yaml` file consisting of a single Ubuntu 20.04 LTS virtual machine (VM) with a software-defined switch connected to the VM. 

The `kvm-compose.yaml` file is a user-defined description of the schema of the deployment, and it is a high-level document that abstracts away the details of the inner workings of TestbedOS from the user. The following sections describe the schema of TestbedOS further that can be included in the `kvm-compose.yaml` file.

The `kvm-compose.yaml` file (the schema) provide the following four available sections, and we have illustrated the use of two of them in [Minimal Working Example](welcome.md#minimal-working-example).

- **Machines (`machines`)** A list of definitions for the guests in a deployment that specifies the guest types and further specifications for each guest machine such as the name of the guest machine, memory capacity, storage capacity, etc. Please see [TestbedOS Guest Machines](machines.md) for more details.

- **Network (`network`)** A list of networking entities available for the Software-Defined Network (SDN) in a deployment. This includes a switch and the SDN topology for the deployment. Please see [TestbedOS Guest Networking](schema_networking.md) for more details.

<!-- - **Tooling (`tooling`)** A list of specialised tools to interact with the deployment for specific purposes. Please see [TestbedOS Tooling](tooling.md) for more details.

- **Deployment Options (`deployment_options`)** A list of options to customise the deployment. Please see [TestbedOS Deployment Options](deployment_options.md). -->