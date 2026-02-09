# Test Harness

We provide a test harness for TestbedOS in the TestbedOS code repository at [`testbed-os/test-harness`](https://github.com/Bristol-Cyber-Security-Group/testbed-os/tree/develop/test-harness) to test all of the features of the testbed through integration testing.
Deployment configurations have been created, in the form of [the `kvm-compose.yaml`](schema.md) and the `kvm-compose-config.json` configuration files to describe each test case.
These are meant to be comprehensive when put together to test the features over single and multiple TestbedOS hosts.
The test cases are validated through 'asset testing', where we check assets such as the created virtual machines and bridge.
Additionally, we also try testing the connection to the guests and between guests to assert the network is working.

## Running the Tests

The test harness can be run by building the Docker image through the Dockerfile from the TestbedOS code repository in [`testbed-os/test-harness`](https://github.com/Bristol-Cyber-Security-Group/testbed-os/tree/develop/test-harness) and then running the docker container.
Please see the [`README.md` file in the same code directory for more explicit instructions](https://github.com/Bristol-Cyber-Security-Group/testbed-os/tree/develop/test-harness/README.md). 

### Memory Requirements

To run the tests, you will need enough memory to support and run up to three TestbedOS hosts as we are running the test harness on three virtual machines where each acts as a single TestbedOS host.
Each test harness configuration for the TestbedOS host requires 5GB per host, so at least 15GB on the machine running the test harness.
It is recommended to use a machine with 32GB, as this probably will not work on a machine with only 16GB.
If such a machine is not possible or available, then please consider disabling one or more of the three host configurations in [`host_images.py`](https://github.com/Bristol-Cyber-Security-Group/testbed-os/blob/develop/test-harness/py_harness/config/host_images.py), you can do this by just commenting or deleting the sections, as two have been shown to be commented in the file.

<!-- ## Test Cases

To test the distributed capability of the testbed, all test cases will be executed with the 1,2,many philosophy.
In this case, this will be 1,2,3 testbed hosts.

Since we are only using up to three testbed hosts, we need three openvswitch bridges.
This is convenient as on a single testbed host, we will be able to test the veth connections between these bridges.
On two testbed hosts, due to the round robin load balancing we will be able to test two tunnels between the two testbed hosts as bridge 1 and 3 will be on host 1 and bridge 2 will be on host 2.
The bridges are connected as 1<=>2<=>3 so we test communication scenarios originating at bridge 1 going to bridge 2 then 3 (essentially arriving back on the original host but through the tunnel topology).
On three testbed hosts, there will be one bridge per host.

:base:
    This test case will simply deploy guests with a setup script.

:linked clone:
    This test case will use the linked clone feature, the backing image will have a shared setup script and the clones will also have a dummy setup script.

:snapshots:
    This test case will solely test the snapshotting feature on top of the base test case. -->

## Architecture

The test harness will create a base image using Ubuntu cloud-init for each of the aforementioned host configurations, to install all the TestbedOS dependencies and source code.
There are a few tests here to make sure everything is installed correctly.
Once the installation has completed, the base image is turned off and is used as a backing image for clones, so that each test case starts with a fresh pre-installed TestbedOS host.

For every test case, we create the required number of clones and push the test case configuration to the host.
On this host, the whole orchestration process is executed then the asset testing is executed.
Additionally, the test case is redeployed once with a `kvm-compose down` then `kvm-compose up` to make sure things still work on consecutive deployments.
After this test finishes, the test case ends and the host clones are destroyed.
If any test case fails, then the test harness will stop at that test and leave the hosts running so that the developer can inspect the failed scenario.
If the test case passes, it moves on to the next test case and creates new host clones to repeat the process for the next test case.

There is an overlap between the test cases and expect more overlap as more test cases are added, however being comprehensive in the various testbed scenarios and testing each feature independently is useful.
Being battle tested over multiple repeated deployments is useful to shake out irregular bugs.


## Technicals

There are a few components in the test harness folder of the TestbedOS code repository, [`testbed-os/test-harness`](https://github.com/Bristol-Cyber-Security-Group/testbed-os/tree/develop/test-harness). 
The following gives a quick introduction to each of them:

`assets`

: The files in here are used in deploying the infrastructure. 
The `iso` folder contains the cloud-init configuration files for the TestbedOS hosts in the test harness. 
The `ssh_key` folder contains the SSH keys used and referenced by `kvm-compose-config.json`. 
The `testbed-network.xml` configuration file describes the libvirt network in which the TestbedOS hosts exist in for the test harness and this is not related to the guest network the main TestbedOS host creates.

`py_harness`

: These are the various Python scripts deploy the test harness infrastructure and run the test cases. 
Please refer to the `README.md` file in the directory for more information.

`py_harness/test_cases`

: These are the folder containing a folder for each distinct test case.
Currently three test cases are included with the TestbedOS code repository, namely, `base`, `mount`, and `ovn`.
Each folder for the test cases include the necessary materials to set up the specific test case and also the `kvm-compose.yaml` file. 
During the execution of the test harness, this folder for the test case is pushed onto the `"Main"` TestbedOS host and becomes the TestbedOS project folder.
The test case result and state json files are placed here and timestamped so you can compare results between runs or inspect any failures.

`py_harness/test_cases/<test case>/artefacts`

: During the running of each test case, any artefact resulting from the execution will be placed here, similar to when a TestbedOS deployment is run.
This includes the base images.


## Asset Testing

At a high level, the objective of the testing the "assets" or the components in the deployment in each test case is to check the following:

- Network bridges for libvirt and openvswitch are created.
- The tunnels for openvswitch are created
- The guests are created.
- Guests are accessible via SSH and can have files pushed.
- The guests can communicate with (all) other guests on the network.
- The guests can communicate with the external web, e.g., to download further dependencies.

## Snapshot Testing

[The TestbedOS snapshot feature](user_interface_kvm_compose.md#subcommand---snapshot) works on local and remote TestbedOS hosts. 
The snapshot testing tests the creation of a snapshot of all guests using the feature on all local and remote hosts when multiple host configurations are enabled.
The testing also tests the restoration of a snapshot by checking if a file created before the restoration disappears after the snapshot is restored. 

<!-- ## DEBUG Mode

The test harness offers a debug mode, activated just by having the `DEBUG` environment variable set.
So for example `DEBUG = ./run_test_harness.sh` will enable debug mode.

This debug mode will skip the base image creation step (assuming you have already built it previously).
Additionally, the debug mode will trigger the whole repo to be synced to the main testbed host.
As the base image creation step can take a while, this allows you to quickly get into running the test cases if you are developing test cases or running the tests to see if a bug has been fixed. -->
