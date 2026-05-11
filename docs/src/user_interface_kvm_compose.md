---
title: USER-INTERFACE-KVM-COMPOSE
section: 1
---

# `kvm-compose` Commands

We have previously seen examples of `kvm-compose` commands [Minimal Working Example](welcome.md#minimal-working-example) being used to orchestrate and interact a deployment in TestbedOS. Here we provide a more exhaustive list of the various subcommands and flags of `kvm-compose`.

## Command Structure

``` bash
kvm-compose [--input] [--project-name] [-v|--verbosity] [--no-ask] [-h|--help] [-V|--version] <SUBCOMMANDS>
```

The basic syntax is to be in a TestbedOS project folder with a `kvm-compose.yaml` file defined for the project and run ``kvm-compose generate-artefacts`` to generate config. Please refer to the different minimal working examples (MWEs): [Minimal Work Example](welcome.md#minimal-working-example), [AVD Minimal Working Example](avd.md#minimal-working-example), and [Docker Minimal Working Example](docker.md#minimal-working-example) to get started

You should not need to use `sudo` with the `kvm-compose` commands, unless you are using a resource (such as an existing disk, file to push into a guest with `cloud-init`) that your user does not have permission for. Remember to also have [the TestbedOS server](server.md) running before running the `kvm-compose` commands, of how can be found [here](server.md#starting-the-testbedos-server-for-a-deployment).

## Description

`kvm-compose` must be run in the desired project directory, where the `kvm-compose.yaml` file (or whichever filename used with `--input=`) file exists.

## Options

`--input <INPUT>`
: Configuration file [default: kvm-compose.yaml]

`--project-name <PROJECT_NAME>`
: Defaults to the current folder name

`-v, --verbosity <VERBOSITY>`
: The verbosity of the logging

`--no-ask`
: Suppress (accept) continue prompts

`-l, --local-command`
: Choose to use commands with or without the testbed server, not all work without the server

`--server-connection <SERVER_CONNECTION>`
: Specify the URL to the testbed server [default: http://localhost:3355/]

`-h, --help`
: Print help

`-V, --version`
: Print version


## Subcommands

`generate-artefacts`
: Create all artefacts for virtual devices in the current configuration

`clear-artefacts`
: Destroy all artefacts for the current configuration

`cloud-images`
: List supported cloud images

`setup-config`
: Setup kvm compose config

`deployment`
: Control deployments on the testbed server

`up`
: Deploy the test case

`down`
: Undeploy the test case

`snapshot`
: Snapshot guests

`analysis-tools`
: Analysis tools

`snapshot-testbed`
: Prepare all artefacts in deployment to be shared and used in another testbed

`help`
: Print this message or the help of the given subcommand(s)

### Subcommand - `up`

Deploy the test case

**Usage**: `kvm-compose up [OPTIONS]`

**Options**:

`-p, --provision`
: Force regenerate guest images

`-r, --rerun-scripts`
: Force rerunning use specified guest setup scripts

`-h, --help`
: Print help

### Subcommand - `down`

Undeploy the test case

**Usage**: `kvm-compose down`

**Options**:

`-h, --help`
: Print help


### Subcommand - `deployment`

Control deployments on the testbed server

**Usage**: `kvm-compose deployment <COMMAND>`

**Commands**:

`create`
: Create a deployment
  
`destroy`
: Destroy a deployment

`list`
: List all deployments

`info`
: This is the name of the deployment that is passed to the deployment commands

`help`
: Print this message or the help of the given subcommand(s)

### Subcommand - `snapshot`

Snapshot guests

**Usage**: `kvm-compose snapshot <COMMAND>`

**Commands**:

`create`
: Create a snapshot for a guest

`delete`
: Destroy a snapshot
  
`info`
: Get information about a guest and it's snapshots

`list`
: List guest snapshots

`restore`
: Restore guest from snapshot or all guests from latest snapshot, if any

`help`
: Print this message or the help of the given subcommand(s)

## Important Considerations

Be aware that if you use `sudo` with the above commands, the file(s) generated as the output may required elevated permissions to use so you will thereafter need to continue to use `sudo` unless you manually edit the owner (`chown`) or permissions (`chmod`).
If you use `kvm-compose up` with or without sudo and if you are using cloud-init images, then be aware that the images downloaded will either go to ``/root/.kvm-compose/`` if you use `sudo` or ``/home/<your home folder/.kvm-compose/`` if you do not.
This means that you may end up downloading the images twice, once in each folder if you interchange the use of sudo.