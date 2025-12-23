# TestbedOS Guest Machines

`TestbedOS` currently provides three types of guest machines for a deployment: libvirt virtual machines (VMs), Docker containers, and Android Virtual Devices (AVDs). 
These guests can all be defined and assigned to the TestbedOS network in the deployment. 
We have introduced Docker and AVD as guests to remove the need to deploy these inside a libvirt virtual machine. 
This removes the overhead of a whole virtual machine in terms of performance and disk usage, and allows all guests to be treated as `first-class` in TestbedOS.
However, if the use case requires it or there is some functionality that is not supported in our implementation, you are free to deploy Docker and AVD instances inside Libvirt virtual machines. 
The following describes further how to deploy the guests in TestbedOS, specifically in the `kvm-compose.yaml` file, which would already be familiar to you if you have followed the [Minimal Working Example](welcome.md#minimal-working-example).

The `machine` section of the `kvm-compose.yaml` file allows you to define one or more instances of guest machines for a deployment. The definitions depend on the type of guest machines for a deployment, which further includes the specific configuration for the definition of each guest. In the `machine` section of the `kvm-compose.yaml` file the following JSON keys can be specified.

- **`name`**: the unique name of the machine.
- **`network`**: an optional list of network interfaces for the guest machine in the Software-Defined Network (SDN) of a deployment. The list of values usually include a switch (`switch`) in the SDN network of the deployment, an IP address for the gateway for the guest (`gateway`), a MAC address for the guest machine (`mac`), and an IP address (`ip`) to be assigned to the guest machine itself in the SDN of the deployment.
- **`machine_type`**: one of the supported guest types. The values are `libvirt` for a libvirt VM, `docker` for a docker container, and `avd` for an AVD.

The snippets below shows three examples of the above fields in the `kvm-compose.yaml` file (separate). For a libvirt VM guest,

``` yaml
...
    - name: libvirt-guest
      network:
          - switch: sw0
            gateway: 10.0.0.1
            mac: "00:00:00:00:00:01"
            ip: "10.0.0.10"
      libvirt:
...
```

For a Docker container guest,

``` yaml
...
    - name: docker-guest
      network:
          - switch: sw0
            gateway: 10.0.0.1
            mac: "00:00:00:00:00:01"
            ip: "10.0.0.10"
      docker:
...
```
For an AVD guest,

``` yaml
...
    - name: avd-guest
      network:
        - switch: sw0
          gateway: 10.0.0.1
          mac: "00:00:00:00:00:01"
          ip: "10.0.0.10"
      avd:
...
```
