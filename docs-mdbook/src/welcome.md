# Welcome to TestbedOS Documentation!

## What is TestbedOS?

TestbedOS[^1] is a platform for launching a virtualised testbed on abstract topologies to support academic research and experimentation. TestbedOS provides researchers the ability to quickly deploy an experimental setup and easily share their research artefacts on a single platform. Specifically, TestbedOS combines existing virtualisation technologies and automatically deploys various virtual machines and their networking configuration based on a user-defined specification without manual user setup. TestbedOS additionally allows users to interact with the deployed virtual machines once they are up and running, and TestbedOS provides additional tooling based on the specific applications that the user might require. Currently, TestbedOS is only supported on Debian 12, Ubuntu 20.04 LTS, and Ubuntu 22.04 LTS.

## Quick Installation

The TestbedOS GitHub repository can be found at [`https://github.com/Bristol-Cyber-Security-Group/testbed-os`](https://github.com/Bristol-Cyber-Security-Group/testbed-os>) and we have packaged the installation into an Ansible playbook. The target supported platform for TestbedOS currently assumes that you have administrator privileges and that you are the single user on your machine. We will build and install TestbedOS based on this codebase using the following steps.

1. On your terminal, install the prerequisites for the installation if they are not already available on your machine.
    
    ```
    sudo apt install git
    sudo apt install ansible -y
    ```

2. Clone the TestbedOS repository from [`https://github.com/Bristol-Cyber-Security-Group/testbed-os`](https://github.com/Bristol-Cyber-Security-Group/testbed-os>) onto your local system.

    ```
    git clone https://github.com/Bristol-Cyber-Security-Group/testbed-os.git
    ```

3. Navigate to the directory that contains the Ansible playbook `setup.yml` and run it to start the installation process. 

    ```
    cd testbed-os/setup/singleton
    ansible-playbook --ask-become-pass setup.yml
    ```

    - Note: the `--ask-become-pass` flag will prompt you for your normal `sudo` password.

4. Once the installation is complete, run the following command to verify the installation of TestbedOS.

    ```
    kvm-compose --version
    ```
    The output should be the following.
    ```
    kvm-compose-schemas 1.0
    ```

## Minimal Working Example

We will show a minimal working example (MWE) of a testbed deployment or a _test case_ and the most common commands for the lifecycle of creating, running, and stopping a deployment. The specification for this MWE is in the form of a YAML file called `kvm-compose.yaml`, which is also the specification for any other deployments on TestbedOS. We assume that TestbedOS has been installed as per the instructions in the previous section. 

The `kvm-compose.yaml` specification file for this MWE has been included in the TestbedOS repository from [`https://github.com/Bristol-Cyber-Security-Group/testbed-os`](https://github.com/Bristol-Cyber-Security-Group/testbed-os>). The `kvm-compose.yaml` file for this MWE contains the following:

```
machines:
  - name: server
    network:
      - switch: sw0
        gateway: 10.0.0.1
        mac: "00:00:00:00:00:01"
        ip: "10.0.0.10"
    libvirt:
      cpus: 1
      memory_mb: 1024
      libvirt_type:
        cloud_image:
          name: ubuntu_20_04
          expand_gigabytes: 2

network:
  ovn:
    switches:
      sw0:
        subnet: "10.0.0.0/24"
```

This MWE deploys a virtual machine (VM) assigned the identifier `server` in the deployment with the specification of one core CPU, 1024 MB memory, a maximum of 2 GB of virtual disk size for QCOW2 format (more details on this in ), an IP address of `10.0.0.10` and a MAC address of `00:00:00:00:00:01`, and running Ubuntu 20.04 LTS. The networking setup contains a switch with the identifier `sw0` that connects to the `server` VM acting as the VM's gateway, and serves the subnet `10.0.0.0/24`.

1. On your terminal, navigate to `/testbedos/examples/mwe` where the `kvm-compose.yaml` file for the MWE lives. Alternatively, if you prefer you can also create a new project directory outside of the cloned TestbedOS repository, which might be the more preferable case for your own custom TestbedOS deployments. For this, we will name the project `example`.

    ```
    cd testbedos/examples/mwe
    ```
    or
    ```
    mkdir example
    cp testbedos/examples/mwe/kvm-compose.yaml example/kvm-compose.yaml
    cd example
    ```

2. Before any TestbedOS command can be run, the TestbedOS server should be running with the following command.

    ```
    sudo systemctl start testbedos-server
    ```

3. TestbedOS will provision the required artefacts for the deployment from the following command. This will generate the `artefacts` folder and a state file for the deployment called `example-state.json`.

    ```
    kvm-compose generate-artefacts
    ```

4. The following command brings the deployment, i.e., the virtual machines and the networking component up and running.

    ```
    kvm-compose up
    ```

5. Once the deployment is up and running, you can interact with the deployment based on the type of guests and networking specified. TestbedOS provides many ways of interacting with a deployment and an example of this can be the following command, which uses the shell of the deployed `server` VM to execute the `ls` bash command. 

    ```
    kvm-compose exec server shell-command echo "hello world"
    ```

    This specifies the subcommand `exec` to execute a `shell-command` operation on the guest `server`. The output will show the following to indicate a successful execution.
    ```
    # Some TestbedOS messages

    2025-12-16T10:45:24.824438Z  INFO kvm_compose_lib::orchestration::websocket: Command output:
    hello world
    
    # Followed by more TestbedOS messages
    ```

6. Once you are done with the deployment, simply run the following command to bring the deployment down.

    ```
    kvm-compose down
    ```

7. The deployment will maintain the state from the previous run. However, if this should not be the case, you can clear the state from the previous deployment run by running the following.

    ```
    kvm-compose clear-artefacts
    ```
This will delete the artefacts folder and that means the next time the deployment is run, `kvm-compose generate-artefacts` should be run before `kvm-compose up`.

## Uninstallation

You should tear down any test cases before uninstalling the testbed, see :ref:`orchestration <orchestration/index:orchestration>` for more information on how to tear down a test case.

If you want to the testbed (assuming all vms and networking components have been destroyed), you can use the ``tear-down.sh`` script in the root of the testbed-or repo to remove the kvm-compose binary and python code+environments originally installed via setup.sh.

## Further Details and Documentation

For more details on what TestbedOS offers and topics on how TestbedOS works under the hood, please refer to the following documentation.

- [TestbedOS User Interface](user_interface.md) for the different user interfaces that TestbedOS provides, including the CLI that we have seen in the [MWE](#minimal-working-example) . 
- [TestbedOS Dependencies](dependencies.md) for more details on what dependencies are being installed and the installation setup on your machine.
- |kvm-compose.yaml| for the complete schema of the yaml file
- |orchestration| section for the deployment approach
- |networking| section for information on the network architecture of the testbed
- |installation|, |host.json| sections for initial setup before using yaml files to create testbed deployments

To get started with your first test case, see the |examples| topic which will walk you through a minimal test case building up a |kvm-compose.yaml| file.


[^1]: The term testbed operating system was being used by Professor Steve Wong at the Singapore Institute of Technology.
