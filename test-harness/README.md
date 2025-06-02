# Test Harness

This is the test harness for the testbed, with the objective to test all the features and processes of the testbed code.
However, this is not a replacement for testing on physical hardware, which will be a little different.
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

We use cloud-init as a base image for the testbed hosts to simplify provisioning and configuration.
The base image operating systems will range over the supported operating systems for the testbed.
Once the testbed is installed on the base image, we will utilise linked clones to spawn N number of identical hosts that will be further configured.
The use of linked clones just makes clearing and starting a new test environment space and time efficient.

The tests will be repeated on one, two and three tested hosts to test the clustering feature and internal processes.

Given that we are building virtual machines for the testbed hosts, the testbed guests will be nested virtual machines.
These nested virtual machines will be short-lived and will not be doing any intense compute of I/O work so the performance hit will be negligible.

## Testing Phase 1
This testing phase only concerns the installation of the testbed onto a fresh testbed host.
Additionally, the connection between the hosts will be tested.
Assets such as the guest cloud-init images to be used will also be downloaded.

The output of this phase will be snapshotted as a starting point for the tests in phase 2.

## Testing Phase 2
This testing phase will test each pre-defined test case, defined as the kvm-compose.yaml files.

Runtime tests such as:
- snapshots
- CLI commands not directly involved in orchestration of a test case
- running commands and pushing/pulling files to/from guests
- exhaustive network feature tests based on the yaml file constraints


# Features Explicitly Not Tested

- guests with desktop environments
  - reason: Guests with a GUI has been experimental in this project. Testing and automating graphical user interfaces is more complex aside from asserting the desktop environment exists. The performance of nested guests is also prohibitive by increasing testing complexity due to delays in input.
- android guests (for now)
  - reason: The testbed host must have a desktop environment to support the emulators graphics dependencies, we are currently only using headless hosts

# Results and Reports

The test harness will output a report at the end of the run.
This will contain the results for all tests executed, in a nested JSON format.
The nesting captures the parameter combinations of base operating system, number of hosts and the test cases.
Generally following test pass/fail result record using booleans.

# Usage and CICD

The test harness has been hooked up to the GitHub actions workflow.
It will run on every PR and commit push for each branch.

# Adding More Test Coverage

Please see the README.md in the `test_cases` folder.
