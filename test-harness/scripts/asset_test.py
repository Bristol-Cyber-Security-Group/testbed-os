import sys
import json
from fabric import Connection
import invoke
import datetime
import os

# For each test case, where the name should be passed as the first argument to this script, it will create
# a folder in the test harness artefacts test cases folder containing any artefacts and results.

# To keep things simple for now, if there is a failed test, the test harness will log the test, continue but exit
# with error code 1 to prevent the test harness from continuing.
# This is so that you can then inspect the failed environment to start debugging.
# This test harness was not designed to just run completely and just report, like CICD but as a way
# to have reproducible bugs and environment.
# Don't forget to delete the clones manually once you are done with investigating the failed scenario to run the
# tests again.

# This script will ssh into the testbed hosts to check for the assets that should be created as defined
# in the project-state.json, which represents everything that should be created for the testbed test case.
# The following assets are checked:
# 1) existence of guests
# 2) the libvirt network is created (and the bridge)
# 3) the ovs bridges are created
# 4) the veths are created
#    4a) veth between libvirt project bridge and the ovs entry point on main host
#    4b) veth between ovs bridges
# 5) the tunnels between testbed hosts are created
# 6) ability to ssh to guest via hostname (which also tests if the guest has an ip)
# 7) check if any context/scripts has been pushed to guests
# 8) guests can communicate with eachother over the deployed network
# 9) guests can communicate with the external web

# What it cant do/ Limitations
# - automate checking if the run/setup scripts have executed
#   (unless a specific test is created for a specific set of scripts)
# - since we are testing artefacts being pushed (setup script, context) and all the tests here are executed
#   then all the test cases must have the same tests, until we enable granular test selection
# - when different guest types are introduced, it might be useful to split up the asset test script into
#   specific network, kvms, docker etc scripts to remove the constraints above
# - if the network is segmented logically or through openflow rules, the connection tests will fail

# Potential tests
# - check linked clone golden images should be turned off?
#   if the clones are on then this is necessarily off due to write lock
# - run script/command but using the python code rather than a direct ssh command

# TODO processes to be checked, different script?
# - creating, listing and deleting snapshots TODO once the interface has been updated

TEST_CASE_NAME = sys.argv[1]
TESTBED_HOSTNAME = "nocloud@testbed-host-one"
TESTBED_HOST_KEY = "../assets/ssh_key/id_ed25519"
PROJECT_STATE_SAVE_LOCATION = f"../artefacts/test_cases/{TEST_CASE_NAME}/"
TEST_TIME = datetime.datetime.now().replace(microsecond=0).isoformat()
CURRENT_PROJECT_STATE = PROJECT_STATE_SAVE_LOCATION + "project-state.json"
TIMESTAMPED_PROJECT_STATE = PROJECT_STATE_SAVE_LOCATION + f"{TEST_TIME}_project-state.json"

# connection to testbed main
ssh = Connection(TESTBED_HOSTNAME,
                 connect_kwargs={"key_filename": TESTBED_HOST_KEY},
                 gateway=None)

test_case_results = []
failures_occurred = []


def log_success(in_success):
    success_text = "SUCCESS: " + in_success
    print("\033[92m" + success_text + "\033[0m")
    test_case_results.append(success_text)


def log_failure(in_failure):
    failure_text = "FAILURE: " + in_failure
    print("\033[91m" + failure_text + "\033[0m")
    test_case_results.append(failure_text)
    failures_occurred.append(failure_text)


def exit_test_harness(end_message):
    print(end_message)
    print(f"FAIL: Exiting test harness for test case '{TEST_CASE_NAME}' due to the above error.")
    exit(1)


def get_project_state():
    # place the project-state.json into the test harness artefacts folder under this test case's name
    ssh.get("/home/nocloud/project/project-state.json", local=PROJECT_STATE_SAVE_LOCATION)


def read_project_state() -> dict:
    with open(CURRENT_PROJECT_STATE, "r") as f:
        state = json.load(f)
    # print(state_json)
    return state


def test_libvirt_guest_exists(in_host, in_hostname):
    try:
        testbed_hosts_connections[in_host].run(f"sudo virsh domstate {in_hostname}")
        log_success(f"TEST 1 - Testbed guest {in_hostname} exists")
    except invoke.exceptions.UnexpectedExit as e:
        # print(e)
        log_failure(f"TEST 1 - Testbed guest {in_hostname} did not exist")


