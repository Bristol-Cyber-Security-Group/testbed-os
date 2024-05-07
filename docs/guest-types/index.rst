Guest Types
===========

The testbed supports three main types of guests in the yaml file:

- Libvirt virtual machines
- Docker containers
- Android Virtual Devices (AVD)

These guests can all be defined and assigned to the testbed network.
While they all support being deployed in the testbed, only the cloud-init based Libvirt guests support automation at this time.

We have introduced Docker and AVD as guests to remove the need to deploy these inside a Libvirt virtual machine.
This removes the overhead of a whole virtual machine in terms of performance and disk usage.
However, if the use case requires it or there is some functionality that is not supported in our implementation, you are free to deploy Docker and AVD instances inside Libvirt virtual machines.

The following subsections describe further the capabilities of the guest types, implementation details and limitations.

Libvirt
-------

The Libvirt virtual machines are split into three sub-types based on their mode of installation:

- cloud image (using cloud-init)
- existing disk (bring a preconfigured image)
- iso guest (bring an installation .iso image)

The cloud image based guests will have full automation possible, due to the ability to seed the deployment using cloud-init.
We are able to customise the deployment using the cloud-init functionality.
Additionally, this allows us to insert SSH keys to be able to remotely control the guest and customise further and run scripts etc.

Both existing disk and iso guests are limited to just being able to be started in the testbed and in the network, but will require manual intervention to set up.
For example, if the user sets up the SSH keys in an existing disk guest before deploying in the testbed then they will be able to control this guest remotely.
However, such guests may only be running a webserver for experimentation in the testbed for example that is preconfigured so needing to set up SSH keys may not be necessary as it is ready to be used.
Note that the user may need to configure networking such as enabling DHCP or manually assigning an IP address, this is done automatically for you if using cloud image guest type.

Depending on how the guest is configured, if getty is enabled inside the guest you will be able to make a TCP TTY based connection directly to the guest.
See in the state.json file after you have executed `generate-artefacts` to see the port number for this TTY.
You will need to log in to the guest using the username and password.

Docker
------

The docker guests have little configuration needed, the testbed will provision the required networking for the running container.
Note that since the container is assigned to the testbed network, the networking works slightly differently to how you expect if you were using just docker or docker-compose.
We utilise `ovs-docker` to assign the container to the specified OVS bridge, which creates an interface `eth0` in the container and assigns an IP address.

You can use the various built in Docker commands in the CLI to inspect or control the container.
Make sure you use the name of the container the testbed assigns, which will be `<project name>-<machine name>`.
For example, if the project is called `analysis` and the docker machine name in the yaml file is called `webserver`, then the container name will be `analysis-webserver`.
If you have used scaling in the machine definition, you will need to append the id of the scaled container.
For example, if you have scaling set to 3 (where the ids start at 0) and you want to run a docker command on the second id (1) then you would use `analysis-webserver-1` as the container name.

Android Virtual Device
----------------------

The AVD guests can either be created and deployed on demand or you are able to bring a pre-configured image to the testbed.

The AVD emulator has some peculiarities in how it provisions networking for the emulated device, see https://developer.android.com/studio/run/emulator-networking
Specifically how it binds the networking to the localhost, it required some isolation to be able to integrate the emulator inside the testbed network.
This means we deploy the emulator inside a network namespace, which is connected to the designated testbed network bridge through veths.
Currently, the emulator can act as a client to the other guests in the network i.e. the applications in the emulator can make requests to web servers on other (non-AVD) guests in the testbed network.
However, we currently don't support the emulator acting as a server to other guests - this may be supported in the future.
You may have some success by utilising ADB to enable port forwards to the device or by editing the deploy script and adding Qemu options.

It is possible to utilise ADB to control the android guest remotely, however note that due to the emulator being in the namespace as mentioned above, the ADB server will also run inside the namespace and accept connections through the namespace's localhost.
This only means any ADB commands must be executed using the namespace command, for example:

``sudo ip netns exec android-test adb -s emulator-5554 shell``

where `android-test` is the namespace name.

However you can also use the built in `exec` commands to run commands via ADB but note that since the command runs from the testbed server, you cannot open a shell to the guest - only one time commands.

Additionally, due to how we deploy and control guests through the testbed-os server, the AVD configuration will be under the root user.
This is a critical detail to allow the ADB connection to authenticate using the key that is found in the `$USER` directory, which in this case will be under root.
If the emulator is not under the root user, say you are bringing in a pre-configured emulator you may need to use the command as following:

``sudo -E bash -c  'ip netns exec android-test adb -s emulator-5554 shell' $USER``

where `$USER` can be left as is, if the emulator is under your user account or replace it with the user's account.
