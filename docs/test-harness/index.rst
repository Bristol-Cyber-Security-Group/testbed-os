================
Test Harness
================

The test harness is used to test all of the features of the testbed through integration tests.
Test cases have been created, in the form of `kvm-compose.yaml` and `kvm-compose-config.json` files to describe the test case.
These are meant to be comprehensive when put together to test the features over single and multiple testbed hosts.
The test cases are validated through "asset testing", where we check assets such as the created virtual machines and bridge.
Additionally, we also try connection tests to the guests and between guests to assert the network is working.

To run the tests, you will need enough memory to support up to the three testbed host configuration.
This configuration needs 5GB per testbed host, so at least 15GB on the machine running the test harness.
This probably wont work on a machine with only 16GB, it is recommended to use a machine with 32GB.
If this is not possible/available, then consider disabling the three testbed host tests in the `run_test_cases.sh` - you can do this by just commenting or deleting the sections.

Architecture
------------

The test harness will create a base image using ubuntu cloud-init, to install all the TestbedOS dependencies and source code.
There are a few tests here to make sure everything installed correctly.
Once this install has completed, this base image is turned off and is used as a backing image for clones, so that each test case starts with a fresh pre-installed testbed.

For every test case, whether a single testbed host or more, we create the required number of clones and push the test case configuration to the designated master.
On this master, the whole orchestration process is executed then the asset testing is executed.
Additionally, the test case is redeployed once with a "down" then "up" to make sure things still work on consecutive deployments.
After this second deployment test finishes, the test case ends and these testbed host clones are destroyed.
If any test in the test case fails, then the test harness will stop at that test and leave the testbed hosts running so that the developer can inspect the failed scenario.
If the test case passes, it will continue to the next test case and create new testbed host clones and repeat the process.

There is overlap between the testcases and expect more overlap as more test cases are added, however being comprehensive in the various testbed scenarios and testing each feature independently is useful.
Being battle tested over multiple repeated deployments is useful to shake out irregular bugs.

Technicals
----------

There are a few folders in the test harness folder, their uses:

:test_cases:

    These are the folders containing distinct test cases.
    There is a yaml file for the test and a few kvm-compose-config.json files, each describing the one, two, many host setups.
    Also includes any scripts/artefacts that the yaml file references.
    The test case folder is pushed to the testbed master host and becomes the project folder.

:assets:

    The files in here are used in deploying the infrastructure.
    There is the `iso` folder, which contains the cloud-init config files for the testbed hosts.
    There is the `ssh_key` folder which contains the ssh keys used and referenced by the kvm-compose-config.json.
    There is the `testbed-network.xml` which describes the libvirt network in which the testbed hosts exist in - this is not related to the testbed network the master testbed host creates.

:scripts:

    There are various scripts here to deploy the whole test case infrastructure and run the test cases.

:artefacts:

    In this folder any artefacts that are created from running test cases will be placed here.
    The base images will also be placed here.
    Each test case will also have its own folder created and results pushed into there, organised by the test case name.
    The test case result and state json files are placed here and timestamped so you can compare results between runs or inspect any failures.

Test Cases
----------

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
    This test case will solely test the snapshotting feature on top of the base test case.

Asset Testing
-------------

The list of tests will be continuously being updated as new features are added and bugs are found.
To prevent duplication of documentation, please see the asset test script for a detailed list of tests.
This script can be found in `test-harness/scripts/asset_test.py` and details in comments at the top of the script.

At a high level, the objective of the asset test script is to check the following:

- network bridges for libvirt and openvswitch are created
- the tunnels for openvswitch are created
- the guests are created
- guests are accessible via SSH and can have files pushed
- the guests can communicate with (all) other guests on the network
- the guests can communicate with the external web (i.e. to download further dependencies etc)

Snapshot Testing
----------------

The snapshot feature must work on local and remote testbed hosts.
By creating a snapshot of all guests, when using multiple testbed hosts the test will cover using the snapshot feature on remote hosts.
The test of a restore from snapshot will test that a file created after a snapshot is made will disappear when the snapshot is restored.


DEBUG Mode
----------

The test harness offers a debug mode, activated just by having the `DEBUG` environment variable set.
So for example `DEBUG = ./run_test_harness.sh` will enable debug mode.

This debug mode will skip the base image creation step (assuming you have already built it previously).
Additionally, the debug mode will trigger the whole repo to be synced to the master testbed host.
As the base image creation step can take a while, this allows you to quickly get into running the test cases if you are developing test cases or running the tests to see if a bug has been fixed.