def test_docker_guest_exists(in_host, in_container_name):
    try:
        testbed_hosts_connections[in_host].run(f"sudo docker ps | grep {in_container_name} | wc -l")
        log_success(f"TEST 1 - Testbed guest {in_container_name} exists")
    except invoke.exceptions.UnexpectedExit as e:
        # print(e)
        log_failure(f"TEST 1 - Testbed guest {in_container_name} did not exist")

def test_libvirt_network(in_host):
    try:
        print("testing for the existence of the libvirt network")
        testbed_hosts_connections[in_host].run(f"sudo virsh net-info --network project-network")
    except invoke.exceptions.UnexpectedExit as e:
        # print(e)
        log_failure(f"Test 2 - Testbed libvirt network did not exist")
        return
    try:
        print("testing for the existence of the libvirt network bridge")
        testbed_hosts_connections[in_host].run(f"ip addr show project-prjbr0")
    except invoke.exceptions.UnexpectedExit as e:
        # print(e)
        log_failure(f"TEST 2 - Testbed libvirt network did not exist")
        return
    log_success(f"TEST 2 - libvirt network exists on {in_host}")


def test_ovs_bridges(ovs_bridge_list):
    print(f"TEST 3: testing ovs bridges")
    for bridge, testbed_host in ovs_bridge_list:
        try:
            print(f"testing for the existence of the ovs bridge {bridge} in host {testbed_host}")
            testbed_hosts_connections[testbed_host].run(f"sudo ovs-vsctl br-exists {bridge}")
            log_success(f"TEST 3 - Testbed ovs bridge {bridge} exists in host {testbed_host}")
        except invoke.exceptions.UnexpectedExit as e:
            # print(e)
            log_failure(f"TEST 3 - Testbed ovs bridge {bridge} did not exist in host {testbed_host}")


def test_veth_connections_4a(in_host):
    try:
        print("test veth pair connected to libvirt network bridge")
        result = testbed_hosts_connections[in_host].run(f"ip link show master project-prjbr0")
        if "project-veth1@" not in result.stdout:
            log_failure(f"TEST 4a - Testbed veth was not connected to libvirt network bridge")
            return
    except invoke.exceptions.UnexpectedExit as e:
        # print(e)
        log_failure(f"TEST 4a - Testbed veth test failed as the bridge did not exist")
        return
    try:
        print("test veth pair connected to external ovs bridge")
        result = testbed_hosts_connections[in_host].run(f"sudo ovs-vsctl list-ports project-br0")
        if "project-veth0" not in result.stdout:
            log_failure(f"TEST 4a - Testbed veth was not connected to ovs external bridge")
            return
    except invoke.exceptions.UnexpectedExit as e:
        # print(e)
        log_failure(f"TEST 4a - Testbed veth test failed as the ovs bridge did not exist")
        return
    log_success(f"TEST 4a - Testbed veth was connected to libvirt bridge")


def test_veth_connections_4b(in_bridge_connection):
    source_br = in_bridge_connection["source_br"]
    target_br = in_bridge_connection["target_br"]
    source_veth = in_bridge_connection["source_veth"]
    target_veth = in_bridge_connection["target_veth"]
    try:
        print(f"test veth existence on {source_br}")
        result = testbed_hosts_connections[in_bridge_connection["testbed_host"]].run(f"sudo ovs-vsctl list-ports {source_br}")
        if source_veth not in result.stdout:
            log_failure(f"TEST 4b - Testbed veth '{source_veth}' was not connected to ovs bridge '{source_br}'")
            return
    except invoke.exceptions.UnexpectedExit as e:
        # print(e)
        log_failure(f"TEST 4b - Testbed veth test failed as the ovs bridge '{source_br}' did not exist")
        return
    try:
        print(f"test veth existence on {target_br}")
        result = testbed_hosts_connections[in_bridge_connection["testbed_host"]].run(f"sudo ovs-vsctl list-ports {target_br}")
        if target_veth not in result.stdout:
            log_failure(f"Test 4b - Testbed veth '{target_veth}' was not connected to ovs bridge '{target_br}'")
            return
    except invoke.exceptions.UnexpectedExit as e:
        # print(e)
        log_failure(f"TEST 4b - Testbed veth test failed as the ovs bridge '{target_br}' did not exist")
        return
    log_success(f"TEST 4b - Testbed veth has connection to ovs bridges")


