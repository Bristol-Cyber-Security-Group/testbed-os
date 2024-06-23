shell-command
#############

This allows you to run ad-hoc commands in the guest.
Currently only libvirt guests that are type `cloud_image` are supported, as this uses `SSH` to execute the command.

You must specify the command, for example `ls .` which will just print the contents of the current directory.

