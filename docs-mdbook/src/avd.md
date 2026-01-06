# TestbedOS Android Virtual Device

TestbedOS additionally provides the capability of spawning Android emulators for the research and experimentation on the Android platform implemented through the [Android Virtual Device feature](https://developer.android.com/studio/run/emulator-commandline#listing-filedir). This is available through the `avd` field in the `kvm-compose.yaml` file. The following lists the available options for the `avd` section:

- **`android_api_version`**:	the API version of the Android operating system supported,
- **`playstore_enabled`**:	whether the Google Play Store will be installed or not (Play Store enables various OS protections preventing tools that need root access),
- **`setup_script`**:	an arbritrary script that will be executed one the emulator is ready, and
- **`run_script`**:	an arbritary script that will be executed one the emulator is turned on (after setup_script, if defined).

## Minimal Working Example

This section demonstrates on running a Minimal Working Example (MWE) for an Android guest. The steps are similar to deploying a libvirt guest at the start of the documentation in [Minimal Working Example](welcome.md#minimal-working-example) and the following is the content for the `kvm-compose.yaml` file for deploying a MWE on an Android guest.

``` yaml
machines:
  
  - name: phone
    network:
      - switch: sw0
        gateway: 10.0.0.1
        mac: "00:00:00:00:00:01"
        ip: "dynamic"
    android:
      avd:
        android_api_version: 28
        playstore_enabled: false

network:
  ovn:
    switches:

      sw0:
        subnet: "10.0.0.0/24"

      public:
        subnet: "172.16.1.0/24"
        ports:
          - name: ls-public
            localnet:
              network_name: public

    routers:

      lr0:
        ports:

          - name: lr0-sw0
            mac: "00:00:00:00:ff:01"
            gateway_ip: "10.0.0.1/24"
            switch: sw0

          - name: lr0-public
            mac: "00:00:20:20:12:13"
            gateway_ip: "172.16.1.200/24"
            switch: public
            set_gateway_chassis: main

        static_routes:
          - prefix: "0.0.0.0/0"
            nexthop: "172.16.1.1"

        nat:
          - nat_type: snat
            external_ip: "172.16.1.200"
            logical_ip: "10.0.0.0/16"

        dhcp:
          - switch: sw0
            exclude_ips:
              from: "10.0.0.1"
              to: "10.0.0.20"
```

The TestbedOS server for starting a deployment with an Android guest has to be started with the following command. This is a temporary fix as we work towards having the TestbedOS started the same way regardless of the guest type.

``` shell
sudo /usr/local/bin/testbedos-server
```

After `kvm-compose generate-artefacts` and `kvm-compose up` on this `kvm-compose.yaml` file, you should see the GUI for the Android emulator on the screen. Please feel free to have a go at interacting with it.

## Interacting with the Android Guest via TestbedOS

TestbedOS provides various ways and tools to interact with the Android guest other than the default GUI interaction on the Android emulator. For example, you can also interface with the Android Debug Bridge (ADB) shell during a deployment. With the Android guest in the [MWE from the previous section](#minimal-working-example) up and running (i.e., after `kvm-compose up`), run the following command on a terminal to see the ADB shell in action through TestbedOS.

``` shell
kvm-compose exec phone tool adb shell echo "hello"
```

This will generate the following output indicating the success of interaction.

```
... 
kvm_compose_lib::orchestration::websocket: hello
...
```

For a more complete list of the available options to interact with the Android guest, please refer to [CLI](cli.md).

## AVD Emulator Networking 

The AVD emulator has some peculiarities in how it provisions networking for the emulated device, see https://developer.android.com/studio/run/emulator-networking, specifically on how it binds the networking to the local (TestbedOS) host. 
The local host networking requires some isolation to be able to integrate the emulator inside TestbedOS's network. 
This means we deploy the emulator inside a network namespace (`netns`), which is connected to the designated TestbedOS network bridge through virtual Ethernet interfaces (`veth`s). 

It is possible to utilise ADB to control the Android guest remotely, however note that due to the emulator being in the namespace as mentioned above, the ADB server will also run inside the namespace and accept connections through the namespace's TestbedOS host. This only means any ADB command must be executed through the namespace command, for example:

``` shell
sudo ip netns exec android-test adb -s emulator-5554 shell
```

where `android-test` is the name of the namespace.

The example shown in [the previous section](#interacting-with-the-android-guest-via-testbedos) is the mechanism that TestbedOS provides for users to avoid having to manually work out the network namespaces. Note that this would have to be run from the root of the TestbedOS project where the `kvm-compose.yaml` file lives. Also, since the command runs from the TestbedOS server, you cannot open an interactive shell to the guest, only one-time commands.

## Existing Preconfigured AVD Images

The AVD guests can either be created from scratch on TestbedOS and deployed on demand on a deployment, or you are also able to bring a pre-configured image to the testbed. 
Due to how we deploy and control guests through the TestbedOS server, the AVD configuration will be under the root user. 
This is a critical detail to allow the ADB connection to authenticate using the key that is found in the $USER directory, which in this case will be under root. If the emulator is not under the root user, say you are bringing in a pre-configured emulator you may need to use the command as following:

``` shell
sudo -E bash -c  'ip netns exec android-test adb -s emulator-5554 shell' $USER
```

where `$USER` can be left as is, if the emulator is under your user account or replace it with the user's account.

## Android Guest Interaction with Other Guest Types

Currently, the emulator can act as a client to the other guests in the deployment network, i.e., the applications in the emulator can make requests to web servers on other (non-AVD) guests in the testbed network. 
However, we currently don't support the emulator acting as a server to other guests but this may be supported in the future. 
You may have some success by utilising ADB to enable port forwards to the device or by editing the deploy script and adding QEMU options.