def test_geneve_tunnels(in_tunnel_connection):
    source_br = in_tunnel_connection["source_br"]
    target_br = in_tunnel_connection["target_br"]
    key = in_tunnel_connection["key"]
    source_remote_ip = in_tunnel_connection["source_remote_ip"]
    target_remote_ip = in_tunnel_connection["target_remote_ip"]
    # test on source side
    try:
        print(f"test tunnel existence on bridge {source_br} on host {in_tunnel_connection['testbed_host_source']}")
        result = testbed_hosts_connections[in_tunnel_connection["testbed_host_source"]].run(f"sudo ovs-vsctl show")
        target_string = "options: {key=" + key + ', remote_ip="' + source_remote_ip + '"}'
        if target_string not in result.stdout:
            log_failure(f"TEST 5 - the tunnel was not created for bridge '{source_br}' on host '{in_tunnel_connection['testbed_host_source']}'")
            return
    except invoke.exceptions.UnexpectedExit as e:
        # print(e)
        log_failure(f"TEST 5 - Testbed tunnel on {source_br} did not exist")
        return
    # test on target side
    try:
        print(f"test tunnel existence on bridge {target_br} on host {in_tunnel_connection['testbed_host_target']}")
        result = testbed_hosts_connections[in_tunnel_connection["testbed_host_target"]].run(f"sudo ovs-vsctl show")
        target_string = "options: {key=" + key + ', remote_ip="' + target_remote_ip + '"}'
        if target_string not in result.stdout:
            log_failure(f"Test 5 - The tunnel was not created for bridge '{target_br}' on host '{in_tunnel_connection['testbed_host_target']}'")
            return
    except invoke.exceptions.UnexpectedExit as e:
        # print(e)
        log_failure(f"TEST 5 - Testbed tunnel on {target_br} did not exist")
        return
    log_success(f"TEST 5 - Testbed tunnel pair exists for {source_br} and {target_br}")


def test_libvirt_guest_ssh(in_host, in_hostname, in_key_loc):
    try:
        # ignore key checking, we are just making local SSH connections in a test environment
        testbed_hosts_connections[in_host].run(f"ssh -i {in_key_loc} -o 'StrictHostKeyChecking no' -o 'UserKnownHostsFile /dev/null' -o ConnectTimeout=10 nocloud@{in_hostname} true")
        log_success(f"TEST 6 - Testbed guest {in_hostname} accessible via SSH")
    except invoke.exceptions.UnexpectedExit as e:
        # print(e)
        log_failure(f"TEST 6 - Testbed guest {in_hostname} not accessible via SSH")


def test_docker_guest_curl(in_host, in_hostname):
    try:
        # for docker guests, from the host we dont have hostname resolution so us ip
        testbed_hosts_connections[in_host].run(f"curl {in_hostname}")
        log_success(f"TEST 6 - Testbed guest {in_hostname} accessible via curl")
    except invoke.exceptions.UnexpectedExit as e:
        # print(e)
        log_failure(f"TEST 6 - Testbed guest {in_hostname} not accessible via curl")


def test_libvirt_pushed_artefacts(in_host, in_hostname, in_name, in_key_loc):
    try:
        print("testing for context folder in /etc/nocloud/")
        testbed_hosts_connections[in_host].run(f"ssh -i {in_key_loc} -o 'StrictHostKeyChecking no' -o 'UserKnownHostsFile /dev/null' -o ConnectTimeout=10 nocloud@{in_hostname} ls /etc/nocloud/context/kvm-compose.yaml")
    except invoke.exceptions.UnexpectedExit as e:
        # print(e)
        log_failure(f"TEST 7 - Testbed guest {in_hostname} did not have context pushed or not accessible")
        return
    try:
        print("testing for setup script in /tmp/")
        testbed_hosts_connections[in_host].run(f"ssh -i {in_key_loc} -o 'StrictHostKeyChecking no' -o 'UserKnownHostsFile /dev/null' -o ConnectTimeout=10 nocloud@{in_hostname} ls /tmp/{in_name}-setup.sh")
    except invoke.exceptions.UnexpectedExit as e:
        # print(e)
        log_failure(f"TEST 7 - Testbed guest {in_hostname} did not have setup script pushed")
        return
    log_success(f"TEST 7 - Testbed guest {in_hostname} had setup script pushed")


