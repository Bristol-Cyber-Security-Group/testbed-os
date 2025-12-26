# Benchmarking

The benchmarking suite can be found in the `util/benchmarking` folder.
You can run it with `sudo ./run_benchmarking.sh`.

The tests are split up into two:

1. time to start and stop a deployment of different guest types
2. time the guests take to run performance intensive tasks

- Test 1. simply times the commands.

- Test 2. uses the [`Yet Another Bench Script`](https://github.com/masonr/yet-another-bench-script) tool.

The results are recorded in a results folder with the time and date of the execution of the benchmark.

- Test 1. is recorded into two CSV files, one for up and one for down and is recorded in milliseconds.

- Test 2. stores the JSON output from the YABS script.
These results can then be examined or parsed by a script.

## Test Cases

There are five test cases and they are as follows:

- bench AVD
    - deploys an Android emulator with AVD
    - used in Test 1
- bench clones
    - creates one reference libvirt cloud-init image, then deploys three linked clones off this base image
    - used in Test 1
- bench docker
    - deploys one docker container
    - used in Test 1
- bench libvirt (one VM)
    - deploys one libvirt cloud-init image
    - used in Test 1 and 2
- bench libvirt (three VMs)
    - deploys three libvirt cloud-init images
    - used in Test 2


