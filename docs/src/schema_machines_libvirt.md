---
title: SCHEMA-MACHINES-LIBVIRT
section: 1
---

# TestbedOS libvirt Guest Machines

libvirt is an open-source toolkit commonly used on the Linux platforms to manage virtual machines (VMs) and virtualisation stack. TestbedOS supports the three subtypes of libvirt guests based on how the guest machines are built:
- libvirt cloud images using `cloud-init` (a tool that automatically applies a configuration to a libvirt guest on its first boot),
- preconfigured and prebuilt libvirt images on the TestbedOS host's existing disk, and
- ISO libvirt guest based on an installation `.iso` image.

The `libvirt` subsection in the `kvm-compose.yaml` file offers the following fields for guest machines of libvirt VMs. 
- **`libvirt_type`**: one of the three supported subtypes of libvirt guests, the value for this field can be: 
  - **`cloud_image`**: for cloud images with further configuration using `cloud-init`,
  - **`existing_disk`**: for a libvirt guest based on a prebuilt image on a host's existing disk, and
  - **`iso-guest`**: for a libvirt guest based on an ISO installation image.
- **`cpus`**: the number of virtual cpus to assign to the libvirt guest.
- **`memory_mbs`**: the amount of memory in megabytes to be allocated to the libvirt guest.

The following snippets show examples of the relevant parts of possible libvirt guest definitions for each of the libvirt guest types. 

## libvirt Guest Type: Cloud Image

For a `cloud_image` libvirt guest, these further options are available:
- **`name`**: name of the supported `cloud-init` image,
- **`expand_gigabyte`**: the size of the disk storage from the TestbedOS to be allocated to the guest,
- **`environment`**: environment variables to be supplied to the `cloud_image` libvirt guest as a key-value store,
- **`context`**: a directory on the TestbedOS host to be mounted to the guest's directory of `/etc/nocloud/context`,
- **`setup_script`**: the script executed on the guest before deployment, specifically after `kvm-compose generate-artefacts`, and
- **`run_script`**: the script executed on the guest at the start of the deployment, specifically after `kvm-compose up`.