def test_docker_artefacts(in_host, in_hostname, in_conf):
    # must check each asset in the containers config

    # TODO - environment

    # TODO - env file

    # volumes
    for volume in in_conf["docker"]["volumes"]:
        # check if the folder exists
        try:
            testbed_hosts_connections[in_host].run(f"sudo docker exec -i {in_hostname} ls {volume['target']}")
        except invoke.exceptions.UnexpectedExit as e:
            # print(e)
            log_failure(f"TEST 7 - Testbed guest {in_hostname} did not have mount at {volume['target']}")
            return

    # TODO - devices

    log_success(f"TEST 7 - Testbed guest {in_hostname} had all artefacts mounted")


def test_libvirt_guest_communication(in_host, in_hostname, in_guest_list):
    # this is testing comms from a libvirt guest to any other guest type
    try:
        print(f"pushing guest ssh key to {in_hostname}")
        # must be run from main
        testbed_hosts_connections["testbed-host-one"].run(f"""rsync -av -e "ssh -i /home/nocloud/.ssh/id_ed25519_kvm -o 'StrictHostKeyChecking no' -o 'UserKnownHostsFile /dev/null' -o ConnectTimeout=10" /home/nocloud/.ssh/id_ed25519_kvm nocloud@{in_hostname}:/home/nocloud/.ssh/""")
    except invoke.exceptions.UnexpectedExit as e:
        # print(e)
        log_failure(f"TEST 8 - could not push ssh key to {in_hostname}")
        return

    # iterate through guest list and attempt communication
    for name, guest_data in in_guest_list.items():
        other_guest_type = get_guest_type(guest_data)

        match other_guest_type:
            case "libvirt":
                # skip guests that are turned off since they are backing images for linked clones
                if guest_data["is_golden_image"]:
                    print(f"skipping linked clone golden image {guest_data['libvirt']['hostname']}")
                    continue
                # don't try to ssh to self
                if guest_data["libvirt"]["hostname"] != in_hostname:
                    print(f"connecting to {guest_data['libvirt']['hostname']} from {in_hostname}")
                    # ssh from this computer into the testbed host, ssh into the guest on the testbed host,
                    # then ssh from the guest to another guest
                    try:
                        # must be run from main
                        testbed_hosts_connections["testbed-host-one"].run(f"""ssh -i /home/nocloud/.ssh/id_ed25519_kvm -o 'StrictHostKeyChecking no' -o 'UserKnownHostsFile /dev/null' -o ConnectTimeout=10 nocloud@{in_hostname} "ssh -i /home/nocloud/.ssh/id_ed25519_kvm -o 'StrictHostKeyChecking no' -o 'UserKnownHostsFile /dev/null' nocloud@{guest_data['libvirt']['hostname']} hostname" """)
                    except invoke.exceptions.UnexpectedExit as e:
                        # print(e)
                        log_failure(f"TEST 8 - Could not ssh from {in_hostname} to {guest_data['libvirt']['hostname']}")
                        return
            case "docker":
                # skip docker guests that are not up as they are the main config definition for scaled containers
                if guest_data["docker"]["scaling"]:
                    print(f"skipping container {guest_data['name']}")
                    continue

                print(f"connecting to {guest_data['name']} from {in_hostname}")
                # send a curl request to the container
                try:
                    # must be run from main
                    testbed_hosts_connections["testbed-host-one"].run(f"""ssh -i /home/nocloud/.ssh/id_ed25519_kvm -o 'StrictHostKeyChecking no' -o 'UserKnownHostsFile /dev/null' -o ConnectTimeout=10 nocloud@{in_hostname} "curl {guest_data['docker']['hostname']}" """)
                except invoke.exceptions.UnexpectedExit as e:
                    # print(e)
                    log_failure(f"TEST 8 - Could not curl from {in_hostname} to {guest_data['name']}")
                    return

    log_success(f"TEST 8 - All communication attempts from {in_hostname} to other guests worked")


