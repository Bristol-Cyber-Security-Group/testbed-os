---
title: USER-INTERFACE-EXEC-SHELL-COMMANDS
section: 1
---

# Shell Commands

We provide the `shell-command` subcommand under `kvm-compose exec` to run ad-hoc commands on the shell of a deployed guest. The structure of the `shell-command` subcommand is as follows.

``` bash
kvm-compose exec <guest name> shell-command [[-t|--timeout-ms]] [[-s|--suppress-logging]] [[-h|--help]] <shell command> 
```

## Parameters

`-t, --timeout-ms <TIMEOUT_MS>`
: (Optional) Timeout for the shell command to run. Defaults to 5 seconds (default: 5000)

`-s, --suppress-logging`
: (Optional) Suppress all logging during command execution.

`-h, --help`
: (Optional) Print the help message

`<shell command>`
: The shell commands for the guest to execute

## Examples

```
kvm-compose exec server shell-command echo "Hello world"
```

## Internals

The command are executed via a pseudo terminal on the guest, which means there is a requirement for credentials. For `cloud-init` libvirt guests, we have default credentials already configured (username and password: `nocloud`) but if you have your own virtual machine then you must supply them in the `kvm-compose.yaml` file as follows (trimmed with elipses for brevity).

``` yaml
- name: server
  ...
  libvirt:
    cpus: 2
    memory_mb: 2048
    username: ubuntu
    password: password
    libvirt_type:
      ...
```

The `username` and `password` fields exist under the libvirt indentation level but above the `libvirt_type`. If the guest has passwordless login, then just leave an empty string but the element must be present.

The command running does not currently have a mechanism to elevate privileges i.e. defer to the user for a password. For example, if you attempt to run a command and it requires the root password that is not the same as the password provided.

Note: if you want to send some command that uses special characters i.e. `$` or `~`, then make sure to wrap these or the whole command in quotes. Otherwise your current shell will expand them before the command is captured by TestbedOS code.
