# TestbedOS Android Virtual Device

TestbedOS additionally provides the capability of spawning Android emulators for the research and experimentation on the Android platform implemented through the [Android Virtual Device feature](https://developer.android.com/studio/run/emulator-commandline#listing-filedir). This is available through the `avd` field in the `kvm-compose.yaml` file. The following lists the available options for the `avd` section:

- **`android_api_version`**: the API version for the AVD, and
- **`playstore_enabled`**: to allow Google Play Store and other Google services on the AVD.

## Minimal Working Example

This section demonstrates on running a Minimal Working Example (MWE) for an Android guest. The steps are similar to deploying a libvirt guest at the start of the documentation in [Minimal Working Example](welcome.md#minimal-working-example).


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