def test_docker_guest_communications(in_host, in_hostname, in_guest_list):
    # iterate through guest list and attempt communication
    for name, guest_data in in_guest_list.items():
        other_guest_type = get_guest_type(guest_data)
        match other_guest_type:
            case "libvirt":
                # skip guests that are turned off since they are backing images for linked clones
                if guest_data["is_golden_image"]:
                    print(f"skipping linked clone golden image {guest_data['libvirt']['hostname']}")
                    continue
                print(f"connecting to {guest_data['libvirt']['hostname']} from {in_hostname}")
                # ssh from this computer into the testbed host, run command from container to the python http server
                # on the target libvirt guest that has been run in the background from the setup script
                try:
                    # must be run from testbed host with the container
                    testbed_hosts_connections[in_host].run(f"""sudo docker exec -i {in_hostname} curl {guest_data['libvirt']['hostname']}:8000""")
                except invoke.exceptions.UnexpectedExit as e:
                    # print(e)
                    log_failure(f"TEST 8 - Could not curl from {in_hostname} to {guest_data['libvirt']['hostname']}:8000")
                    return
            case "docker":
                # don't try to curl self
                if guest_data["docker"]["hostname"] != in_hostname:
                    if guest_data["docker"]["scaling"]:
                        print(f"skipping base docker image {guest_data['docker']['hostname']}")
                        continue
                    print(f"connecting to {guest_data['docker']['hostname']} from {in_hostname}")
                    # ssh from this computer into the testbed host, run command from container to the python http server
                    # on the target libvirt guest that has been run in the background from the setup script
                    try:
                        # must be run from testbed host with the container
                        testbed_hosts_connections[in_host].run(f"""sudo docker exec -i {in_hostname} curl {guest_data['docker']['hostname']}""")
                    except invoke.exceptions.UnexpectedExit as e:
                        # print(e)
                        log_failure(
                            f"TEST 8 - Could not curl from {in_hostname} to {guest_data['docker']['hostname']}")
                        return
    log_success(f"TEST 8 - All communication attempts from {in_hostname} to other guests worked")


def test_libvirt_guest_external_communication(in_host, in_hostname):
    try:
        print(f"trying to access the external web from guest '{in_hostname}'")
        testbed_hosts_connections[in_host].run(f"ssh -i /home/nocloud/.ssh/id_ed25519_kvm -o 'StrictHostKeyChecking no' -o 'UserKnownHostsFile /dev/null' -o ConnectTimeout=10 nocloud@{in_hostname} wget google.com")
    except invoke.exceptions.UnexpectedExit as e:
        # print(e)
        log_failure(f"TEST 9 - could not connect to external web with guest '{in_hostname}'")
        return
    log_success(f"TEST 9 - connected to external web with guest '{in_hostname}'")


def test_docker_guest_external_communication(in_host, in_hostname):
    try:
        print(f"trying to access the external web from guest '{in_hostname}'")
        testbed_hosts_connections[in_host].run(f"sudo docker exec -t {in_hostname} curl google.com")
    except invoke.exceptions.UnexpectedExit as e:
        # print(e)
        log_failure(f"TEST 9 - could not connect to external web with guest '{in_hostname}'")
        return
    log_success(f"TEST 9 - connected to external web with guest '{in_hostname}'")


# SETUP CODE
get_project_state()
state_json = read_project_state()
# set up a dictionary with ssh connections to the testbed hosts created for running test commands
testbed_hosts_connections = {}
for host in state_json["testbed_hosts"]:
    conn_string = f"{state_json['testbed_hosts'][host]['username']}@{state_json['testbed_hosts'][host]['hostname']}"
    # print(conn_string)
    testbed_hosts_connections.update({
        host: Connection(conn_string,
                         connect_kwargs={"key_filename": TESTBED_HOST_KEY},
                         gateway=None)
    })

# TEST CODE - see list of tests with numbers at top of script for test objectives


def get_guest_type(in_config):
    # takes in state_json["testbed_guests"][guest] and checks what type it is here
    if _ := in_config.get("libvirt"):
        return "libvirt"
    elif _ := in_config.get("docker"):
        return "docker"
    else:
        raise NotImplemented("guest type not found")


# test 1)
print(f"TEST 1: testing for existence of guests")
for guest in state_json["testbed_guests"]:
    guest_type = get_guest_type(state_json["testbed_guests"][guest])

    match guest_type:
        case "libvirt":
            guest_hostname = state_json["testbed_guests"][guest]["libvirt"]["hostname"]
            guest_host = state_json["testbed_guests"][guest]["testbed_host"]

            # skip guests that are turned off since they are backing images for linked clones
            if not state_json["testbed_guests"][guest]["is_golden_image"]:
                test_libvirt_guest_exists(guest_host, guest_hostname)
        case "docker":
            container_name = state_json["testbed_guests"][guest]["name"]
            guest_host = state_json["testbed_guests"][guest]["testbed_host"]

            if not state_json["testbed_guests"][guest]["docker"]["scaling"]:
                test_docker_guest_exists(guest_host, container_name)

