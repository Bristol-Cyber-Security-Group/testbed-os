---
title: INTERFACES-EXEC-TOOL
section: 1
---

# Tool

TestbedOS additionally offers tools to run against a guest in the deployment environment and this is done through the `tool` subcommand of `kvm-compose`. The structure of the `tool` subcommand is as follows.

``` bash
kvm-compose exec <guest name> tool <tool name> <tool options> [-h|--help]
```

Like other `kvm-compose` subcommands, the `tool` subcommand is to be run after the guest has been deployed, i.e., after `kvm-compose up`. Please see the [Minimal Working Example (MWE)](welcome.md) on the `kvm-compose` commands to deploy a guest.

We additionally provide tools that are not necessarily run against a specific guest but the deployment environment through [`kvm-compose tool` subcommand](user_interface_tool.md).

## `adb`

The `adb` tool can only be used on Android guests or [AVD guests](avd.md) to support running the Android Debugging Bridge (ADB) commands on the guests and the ADB support various commands on Android devices in general. The command structure for the `adb` tool is as follows:

``` bash
kvm-compose exec <guest name> tool adb <adb command>
```

The `<adb commands>` argument refers to the trailing commands that the ADB will execute separated by spaces. You can run `help` as the ADB command argument and it will give you the list of available ADB commands. For example, the following command runs the `ls` command via the ADB on the `phone` AVD guest. 

``` bash
kvm-compose exec phone tool adb ls .
```

## `frida-setup`

The `frida-setup` tool can only be used on Android guests or [AVD guests](avd.md) for the installation and the initial setup of the Frida server on the Android emulators. This is to facilitate further Frida operations on the Android guest, e.g., the later [`test-permissions`](#test-permissions), [`tls-intercept`](#tls-intercept), and [`test-privacy`](#test-privacy) tools. For example, the following command installs and sets up the Frida server on the `phone` AVD guest.

`kvm-compose exec phone tool frida-setup`

In order for this tool to work, the ADB daemon (`adbd`) has to be run as root, for which this command will try to do. Please note that this requires the Android emulator to not have Google Play Store services as per Google's restrictions.

In TestbedOS, you can disable the Play Store services in the `kvm-compose.yaml` file by defining the key-value `playstore_enabled: false`.
If the AVD guest had been previously deployed with `playstore_enabled: true`, you must re-deploy the emulator.
Please see [AVD guests](avd.md) for a more detailed overview on the `kvm-compose.yaml` definition for the AVD guests.

## `install-apk`

The `install-apk` tool can only be used on Android guests or [AVD guests](avd.md) to provide a shortcut to install an APK file onto an Android guest. This tool installs an APK file onto the specified Android guest via the `kvm-compose` interface directly without you having to manually use the [`adb`](#adb) tool although this tool uses `adb` under the hood.

The command structure for the `install-apk` tool is as follows.

``` bash
kvm-compose exec <guest name> tool install-apk <full path apk>
```

You must specify the `<full path apk>` argument to the tool where `<full path apk>` refers the absolute path to the APK file on the TestbedOS host. For example, 

``` bash
kvm-compose exec phone tool install-apk /home/debian/sample-project/test.apk
```

## `test-permissions`

The `test-permissions` tool can only be used on Android guests or [AVD guests](avd.md) to test the permissions granted for a specific Android application that has been installed on the Android guest. This tool outputs a `permissions.txt` file which logs all of the permission statuses and a summary at the end of all permissions which are granted to the application. Please see the script source [`log-permissions.py`](https://github.com/Bristol-Cyber-Security-Group/Frida-Tools/blob/main/permissions/log-permissions.py) for more information.

The command structure for the the tool is as follows.

``` bash
kvm-compose exec <guest name> tool test-permissions <package name> <output path>
```

You must specify as the argument to the command: `<package name>` and `<output path>` where `<package name>` is the name of the  app you want to test, and `<output path>` is the destination folder you want the output data, i.e., the `permissions.txt` file, to be placed. The value for `<package name>` can be found using the ADB shell command `adb shell pm list packages` and the equivalent for TestbedOS via `kvm-compose` is `kvm-compose exec <guest name> tool adb shell pm list packages`. 

For example,

``` bash
kvm-compose exec phone tool test-permissions org.thoughtcrime.securesms /home/debian/test_perimissions/
```


## `tls-intercept`

The `tls-intercept` tool can only be used on Android guests or [AVD guests](avd.md) to reveal the underlying HTTP messages sent in the TLS tunnel of an Android application installed on a guest. This tool outputs two CSV files which log the HTTP messages that are read and written to and fro the TLS tunnel via either the Java's Conscrypt library or the lower-level `SSLRead` and `SSLWrite` methods. Please see the script source [`intercept.py`](https://github.com/Bristol-Cyber-Security-Group/Frida-Tools/blob/main/TLS-intercept/intercept.py) for more information.

The command structure for the the tool is as follows.

``` bash
kvm-compose exec <guest name> tool tls-intercept <package name> <output path>
```

You must specify as the argument to the command: `<package name>` and `<output path>` where `<package name>` is the name of the Android application you want to test, and `<output path>` is the destination folder you want the output data, i.e., the two CSV files to be placed. The value for `<package name>` can be found using the ADB shell command `adb shell pm list packages` and the equivalent for TestbedOS via `kvm-compose` is `kvm-compose exec <guest name> tool adb shell pm list packages`. 

For example,
 
``` bash
kvm-compose exec phone tool tls-intercept org.thoughtcrime.securesms /home/debian/test_perimissions/
```


## `test-privacy`

The `test-privacy` tool can only be used on Android guests or [AVD guests](avd.md) to perform a suite of privacy analysis tools on an Android application installed on a guest. The privacy analysis toolsuite includes the above [`test-permissions`](#test-permissions) and [`tls-intercept`](#tls-intercept) tools. This tool outputs a single PDF file as a report for the application privacy analysis and the other files as the output of each individual privacy tool. Please see the script source [`test-privacy.sh`](https://github.com/Bristol-Cyber-Security-Group/Frida-Tools/blob/main/test-privacy.sh) for a more complete information.

The command structure for the the tool is as follows.

``` bash
kvm-compose exec <guest name> tool test-privacy <package name> <apk path>
```

You must specify as the argument to the command: `<package name>` and `<full path apk>` where `<package name>` is the name of the Android application you want to test, and `<full path apk>` is the absolute path to the APK file on the TestbedOS host. The value for `<package name>` can be found using the ADB shell command `adb shell pm list packages` and the equivalent for TestbedOS via `kvm-compose` is `kvm-compose exec <guest name> tool adb shell pm list packages`. The output files are placed under the `/logs/<package name>/` folder.

For example,
 
``` bash
kvm-compose exec phone tool test-privacy org.thoughtcrime.securesms /home/debian/test_permissions/
```