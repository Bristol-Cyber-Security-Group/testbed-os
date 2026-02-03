# Benchmarking

The benchmarking suite can be found in the `util/benchmarking` folder from the TestbedOS source code [`testbed-os`](https://github.com/Bristol-Cyber-Security-Group/testbed-os).

The benchmark is tested on different deployment configuration (for details please see [deployments](#deployments)) and it is split up into two:

Test 1
: times to start and stop a deployment of different guest types. This test simply times the `kvm-compose up` and `kvm-compose down` commands for each deployment. The results are recorded into two CSV files in milliseconds, namely, `up.csv` and `down.csv` for `kvm-compose up` and `kvm-compose down` respectively.

Test 2
: times the guests take to run performance intensive tasks. This test uses the [`Yet Another Bench Script`](https://github.com/masonr/yet-another-bench-script) tool. stores the JSON output from the YABS script.

You can run the benchmark with `sudo ./run_benchmark.sh`. 
All results are recorded in the `results` folder with the time and date of the execution of the benchmark.

## Deployments

The benchmark tests in the previous section runs the following different deployments:

- Test 1: deploys an Android emulator with AVD.
- Test 1: creates one reference libvirt cloud-init image (VM), then deploys three linked clones off this base image.
- Test 1: deploys one docker container.
- Test 1 and 2: deploys one libvirt cloud-init image.
- Test 2: deploys three libvirt cloud-init images (VMs).