# test 2)
print(f"TEST 2: testing libvirt network")
for host in state_json["testbed_hosts"]:
    if state_json["testbed_hosts"][host]["is_main_host"]:
        test_libvirt_network(state_json["testbed_hosts"][host]["hostname"])

# test 3)
host_to_bridge = []
for ovs_bridge in state_json["network"]["bridges"]:
    host_to_bridge.append((f"project-{ovs_bridge['name']}", ovs_bridge['testbed_host']))
test_ovs_bridges(host_to_bridge)

# test 4)

# test 4a)
print(f"TEST 4a: testing veth connection to libvirt network")
# the veth link between the project-prjbr0 (libvirt) and the "external ovs bridge (project-br0)"
# will be a pair project-veth1@project-veth0, where the connections:
# project-prjbr0 <= project-veth1@project-veth0 => project-br0
for host in state_json["testbed_hosts"]:
    if state_json["testbed_hosts"][host]["is_main_host"]:
        test_veth_connections_4a(state_json["testbed_hosts"][host]["hostname"])

# test 4b)
print(f"TEST 4b: testing veth connection between ovs bridges")
# the state json contains a list of bridge connections, here we test only type "ovs"
for bridge_pairs in state_json["network"]["physical_bridge_connections"]:
    ovs = state_json["network"]["physical_bridge_connections"][bridge_pairs].get("ovs")
    # if this bridge pair key is not "ovs" then this value will be None
    if ovs:
        test_veth_connections_4b(state_json["network"]["physical_bridge_connections"][bridge_pairs]["ovs"])

# test 5)
# We must test the tunnel on both sides, since the tunnel points to a unique IP address on the remote without
# a specific binding to a bridge on the remote - see the networking architecture documentation for more details.
# From "ovs-vsctl show" we will get all the bridges and ports etc, without doing various database lookups we can
# search for a substring since we have all the info needed in the state.json, section from ovs-vsctl show:
# | Port geneve0
# |             Interface geneve0
# |                 type: geneve
# |                 options: {key=br0br1, remote_ip="10.0.1.12"}
# If we can construct "options: {key=br0br1, remote_ip="10.0.1.12"}" we can for now say the tunnel worked
# as long as we can confirm for both sides this exists and the following guest communication tests work.
# This is because the 'key' will be unique between each tunnel in addition to the ip address specific to that
# tunnel pair
if len(state_json["testbed_hosts"]) > 1:
    print(f"TEST 5: testing tunnels")
    for bridge_pairs in state_json["network"]["physical_bridge_connections"]:
        tunnel = state_json["network"]["physical_bridge_connections"][bridge_pairs].get("tunnel")
        # if this bridge pair key is not "tunnel" then this value will be None
        if tunnel:
            test_geneve_tunnels(state_json["network"]["physical_bridge_connections"][bridge_pairs]["tunnel"])
else:
    print(f"TEST 5: not running as there is only one testbed host")


# test 6)
# to ssh to a guest, this must happen from the main since the main is the one with the DNS translation
print(f"TEST 6: testing guest is accessible through network from master host")
for guest in state_json["testbed_guests"]:
    guest_type = get_guest_type(state_json["testbed_guests"][guest])

    match guest_type:
        case "libvirt":
            guest_hostname = state_json["testbed_guests"][guest]["libvirt"]["hostname"]
            guest_key_location = state_json["testbed_guest_shared_config"]["ssh_private_key_location"]

            # skip guests that are turned off since they are backing images for linked clones
            if not state_json["testbed_guests"][guest]["is_golden_image"]:
                test_libvirt_guest_ssh("testbed-host-one", guest_hostname, guest_key_location)

        case "docker":
            # skip guests that are turned off since they are backing config for scaled containers
            if not state_json["testbed_guests"][guest]["docker"]["scaling"]:
                # use ip since no hostname resolution from host to guest for now
                test_docker_guest_curl("testbed-host-one", state_json["testbed_guests"][guest]['docker']['static_ip'])


