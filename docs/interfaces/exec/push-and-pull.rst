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

If you are using the CLI for more detailed command usage, please use `kvm-compose exec push --help`.

Pull
----

Not yet implemented.
