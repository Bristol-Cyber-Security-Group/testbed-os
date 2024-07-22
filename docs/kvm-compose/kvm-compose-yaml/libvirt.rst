Libvirt Type
============

Each of the `libvirt_type` has restrictions in the options available.
The following are the options:

cloud_image
~~~~~~~~~~~

:name: name of the supported cloud-init image
:expand_gigabytes: the size of the disk the guest should have
:environment: some variables that the guest is supplied in a key value store
:context: a folder that should be mounted into the guest at `/etc/nocloud/context/`
:setup_script: specify the setup script that will be run on orchestration
:run_script: specify the run script that will be run at the end of orchestration

existing_disk
~~~~~~~~~~~~~

:path: path to the pre-prepared image to be used with the testbed
:driver_type: optional, `raw` or `qcow2`, defaults to `raw`
:device_type: optional, `disk` or `cdrom`, defaults to `disk`
:readonly: optional, `true` or `false`, defaults to `false`

iso_guest
~~~~~~~~~
Not yet implemented.
