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

bench_start=$(date +%s%N)

# create a results directory for this script run
results_folder_name="$(date +'%Y-%m-%d_%H-%M-%S')_results"
mkdir -p "results/$results_folder_name"
cd "results/$results_folder_name"
results_folder=$(pwd)
#owner=$(id -un)
cd ../..
#chown -R $owner:$owner results/

# store the testbed-os folder so we can easily move between examples without having
# to work out relative paths between tests
cd deployments
project_dir=$(pwd)

# make sure the ssh key is the right permissions for the guests
ssh_key_loc="../../../../kvm-compose/kvm-compose/assets/id_ed25519_testbed_insecure_key"
# different relative path for chmod
sudo chmod 600 "../../../kvm-compose/kvm-compose/assets/id_ed25519_testbed_insecure_key"

# ssh opts to prevent previous connections breaking the script
ssh_opts="-o BatchMode=yes -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null"

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

# libvirt 3 vm example
echo "Running Libvirt 3 VM example ..."
libvirt3_up_elapsed=$(test_up "bench_three_libvirt")
sleep 1
libvirt3_down_elapsed=$(test_down "bench_three_libvirt")

# Clones example
echo "Running Clones example ..."
clones_up_elapsed=$(test_up "bench_clones")
sleep 1
clones_down_elapsed=$(test_down "bench_clones")

### Performance timings

## CPU

echo "Measuring example CPU/IO/Network stress test times"

# Libvirt one VM example
echo "Starting Libvirt one VM example ..."

cd "$project_dir/bench_libvirt"
kvm-compose up > /dev/null 2>&1
sleep 20

ssh -i $ssh_key_loc nocloud@172.16.1.10 $ssh_opts 'curl -sL https://yabs.sh | bash -s -- -w bench.json -j -s "-"'
ssh -i $ssh_key_loc nocloud@172.16.1.10 $ssh_opts 'cat bench.json' > $results_folder/one_vm_bench.json

kvm-compose down > /dev/null 2>&1
cd ..

# Libvirt three VM example

echo "Starting Libvirt three VM example ..."

cd "$project_dir/bench_three_libvirt"
kvm-compose up > /dev/null 2>&1
sleep 30

# run benchmark on three VMs, let the final one block
ssh -i $ssh_key_loc nocloud@172.16.1.11 $ssh_opts 'curl -sL https://yabs.sh | bash -s -- -w bench.json -j -s "-"' > /dev/null 2>&1 &
ssh -i $ssh_key_loc nocloud@172.16.1.12 $ssh_opts 'curl -sL https://yabs.sh | bash -s -- -w bench.json -j -s "-"' > /dev/null 2>&1 &
ssh -i $ssh_key_loc nocloud@172.16.1.13 $ssh_opts 'curl -sL https://yabs.sh | bash -s -- -w bench.json -j -s "-"'

# retrieve the results
echo "retrieving benchmark results from the three VMs ..."
until ssh -i $ssh_key_loc nocloud@172.16.1.11 -o BatchMode=yes -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null "[ -f \"bench.json\" ]" > /dev/null 2>&1; do
  echo "benchmark on VM 1 not yet ready, sleeping 10s"
  sleep 10
done
ssh -i $ssh_key_loc nocloud@172.16.1.11 $ssh_opts 'cat bench.json' > $results_folder/three_vm1_bench.json

until ssh -i $ssh_key_loc nocloud@172.16.1.12 -o BatchMode=yes -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null "[ -f \"bench.json\" ]" > /dev/null 2>&1; do
  echo "benchmark on VM 2 not yet ready, sleeping 10s"
  sleep 10
done
ssh -i $ssh_key_loc nocloud@172.16.1.12 $ssh_opts 'cat bench.json' > $results_folder/three_vm2_bench.json

until ssh -i $ssh_key_loc nocloud@172.16.1.13 -o BatchMode=yes -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null "[ -f \"bench.json\" ]" > /dev/null 2>&1; do
  echo "benchmark on VM 3 not yet ready, sleeping 10s"
  sleep 10
done
ssh -i $ssh_key_loc nocloud@172.16.1.13 $ssh_opts 'cat bench.json' > $results_folder/three_vm3_bench.json

kvm-compose down > /dev/null 2>&1

# done with tests
echo "benchmark suite finished, wrapping up ..."

# finally stop the testbed server
echo "stopping the testbed server"
kill $testbed_server_pid
wait $testbed_server_pid 2>/dev/null

# save results
cd "$project_dir/../results/$results_folder_name"

echo "saving up and down timing results into up.csv and down.csv, these are in milliseconds"
echo "avd,docker,libvirt,libvirt3,clones" > up.csv
echo "$avd_up_elapsed,$docker_up_elapsed,$libvirt_up_elapsed,$libvirt3_up_elapsed,$clones_up_elapsed," >> up.csv
echo "avd,docker,libvirt,libvirt3,clones" > down.csv
echo "$avd_down_elapsed,$docker_down_elapsed,$libvirt_down_elapsed,$libvirt3_down_elapsed,$clones_down_elapsed," >> down.csv

echo "benchmark results json files have been saved, the filename represents the test"

# give time of full suite
bench_end=$(date +%s%N)
bench_elapsed=$(( bench_end - bench_start ))
echo "the whole benchmark script took $(awk "BEGIN {print $bench_elapsed / 1000000000}")s to run"

echo "benchmarking done."