The following is an example snippet in the `kvm-compose.yaml` file on the relevant section for the `cloud_image` libvirt guest type. We have seen a more complete working example in [Minimal Working Example](installation.md#minimal-working-example).

``` yaml
- name: cloud-image-guest
  libvirt:
    libvirt_type:
      cloud_image:
        name: ubuntu_20_04
```

## libvirt Guest Type: Existing Disk

For an `existing_disk` libvirt guest, these further options are available:
- **`path`**: the TestbedOS host path to the pre-existing libvirt image
- **`driver_type`**: an optional field for the image format of the pre-existing image. Possible values are `raw` or `qcow2`, with `raw` as the default value, 
- **`device_type`**: an optional field for the type of storage device for the `existing_disk` libvirt guest. Possible values are `disk` or `cdrom`, with `disk` as the default value, 
- **`create_deep_copy`**: an optional field to create a deep copy of an image instead of a QCOW2 image (see [scaling](#scaling) for more information on QCOW2), and
- **`readonly`**: a `true` or `false` value that indicates if the disk image is read-only, with `false` as the default value.

The following is an example snippet in the `kvm-compose.yaml` file on the relevant section for the `existing_disk` libvirt guest type.

``` yaml
- name: existing-disk-guest
  libvirt:
    libvirt_type:
      existing_disk:
        path: /path/to/prebuilt/image.img
```

When you bring a pre-configured image to TestbedOS, the original image will not be overwritten to preserve it.
Instead, by default TestbedOS will create a linked clone of this image in the TestbedOS project `artefacts` folder.
This removes the need to create a deep copy of the image, saving time and space on disk.
The user can still defer to a deep copy with the `create_deep_copy` option (please see earlier in the section for the available options).

When the existing disk linked clone is going to be placed on a remote TestbedOS host in [the clustering mode](clustering_mode.md), TestbedOS  send a full copy to the remote host.
This is because we cannot use linked clones over the network, and because TestbedOS does not have a distributed filesystem at the moment to support this.

## libvirt Guest Type: ISO Guest

For an `iso-guest` libvirt guest, with an additional parameter `path` to refer to the installation ISO image on the TestbedOS host's disk.

``` yaml
- name: iso-guest
  libvirt:
    libvirt_type:
      iso_guest:
        path: /path/to/install/iso.iso
```

## Scaling

Scaling for libvirt guest machines is a feature provided by TestbedOS to speed up the provisioning of guests that share a common install and setup.
This allows us to scale a libvirt guest from a single machine definition and create cloned instances of the libvirt guest with less overhead. 
For example, this can be used to spawn multiple instances of a libvirt VM with the same role in the application of the deployment. 

This scaling feature utilises the linked clone functionality offered by QCOW2 that offers disk usage optimisation. 
QCOW2 which stands for QEMU Copy on Write and this means the cloned disks will only contain the difference from the original image which we call the 'golden image'. 
For example, without linked clones, if we have 3 guests that share the same install and take up 10GB of space each, then we use a total of 30GB of space.
With linked clones (3 to match the example), the golden image would take 10GB of space and the 3 guests would start with a few kilobytes in disk space used and only grow as the guest creates or edits files.
Furthermore, if the common install was bandwidth or CPU intensive, using clones we only need to do this once rather than 3 times concurrently which is likely to compete for resources and take more time.

The backing image for the clones will then become a .qcow2 file type.
Therefore the state in the backing image will be available to clones.
Some state will be overwritten by cloud-init (if using cloud-init) such as the hostname and any other post install scripts or any other cloud-init functionality.

This feature is available via the `scaling` parameter under the `libvirt` subsection in the `kvm-compose.yaml` file. Please see [Scaling](orchestration.md#scaling) for more information on what happens under the hood when provisioning the linked clones during [the TestbedOS orchestration](orchestration.md). The `scaling` parameter supports further options:

- **`count`**: the number of clone instances to be made,
- **`interfaces`**: optional, a list of SDN bridges and the connected to the bridge, where each bridge can take a list of clone IDs,
- **`shared_setups`**: optional, a list of scripts executed on the backing image before its clones are created,
- **`clone_setup`**: optional, a list of scripts executed on the clones, and
- **`clone_run`**: optional, a list of scripts executed on the clones at the end of the deployment. 

The `scaling` parameter can only be used with libvirt guests of `cloud_image` and `existing_disk`. Since the definition under the `scaling` parameter has its own `interfaces` options for the networking configuration of the clones, the `scaling` parameter cannot exist at the same time as the higher-level `interfaces` section in the `kvm-compose.yaml` file. The following snippet shows an example of the relevant parts in `kvm-compose.yaml` that uses the `scaling` parameter for a libvirt guest.

``` yaml
# 1) this snippet will create 2 clones from a cloud image definition (ommited with ...)
# 2) the clones are assigned to separate switches
# 3) the clones will be cloned from an image created with the shared script already executed
# 4) the clones will have a setup script executed on them when they are setup, they both have the same script

...
- name: scaling-guest
  libvirt:
    libvirt_type:
      cloud_image:
        ... # 1)
    scaling:
      count: 2
      interfaces: # 2)
        sw0:
          clones: [0]
          gateway: "10.0.0.1"
          ip_type: dynamic
          mac_range:
            from: "00:00:00:00:00:01"
            to: "00:00:00:00:00:01"
        sw1:
          clones: [1]
          gateway: "10.0.0.1"
          ip_type: dynamic
          mac_range:
            from: "00:00:00:00:00:02"
            to: "00:00:00:00:00:02"
      shared_setup: shared.sh # 3)
      clone_setup:
        - script: install.sh
          clones: [0, 1] # 4)
...
```

## Snapshots

TestbedOS also supports snapshots of libvirt guests where you can create, list, restore, and delete the snapshots through the [`kvm-compose` command line interface (CLI)](user_interface.md#cli).
Please refer to the [`snapshot` subcommands](user_interface_kvm_compose.md#subcommand---snapshot) for usage.
The snapshots are stored on the respective TestbedOS hosts where the libvirt guests are created on.
The CLI is merely a wrapper around the libvirt snapshot API, so if you create a snapshot outside of TestbedOS, the snapshot will still be available to be used with TestbedOS.

## Further Information on libvirt Guest Machines

The `cloud_image` libvirt guests will have full automation capabilities offered by TestbedOS, due to the ability to initialise and customise the deployment using `cloud-init` functionality. Additionally, this allows us to insert SSH keys to be able to remotely control the guest and customise further and run scripts.

Both `existing_disk` and `iso-guest` libvirt guests are limited to only be started in TestbedOS deployment and in the deployment network, and they will require manual intervention to set up. For example, if you set up SSH keys in an `existing_disk` guest before being deployed then you will be able to control this guest remotely. However, if such a libvirt guests is only running a preconfigured server in the deployment then setting up SSH keys may not be necessary as the guest is ready to be used. Note that the user may need to configure the guest's networking in the `kvm-compose.yaml` file under the `networking` section (please see [TestbedOS Guest Networking](networking.md)) such as enabling DHCP or manually assigning an IP address. This is done automatically configured if for a `cloud-image` guest.

Depending on how a libvirt guest is configured, if getty is enabled inside the guest you will be able to make a TCP TTY based connection directly to the guest. See in the state.json file after you have executed generate-artefacts to see the port number for this TTY. You will need to log in to the guest using the username and the password as configured by TestbedOS, which are `no-cloud` and `password` respectively.

### Libvirt User Permissions Configurations

[The installation process of TestbedOS](welcome.md#quick-installation) will add the Linux user on your machine that will interface with the libvirt daemon to the libvirt QEMU configuration file and give it permission to use it.
Specifically, the installation will edit the ``/etc/libvirt/qemu.conf`` file in the following section:

    #       user = "+0"     # Super user (uid=0)
    #       user = "100"    # A user named "100" or a user with uid=100
    #
    #user = "root"

    # The group for QEMU processes run by the system instance. It can be
    # specified in a similar way to user.
    #group = "root"

The TestbedOS installation changes the `user` variable into the username of the current user, for example, if your username is `ubuntu`, and the `group` variable to `libvirt`, as follows. 

    #       user = "+0"     # Super user (uid=0)
    #       user = "100"    # A user named "100" or a user with uid=100
    #
    user = "ubuntu"

    # The group for QEMU processes run by the system instance. It can be
    # specified in a similar way to user.
    group = "libvirt"

Once this is changed, the TestbedOS installation restarts the libvirt daemon with the following command.

``` bash
sudo systemctl restart libvirtd
```

If you have multiple users for libvirt or a locked down linux system, please see the libvirt documentation on how to manage this.
The target supported platform for TestbedOS currently assumes that you have administrator privileges and that you are the single user on your machine.