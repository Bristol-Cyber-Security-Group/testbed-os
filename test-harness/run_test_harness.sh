#!/bin/bash

# This script will create a base ubuntu image with all the testbedOS tools and source code, so that we can create
# clones from this base install as fresh environments for the various test cases.
# If a test fails, the test harness will stop at the failure and leave the hosts running to let you inspect the scenario
# Important: This test harness will push the local repository as is, so any in-progress code change i.e. in the rust
# code will be pushed, which can be useful but careful not to do the real tests on WIP code

# This script can also be re-used to create a local testbed environment (with a bit of manual editing).
# To do this,
# - you can comment out the run test cases script
# - look in the run test cases script and copy the "./deploy_testbedhost_clone.sh one 52:54:00:00:00:00"
#   to create up to three testbed hosts "one,two,three" with the respective mac addresses that increment by 1
# - look in the run test cases script to also see how to run various orchestration commands inside the "test_case"
#   function
# Dont forget to undo any changes if you want to then run the test harness.

# this script must be run as sudo
# it basically runs this script again but as sudo with the extra variables, but once that returns/exits
# the original invocation of the script (not as sudo as it was originally) will want to continue so you
# must exit $? to prevent this script from running twice
if [ $EUID != 0 ]; then
    ORIGINAL_USER=$(whoami)
    if [[ -v DEBUG ]]; then
      echo "restarting script with sudo and debug"
      sudo DEBUG=1 ORIGINAL_USER=$ORIGINAL_USER "$0" "$@"
      exit $?
    else
      echo "restarting script with sudo"
      sudo ORIGINAL_USER=$ORIGINAL_USER "$0" "$@"
      exit $?
    fi
fi

cd scripts/ || exit

# make sure the base ubuntu image is installed
./download_ubuntu.sh

# setup libvirt network for testbed hosts
./setup_libvirt_network.sh

echo "make sure all test harness hosts are down before continuing"
sudo virsh destroy testbed-host-one
sudo virsh destroy testbed-host-two
sudo virsh destroy testbed-host-three
sudo virsh destroy testbed-host-base
sleep 3
sudo virsh undefine testbed-host-one
sudo virsh undefine testbed-host-two
sudo virsh undefine testbed-host-three
sudo virsh undefine testbed-host-base

# create base testbed host, debug skips assuming it already exists
if [ -z $DEBUG ]; then
  ./base_testbedhost_setup.sh
  if [ $? -eq 1 ]; then
    echo "Testbed base install did not complete sucessfully."
    sudo virsh shutdown testbed-host-base
    exit 1
  fi
else
  echo "DEBUG: In debug mode, skipping base testbed host setup"
fi

sudo virsh shutdown testbed-host-base

# give the shutdown command a second to complete before continuing
echo "waiting for base testbed image to shut down, if it was up"
sleep 5

### run single testbed host test cases

DEBUG=$DEBUG ORIGINAL_USER=$ORIGINAL_USER ./run_test_cases.sh

# cleanup
#./destroy_libvirt_network.sh
