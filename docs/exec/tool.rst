Tool
####

These commands are run against a specific guest in the deployment.
For example:

`kvm-compose exec <guest name> tool adb ls`

This will run the `ls` command, via the `adb` protocol on the specified guest.

To install apps into the emulator, either you can directly use the emulator and use the mouse to install an app, or use the `adb` command to manually install from an .apk file from your filesystem.

adb
===

The `adb` tool is specific to `android` guests.

The `adb` command support various commands on android devices.
You can run `help` as the command argument and it will give you the list of available commands.


frida-setup
===========

The `frida-setup` tool is specific to `android` guests.

This will setup the frida server inside the android emulator.
If you are running this, you must make sure that the emulator will allow you to run commands as root.
This is possible if you disable the playstore, in the yaml set `playstore_enabled: false`.
If the emulator already had playstore enabled, you must re-provision the emulator.


test-permissions
================

The `test-permissions` tool is specific to `android` guests.

See script source for more information at `https://github.com/Bristol-Cyber-Security-Group/Frida-Tools/blob/main/permissions/log-permissions.py`.

You must specify as the argument to the command: `<packagename> <outdir>`.
Where `<packagename>` is the name of the app you want to test, for example for the Signal Messenger app you either use the short name `signal` or the full name `org.thoughtcrime.securesms`.
Where `<outdir>` is the output folder you want the output data to be placed, for example in your home folder `/home/ubuntu/test_permissions/` if your username is `ubuntu`.


test-privacy
============

The `test-privacy` tool is specific to `android` guests.

See script source for more information at `https://github.com/Bristol-Cyber-Security-Group/Frida-Tools/blob/main/test-privacy.sh`.

You must specify as the argument to the command `<package> <path-to-apk>`.
Where `<package>` is the name of the app you want to test, for example for the Signal Messenger app you either use the short name `signal` or the full name `org.thoughtcrime.securesms`.
Where `<path-to-apk>` is the path on the host to the .apk file for the app used to install on the emulator.


tls-intercept
=============

The `tls-intercept` tool is specific to `android` guests.

See script source for more information at `https://github.com/Bristol-Cyber-Security-Group/Frida-Tools/blob/main/TLS-intercept/intercept.py`.

You must specify as the argument to the command `<package-name> <out-dir>`.
Where `<package-name>` is the name of the app you want to test, for example for the Signal Messenger app you either use the short name `signal` or the full name `org.thoughtcrime.securesms`.
Where `<out-dir>` is the output folder you want the output data to be placed, for example in your home folder `/home/ubuntu/test_permissions/` if your username is `ubuntu`.

