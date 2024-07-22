# Examples

In this folder is a collection of examples to showcase the features of the testbed.
Once you have installed the testbed, you can run `kvm-compose up` in each folder to deploy the example.

# Example Description

## acl

This example deploys three libvirt guests on one switch, and one libvirt guest on another switch.
The aim is to demonstrate the OVN ACL security rules.
Client1 and client2 can communicate, and are on the same switch.
Client2 and client3 can communicate, and are on the same switch.
Client1 and client3 cannot communicate, and are on the same switch.
Client4 cannot communicate with any of the other clients, and is on a separate switch.
This is achieved by placing strict rules on source and destination ip addresses.
These rules are also at a higher priority than a drop all rule, which prevents any traffic due to the filter being generic to all ip traffic.

## avd

This example deploys an android guest, that has access to the internet.
You can experiment with the tooling included with the testbed by running `kvm-compose exec phone tool <choose your tool>`.
Use the `--help` flag after tool to find out which tools are available and their options.

## clones

This example demonstrates the use of scaling out one libvirt guest definition into many guests using linked clones, so that you can do an initial setup once and have many copies quickly deployed.
This means the linked clones of the original guest image will share the same filesystem setup i.e. installed software and data.
These linked clones will also only use the minimal disk space, which is the difference of their filesystem to the original - meaning linked clones is an efficient way to use disk space.

## docker

This example demonstrates docker based guests, including the scaling as demonstrated in the `clones` example.

## ovn

This example demonstrates the different networking options available in the testbed yaml schema.
In the yaml file, there are different guest definitions that have been commented out, which you can uncomment to enable to see.

## sigadb

This example demonstrates how you can utilise an existing VM image to be used in the testbed.
You will need to update the line with `path: /var/lib/libvirt/images/avd-ubuntu22.04.qcow2` to point to the existing image on your filesystem.

## signal

This example deploys two libvirt guests with the signal cli, and one guest with the open source signal server.
There is a README in this example to explain how to further set up and connect the cli clients to the signal server.

## multi interface

This example showcases two libvirt guests, where guest one has two interfaces.
The second interface will be used to communicate with guest two.
Guest two can only communicate with guest one on this second interface, and we utilise ACL rules to prevent this second guest communicating to the first interface on guest one.