# test 7)
print(f"TEST 7: testing artefacts have been pushed to guest")
for guest in state_json["testbed_guests"]:
    guest_type = get_guest_type(state_json["testbed_guests"][guest])

    match guest_type:
        case "libvirt":
            guest_hostname = state_json["testbed_guests"][guest]["libvirt"]["hostname"]
            guest_name = state_json["testbed_guests"][guest]["name"]
            guest_key_location = state_json["testbed_guest_shared_config"]["ssh_private_key_location"]

            # skip guests that are turned off since they are backing images for linked clones
            if not state_json["testbed_guests"][guest]["is_golden_image"]:
                test_libvirt_pushed_artefacts("testbed-host-one", guest_hostname, guest_name, guest_key_location)

        case "docker":
            # TODO - test the mounts and devices
            docker_hostname = state_json["testbed_guests"][guest]["docker"]["hostname"]
            testbed_host = state_json["testbed_guests"][guest]["testbed_host"]
            # skip containers that are backing for docker replicas
            if not state_json["testbed_guests"][guest]["docker"]["scaling"]:
                test_docker_artefacts(testbed_host, docker_hostname, state_json["testbed_guests"][guest])


# test 8)
print(f"TEST 8: testing guest communication")
# will use an ssh connection rather than a ping, as a ping can still work even if there are some networking issues
# that would have prevented SSH from working...
# using the shared ssh key, will try to ping from each guest to all other guests
for guest in state_json["testbed_guests"]:
    guest_type = get_guest_type(state_json["testbed_guests"][guest])

    match guest_type:
        case "libvirt":
            guest_hostname = state_json["testbed_guests"][guest]["libvirt"]["hostname"]
            guest_host = state_json["testbed_guests"][guest]["testbed_host"]

            # skip guests that are turned off since they are backing images for linked clones
            if not state_json["testbed_guests"][guest]["is_golden_image"]:
                test_libvirt_guest_communication(guest_host, guest_hostname, state_json["testbed_guests"])

        case "docker":
            guest_hostname = state_json["testbed_guests"][guest]["docker"]["hostname"]
            guest_host = state_json["testbed_guests"][guest]["testbed_host"]

            # skip containers that are backing for docker replicas
            if not state_json["testbed_guests"][guest]["docker"]["scaling"]:
                test_docker_guest_communications(guest_host, guest_hostname, state_json["testbed_guests"])

# test 9)
print(f"TEST 9: testing external guest communication")
for guest in state_json["testbed_guests"]:
    guest_type = get_guest_type(state_json["testbed_guests"][guest])

    match guest_type:
        case "libvirt":
            guest_hostname = state_json["testbed_guests"][guest]["libvirt"]["hostname"]

            # skip guests that are turned off since they are backing images for linked clones
            if not state_json["testbed_guests"][guest]["is_golden_image"]:
                test_libvirt_guest_external_communication("testbed-host-one", guest_hostname)

        case "docker":
            guest_hostname = "project-" + state_json["testbed_guests"][guest]["name"]
            testbed_host = state_json["testbed_guests"][guest]["testbed_host"]
            # skip guests that are turned off since they are backing images for linked clones
            if not state_json["testbed_guests"][guest]["docker"]["scaling"]:
                test_docker_guest_external_communication(testbed_host, guest_hostname)


# since we are done, rename the project-state.json in the artefacts folder with the kvm-compose-config name
# as it will be overwritten in any future tests - i.e. project-state-kvm-compose-config-2host.json
# TODO

# get number of hosts
n_hosts = len(state_json["testbed_hosts"])
print("="*10)
print(f"Test case results for test case '{TEST_CASE_NAME}' with {n_hosts} hosts:")
for test in test_case_results:
    print(test)
if failures_occurred:
    print(f"FAIL: Some test cases in '{TEST_CASE_NAME}' failed, search the failure message above (text after the test number) to find the error in the logs")
    exit(1)
else:
    print(f"SUCCESS: Test case '{TEST_CASE_NAME}' passed.")

# write the output into the artefacts folder
result_file = PROJECT_STATE_SAVE_LOCATION + f"{TEST_TIME}_test_case_results_{n_hosts}_hosts.txt"
f = open(result_file, "w")
for test in test_case_results:
    f.write(test)
f.close()
print(f"saved test case results in: {result_file}")
# make a copy of the project state with the same timestamp as test results
os.rename(CURRENT_PROJECT_STATE, TIMESTAMPED_PROJECT_STATE)

# clone ssh connections
for host, data in testbed_hosts_connections.items():
    data.close()
