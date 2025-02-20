Push and Pull files from guests
===============================

Both these commands use the same format, one is to push files from the host into the guest.
The other is to pull files from the guest onto the host.

Push
----

Pushing files into the guests is only supported for libvirt guests.
For Docker, please use the file and folder mounts feature.
For Android, please see the ADB tooling which has built in file pushing support.

File pushing is based off using an emulated CD ROM, that is temporarily attached containing the file or folder from the host.
Once mounted in the guest, the file or folder is pushed into the user specified target location.

To be able to push to the guest, the testbed will need credentials to a user on the guest.
For cloud-init libvirt guests, these have default credentials for the `nocloud` user.
If this is your own virtual machine, you will need to supply credentials in the yaml file.
Additionally, consider the permissions needed for the target locations in addition to the permissions available to the user.
The file pushing commands do not run as root, so attempting to push a file into a location the user doesn't have permission for wont work.
There is also currently no mechanism to defer back to the user to ask for a password during the command running.

If you are using the CLI for more detailed command usage, please use `kvm-compose exec push --help`.

Architecture
************

The file pushing contains several steps to prepare the data, prepare the guest, move the data into the guest then clean up.
Firstly the file or the folder is placed into an .iso file.
This file will persist until the end of the command, placed in the `/tmp` folder.
Then a CD ROM device is attached to the guest via the SCSI controller.
A 'CD ROM' is inserted, which is the .iso file just created.
Once this has been inserted, the testbed server will then log into the guest via a pseudo terminal to mount the CD ROM.
To do this, the server needs to work out which device the CD ROM has been mounted through i.e. `/dev/sr0`.
Then this device is mounted onto the filesystem at `/mnt/filepush`.
From here, the file or folder is copied to the target location specified by the user.
Once copied, the device is unmounted from `/mnt/filepush`, then the CD ROM is detached from the guest.
The original .iso file is then deleted from `/tmp`.

Pull
----

Not yet implemented.
