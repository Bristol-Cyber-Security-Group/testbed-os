#!/bin/bash

# Run all test cases, each test case requires a fresh set of testbed hosts built from the base
# testbed host image (using linked clones). The config for each test case must be pushed to the
# master testbed host for the test case. Once test is completed, the testbed hosts must be
# destroyed before the next test case.

# The testbed hosts will each need 4GB of memory as the single testbed test case will create three 1GB memory guests.
# For the three testbed host test case, this means the machine you are running the test case needs 12GB free to host
# the 3 4GB testbed hosts.

# this script can be run with debug mode to skip the base install (if it has already been done)
# DEBUG= ./run_test_cases.sh
# this will also push the current state of the source code into the master
#  (helpful to test new code changes without rebuilding the base image)

#cd scripts/ || exit

# make sure root is using the right python version
USER_POETRY=$(su -c 'which poetry' $ORIGINAL_USER)
USER_PYTHON=$(su -c 'which python' $ORIGINAL_USER)
CURRENT_DIR=$(pwd)

#echo $USER_POETRY
#echo $USER_PYTHON

configure_master () {
  TEST_CASE_NAME="$1"
  KVM_COMPOSE_CONFIG="$2"
  # wait for host to be up
  echo "Waiting for the VM to come up and accept SSH connections."
  for ii in {1..12}
    do
      if ssh -i ../assets/ssh_key/id_ed25519 -o "StrictHostKeyChecking no" -o "UserKnownHostsFile /dev/null" nocloud@testbed-host-one true; then
        echo "testbed host up"
        break
      else
        echo "testbed host not up, waiting 10s"
        sleep 10
      fi
      if [ $ii -eq 6 ]; then
        echo "testbed host was never reachable"
        exit
      fi
  done
  # push the configs
  rsync -av -e "ssh -i ../assets/ssh_key/id_ed25519 -o 'StrictHostKeyChecking no' -o 'UserKnownHostsFile /dev/null'"  \
    ../test_cases/$TEST_CASE_NAME/* \
    nocloud@testbed-host-one:/home/nocloud/project/
  ssh -i ../assets/ssh_key/id_ed25519 -o "StrictHostKeyChecking no" -o "UserKnownHostsFile /dev/null" nocloud@testbed-host-one "sudo mkdir -p /var/lib/testbedos/config/" || exit 1
  ssh -i ../assets/ssh_key/id_ed25519 -o "StrictHostKeyChecking no" -o "UserKnownHostsFile /dev/null" nocloud@testbed-host-one "sudo mv ~/project/$KVM_COMPOSE_CONFIG /var/lib/testbedos/config/kvm-compose-config.json" || exit 1
  # in debug mode, we push the codebase again and build source code again
  if [ -n "$DEBUG" ]; then
    echo "DEBUG: in debug mode current codebase will be pushed and built to master"
    rsync -avr -e "ssh -i ../assets/ssh_key/id_ed25519 -o 'StrictHostKeyChecking no' -o 'UserKnownHostsFile /dev/null'"  \
      --exclude "test-harness/" \
      --exclude "artefacts/*" \
      --exclude "target/*" \
      --exclude ".git/*" \
      ../../../testbed-os \
      nocloud@testbed-host-one:/home/nocloud/
    # install testbedos tools
    ssh -i ../assets/ssh_key/id_ed25519 -o "StrictHostKeyChecking no" -o "UserKnownHostsFile /dev/null" nocloud@testbed-host-one "cd /home/nocloud/testbed-os/ && bash -l ./setup.sh"
  fi
}

run_asset_test () {
  # makes sure to run the python poetry environment in the users environment
  # also makes sure to prevent this error 'OSError: [Errno 25] Inappropriate ioctl for device' with --session-command
  # the env use makes sure that between restarts the right env is used.. could be fixed by adding a .python-version file in the scripts folder?
#  runuser -l $ORIGINAL_USER --session-command "cd $CURRENT_DIR && $USER_POETRY env use 3.10.5"
  runuser -l $ORIGINAL_USER --session-command "cd $CURRENT_DIR && $USER_POETRY run python asset_test.py $TEST_CASE_NAME"
}

test_case () {
  # setup test case specific variables
  TEST_CASE_NAME="$1"
  KVM_COMPOSE_CONFIG="$2"
  echo "Running test case '$TEST_CASE_NAME'"

  configure_master $TEST_CASE_NAME $KVM_COMPOSE_CONFIG

  # start the testbed os server
  ssh -i ../assets/ssh_key/id_ed25519 -o "StrictHostKeyChecking no" -o "UserKnownHostsFile /dev/null" nocloud@testbed-host-one "sudo systemctl start testbedos-server.service" || exit 1

  # run test case
#  ssh -i ../assets/ssh_key/id_ed25519 -o "StrictHostKeyChecking no" -o "UserKnownHostsFile /dev/null" nocloud@testbed-host-one "cd ~/project/ && kvm-compose generate-artefacts" || exit 1
  ssh -i ../assets/ssh_key/id_ed25519 -o "StrictHostKeyChecking no" -o "UserKnownHostsFile /dev/null" nocloud@testbed-host-one "cd ~/project/ && sudo kvm-compose up" || exit 1
  # test for assets created - need to use the user's environment to access the poetry environment (which is in the users profile)
  run_asset_test || exit 1
  # TODO - this sleep and the sleep 10 below are required as for some reason the ssh host keys of the guests become
  #  0 byte files if they are restarted too quickly, this is not seen when doing it manually as you normally wouldn't
  #  do it that quick so this bug has only been seen in the test harness. 10 second sleep was not enough so left it
  #  at 20 ... needs some investigation..
  sleep 20
  # down test case
  ssh -i ../assets/ssh_key/id_ed25519 -o "StrictHostKeyChecking no" -o "UserKnownHostsFile /dev/null" nocloud@testbed-host-one "cd ~/project/ && sudo kvm-compose down" || exit 1
  sleep 10
  # TODO - test for assets destroyed
  # run up again to test a consecutive up (as kvm disks are preserved)
  ssh -i ../assets/ssh_key/id_ed25519 -o "StrictHostKeyChecking no" -o "UserKnownHostsFile /dev/null" nocloud@testbed-host-one "cd ~/project/ && sudo kvm-compose up" || exit 1
  # test for assets created again
  run_asset_test || exit 1
  # down test case
  ssh -i ../assets/ssh_key/id_ed25519 -o "StrictHostKeyChecking no" -o "UserKnownHostsFile /dev/null" nocloud@testbed-host-one "cd ~/project/ && sudo kvm-compose down" || exit 1
  # TODO - test for assets destroyed
  echo "Success: End of test case '$TEST_CASE_NAME'"
}

test_snapshot () {
  # setup test case specific variables
  TEST_CASE_NAME="$1"
  KVM_COMPOSE_CONFIG="$2"
  echo "Running test case '$TEST_CASE_NAME'"

  configure_master $TEST_CASE_NAME $KVM_COMPOSE_CONFIG

  # run test case
#  ssh -i ../assets/ssh_key/id_ed25519 -o "StrictHostKeyChecking no" -o "UserKnownHostsFile /dev/null" nocloud@testbed-host-one "cd ~/project/ && kvm-compose generate-artefacts" || exit 1
  ssh -i ../assets/ssh_key/id_ed25519 -o "StrictHostKeyChecking no" -o "UserKnownHostsFile /dev/null" nocloud@testbed-host-one "cd ~/project/ && sudo kvm-orchestrator up" || exit 1

  ## Test snapshot list (there should be none)

  # TODO - list snapshots

  ## Test snapshot create and restore
  # TODO - create snapshot of all guests

  # TODO - add a file to all guests

  # TODO - restore from snapshot

  # TODO - check that the file does not exist anymore

  ## Test snapshot list

  # TODO - get snapshot information

  # TODO - list all snapshots

  # TODO - list individial guest snapshots

  ## Test snapshot delete

  # TODO - delete snapshot for all guests

  ## Test snapshot list (there should be none)

  # TODO - list snapshots

  # TODO - this sleep and the sleep 10 below are required as for some reason the ssh host keys of the guests become
  #  0 byte files if they are restarted too quickly, this is not seen when doing it manually as you normally wouldn't
  #  do it that quick so this bug has only been seen in the test harness. 10 second sleep was not enough so left it
  #  at 20 ... needs some investigation..
  sleep 20
  # down test case
  ssh -i ../assets/ssh_key/id_ed25519 -o "StrictHostKeyChecking no" -o "UserKnownHostsFile /dev/null" nocloud@testbed-host-one "cd ~/project/ && sudo kvm-orchestrator down" || exit 1

  echo "Success: End of test case '$TEST_CASE_NAME'"
}

down_testbed_hosts () {
  sudo virsh shutdown testbed-host-one
  sudo virsh undefine testbed-host-one
  sudo virsh shutdown testbed-host-two
  sudo virsh undefine testbed-host-two
  sudo virsh shutdown testbed-host-three
  sudo virsh undefine testbed-host-three
}

################
# Test Case Info
################

# create testbed host for test case, specify the name of the clone and the mac address
# (the mac address should be one of the macs listed in the 'testbed-network.xml' dhcp section
# ./deploy_testbedhost_clone.sh one 52:54:00:00:00:00

# scenarios:
# - base:
# - linked clones:

start_time=`date +%s`

###########
# Base Test
###########

## run test case - single host
./deploy_testbedhost_clone.sh one 52:54:00:00:00:00
test_case "base" "kvm-compose-config.json"
down_testbed_hosts

## run test case - two host
./deploy_testbedhost_clone.sh one 52:54:00:00:00:00
./deploy_testbedhost_clone.sh two 52:54:00:00:00:01
test_case "base" "kvm-compose-config-2host.json"
down_testbed_hosts

# run test case - three host
./deploy_testbedhost_clone.sh one 52:54:00:00:00:00
./deploy_testbedhost_clone.sh two 52:54:00:00:00:01
./deploy_testbedhost_clone.sh three 52:54:00:00:00:02
test_case "base" "kvm-compose-config-3host.json"
down_testbed_hosts

###########
# Linked Clone Test
###########

# run test case - single host
./deploy_testbedhost_clone.sh one 52:54:00:00:00:00
test_case "linked_clone" "kvm-compose-config.json"
down_testbed_hosts

# run test case - two host
./deploy_testbedhost_clone.sh one 52:54:00:00:00:00
./deploy_testbedhost_clone.sh two 52:54:00:00:00:01
test_case "linked_clone" "kvm-compose-config-2host.json"
down_testbed_hosts

# run test case - three host
./deploy_testbedhost_clone.sh one 52:54:00:00:00:00
./deploy_testbedhost_clone.sh two 52:54:00:00:00:01
./deploy_testbedhost_clone.sh three 52:54:00:00:00:02
test_case "linked_clone" "kvm-compose-config-3host.json"
down_testbed_hosts

# TODO - multiple linked clone backing images, server (1,2) client (3)?
#  to test process of working out pushing backing disks to remote testbed hosts
#  but to also test when there needs to be two clones on one testbed host

###########
# TODO - Complex Network Topology?
###########

###########
# TODO - Different Load balancing algorithms
###########

###########
# Snapshot Test
###########

## run test case - single host
#./deploy_testbedhost_clone.sh one 52:54:00:00:00:00
#test_snapshot "snapshot" "kvm-compose-config.json"
#down_testbed_hosts

# TODO - run test case - two host
# TODO - run test case - three host

#####################
# END OF TEST HARNESS
#####################
end_time=`date +%s`
runtime=$((end_time-start_time))
echo "All tests passed! Tests took $runtime (s)"
