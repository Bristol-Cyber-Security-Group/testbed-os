# Test Harness

This is the test harness for the testbed, with the objective to test all the features and processes of the testbed code.
However this is not a replacement for testing on physical hardware, which will be a little different.
This test harness will attempt to test as much as possible in a virtual environment.

This is a collection of scripts to deploy N number of libvirt virtual machines to emulate a testbed environment on a single machine.
These scripts will deploy specifically crafted testbed test cases to use the various testbed features.

Test completion and success is determined by checking assets of the end state, in addition to a successful deployment.
A further set of tests will be functional to assert certain behaviours are present i.e. testbed guests can communicate with other guests.

While this test harness is similar to the testbed processes and also uses libvirt, we remove ambiguity by being specific in all parameterisation of the test harness.

Terminology:
- Testbed Host - The hosts that underpin the testbed
- Testbed Guest - The hosts that are run on the testbed hosts across the testbed

# Architecture

We will use cloud-init as a base image for the testbed hosts to simplify getting credentials into the host to control remotely.

We will use snapshots of the testbed hosts to speed up the testing from known working states.

We will use the design of "one, two and many" to test the testbed's capability to work in the various scenarios.
However, the "many" will remain only as three testbed hosts due to our hardware limitation.

## Testing Phase 1
This testing phase only concerns the installation of the testbed onto a fresh testbed host.
Additionally, the connection between the hosts will be tested.
Assets such as the guest cloud-init images to be used will also be downloaded.

The output of this phase will be snapshotted as a starting point for the tests in phase 2.

## Testing Phase 2
This testing phase will test each pre-defined test case, defined as the kvm-compose.yaml files.

All assets for each test case are asserted for their existence, including:
- bridges, tunnels and veths
- guests
  - guest ip address
  - guest communication across the testbed (single and multiple testbed hosts)
  - guest external network connectivity

Further tests:
- snapshots
- CLI commands not directly involved in orchestration of a test case
- running commands and pushing/pulling files to/from guests
- network isolation


# Features Explicitly Not Tested

- guests with desktop environments
  - reason: Guests with a GUI has been experimental in this project. Testing and automating graphical user interfaces is more complex aside from asserting the desktop environment exists. The performance of nested guests is also prohibitive by increasing testing complexity due to delays in input.
- 

# Usage

Execute the single test harness script and wait for all tests to complete.
Make sure libvirt is installed and the cloud-init image is downloaded into this folder.
