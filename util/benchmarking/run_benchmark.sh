#!/bin/bash

### BENCHMARKING

# NOTE: this benchmarking script will destroy deployments before running, so make sure to backup
#       data that you want to keep on any guests.
# NOTE: the example deployment kvm-compose yaml files will be taken as is, so make sure they have
#       not been edited as this will impact the benchmark results
# NOTE: make sure the testbed server is not running
# NOTE: if the images of the guests have not yet been downloaded, this will impact the first run of
#       the benchmark, so either run this once to get these downloaded and discard the result or
#       prep the environment ahead of time

# store the testbed-os folder so we can easily move between examples without having
# to work out relative paths between tests
cd deployments
project_dir=$(pwd)

# we need to launch the testbed server, run it in the background then stop it a the end of the script
# to run it, we need to be in it's proper run folder
echo "starting the testbed server in the background"
cd /var/lib/testbedos/
sudo testbedos-server > /dev/null 2>&1 &
testbed_server_pid=$!
# give the server a second to start up before running commands
sleep 3

# move back to the project folder
cd $project_dir

clear_deployment () {
  kvm-compose clear-artefacts > /dev/null 2>&1
  rm *-state.json > /dev/null 2>&1
}

# args - 1:project name
test_up () {
  cd $project_dir/$1
  clear_deployment

  start=$(date +%s%N)
  kvm-compose up > /dev/null 2>&1
  end=$(date +%s%N)
  up_elapsed=$(( (end - start) / 1000000 ))
  echo "$up_elapsed"
}

# args - 1:project name
test_down () {
  cd $project_dir/$1

  start=$(date +%s%N)
  kvm-compose down > /dev/null 2>&1
  end=$(date +%s%N)
  up_elapsed=$(( (end - start) / 1000000 ))

  clear_deployment

  echo "$up_elapsed"
}

### Deploy Timings

echo "Measuring example deploy times"

# AVD example
echo "Running AVD example ..."
avd_up_elapsed=$(test_up "bench_avd")
sleep 1
avd_down_elapsed=$(test_down "bench_avd")

# Docker example
echo "Running Docker example ..."
docker_up_elapsed=$(test_up "bench_docker")
sleep 1
docker_down_elapsed=$(test_down "bench_docker")

# Libvirt example
echo "Running Libvirt example ..."
libvirt_up_elapsed=$(test_up "bench_libvirt")
sleep 1
libvirt_down_elapsed=$(test_down "bench_libvirt")

# Clones example
echo "Running Clones example ..."
clones_up_elapsed=$(test_up "bench_clones")
sleep 1
clones_down_elapsed=$(test_down "bench_clones")


# print timings
echo "== Deploy Timings"
echo "time for AVD example to deploy: $avd_up_elapsed ms"
echo "time for AVD example to destroy: $avd_down_elapsed ms"
echo "time for Docker example to deploy: $docker_up_elapsed ms"
echo "time for Docker example to destroy: $docker_down_elapsed ms"
echo "time for Libvirt example to deploy: $libvirt_up_elapsed ms"
echo "time for Libvirt example to destroy: $libvirt_down_elapsed ms"
echo "time for Clones example to deploy: $clones_up_elapsed ms"
echo "time for Clones example to destroy: $clones_down_elapsed ms"

# finally stop the testbed server
echo "stopping the testbed server"
kill $testbed_server_pid
wait $testbed_server_pid 2>/dev/null
