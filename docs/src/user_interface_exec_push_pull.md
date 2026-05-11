---
title: USER-INTERFACE-EXEC-PUSH-PULL
section: 1
---

# File Push and Pull

Once a deployment is up and running via `kvm-compose up`, you can *push* (upload) a file to a specific guest via the `push` subcommand or *pull* (download) a file from a specific guest via the `pull` subcommand in the deployment. 

The subcommands are only available for libvirt guests. For Docker, please use the file and folder mounts feature. For Android, please see the ADB tooling which has built in file pushing support. To be able to push to the guest, TestbedOS will need credentials to a user on the guest. For cloud-init libvirt guests, these have the default credentials of  `nocloud` for both the username and password. If this is your own virtual machine, you will need to supply credentials in the `kvm-compose.yaml` file, as shown in the following snippet (trimmed with ellipses for brevity). 

``` yaml
- name: client1
  ...
  libvirt:
    cpus: 2
    memory_mb: 2048
    username: ubuntu
    password: password
    libvirt_type:
      ...
```

Additionally, consider the permissions needed for the target locations in addition to the permissions available to the user. The file pushing commands do not run as root, so attempting to push a file into a location the user does not have the permission for will not work. There is also currently no mechanism to defer back to the user to ask for a password during the command running.

The structure of the `push` and `pull` subcommands are as follows:

``` bash
kvm-compose exec <guest name> [push|pull] [[-h|--help]] [-s|--source-path] <source path> [-t|--target-path] <target path> 
```

## Parameters

`-s,--source-path <source path>`
: Absolute path of the source file or folder to be pushed or pulled

`-t, --target-path <target path>`
: Absolute path of the target file or folder to be pushed or pulled

`-h, --help`
: (Optional) Prints the help message

Note: for the time being `<source path>` and `<target path>` have to be absolute paths on the host or guest machine. If both values are supplied in order on the command, then the flags `--source-path` and `--target-path` are not needed. For example,

## Examples

``` bash
kvm-compose exec server push /home/debian/sample-project/test.txt /home/guest/test.txt
```

``` bash
kvm-compose exec server pull /home/guest/test.txt /home/debian/sample-project/test.txt
```

## Internals

File pushing is based off using an emulated CD-ROM, that is temporarily attached containing the file or folder from the host. Once mounted in the guest, the file or folder is pushed into the user specified target location. The file pushing contains several steps to prepare the data, prepare the guest, move the data into the guest then clean up. Firstly the file or the folder is placed into an `.iso` file. This file will persist until the end of the command, placed in the `/tmp` folder. Then a CD-ROM device is attached to the guest via the SCSI controller. A 'CD-ROM' is inserted, which is the `.iso` file just created. Once this has been inserted, the TestbedOS server will then log into the guest via a pseudo terminal to mount the CD-ROM. To do this, the server needs to work out which device the CD-ROM has been mounted through, i.e., `/dev/sr0`. Then this device is mounted onto the filesystem at `/mnt/filepush`. From here, the file or folder is copied to the target location specified by the user. Once copied, the device is unmounted from `/mnt/filepush`, then the CD-ROM is detached from the guest. The original `.iso` file is then deleted from `/tmp`.