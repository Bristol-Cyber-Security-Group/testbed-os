# How to
This is a quick readme for running installing the Testbed-OS using Ansible.

TestbedOS comes in two modes:
1. Singleton - A mode in which a single Testbed-OS host is configured.
2. Cluster - A mode in which several Testbed-OS machines work in a cluster distributing the load between them.

# Requirements

- Ubuntu 22.04 LTS as a starting point
- Ansible via `apt` package manager

# How to

1. Clone down the [TestBed-OS Repository](https://github.com/Bristol-Cyber-Security-Group/testbed-os)
2. Establish your host, by default this is the same as in the machine on which the script is. However you can establish a remote host by:
    - Establishing an SSH connection with your desired remote host.
    - Modifying the inventory file to point to a remote host.
    - Ensuring the host is Ubuntu 22.04 whether remote or local.
3. Install ansible via `sudo apt install ansible -y` command or using your preferred package manager.
4. Navigate to `/testbed-os/setup/singleton` as currently it is the only mode for which there is an automated script.
5. Run the ansible script using `ansible-playbook --ask-become-pass setup.yml`
    - Please note the `--ask-become-pass` flag will prompt you for your normal `sudo` password. If you don't need one, you also do not need this flag.
6. Install all relevant components. 



# Future Work

- Modularise the ansible installation
- Cluster Installation
- Multi OS logic
