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
- **`device_type`**: an optional field for the type of storage device for the `existing_disk` libvirt guest. Possible values are `disk` or `cdrom`, with `disk` as the default value, and
- **`readonly`**: a `true` or `false` value that indicates if the disk image is read-only, with `false` as the default value.

The following is an example snippet in the `kvm-compose.yaml` file on the relevant section for the `existing_disk` libvirt guest type.

``` yaml
- name: existing-disk-guest
  libvirt:
    libvirt_type:
      existing_disk:
        path: /path/to/prebuilt/image.img
```

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

TestbedOS optionally provides the capability to scale a libvirt guest from a single machine definition and create cloned instances of the libvirt guest. For example, this can be used to spawn multiple instances of a libvirt VM with the same role in the application of the deployment. 

This is available via the `scaling` parameter under the `libvirt` subsection in the `kvm-compose.yaml` file. Please see [Scaling](orchestration.md#scaling) for more information on the scaling architecture. The `scaling` parameter supports further options:

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

## Further Information on libvirt Guest Machines

The `cloud_image` libvirt guests will have full automation capabilities offered by TestbedOS, due to the ability to initialise and customise the deployment using `cloud-init` functionality. Additionally, this allows us to insert SSH keys to be able to remotely control the guest and customise further and run scripts.

Both `existing_disk` and `iso-guest` libvirt guests are limited to only be started in TestbedOS deployment and in the deployment network, and they will require manual intervention to set up. For example, if you set up SSH keys in an `existing_ disk` guest before being deployed then you will be able to control this guest remotely. However, if such a libvirt guests is only running a preconfigured server in the deployment then setting up SSH keys may not be necessary as the guest is ready to be used. Note that the user may need to configure the guest's networking in the `kvm-compose.yaml` file under the `networking` section (please see [TestbedOS Guest Networking](networking.md)) such as enabling DHCP or manually assigning an IP address. This is done automatically configured if for a `cloud-image` guest.

Depending on how a libvirt guest is configured, if getty is enabled inside the guest you will be able to make a TCP TTY based connection directly to the guest. See in the state.json file after you have executed generate-artefacts to see the port number for this TTY. You will need to log in to the guest using the username and the password as configured by TestbedOS, which are `no-cloud` and `password` respectively.