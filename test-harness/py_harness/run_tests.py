import time
import logging
from typing import List
from host import LinkedCloneHost
from guest_control import ssh_command
import harness_settings


def run_tests(linked_clone_hosts: List[LinkedCloneHost]) -> bool:
    # this is the entrypoint to run all the integration tests
    logging.info("Running tests")

    # TODO - register the test cases either programmatically or statically

    # TODO - run the appropriate tests on each test case

    # TODO for now we have an example test case and test, this should be removed once we start to implement real tests
    logging.info("Deploying base test")
    if not deploy_test_case("base", linked_clone_hosts):
        return False

    logging.info("sleeping momentarily to simulate tests ...")
    time.sleep(10)

    logging.info("Destroying base test")
    if not destroy_test_case("base", linked_clone_hosts):
        return False

    return True


def deploy_test_case(example_name: str, linked_clone_hosts: List[LinkedCloneHost]) -> bool:
    # re-usable function to deploy a test harness test case on the testbed ...
    # the tests we want to use exist in the test harness folder, which will have the relevant kvm-compose.yaml files

    # get the location of the test, where the kvm-compose.yaml exists
    test_case = f"{harness_settings.test_case_location}/{example_name}"
    # make sure the testbed-server is running as a daemon on all hosts
    for host in linked_clone_hosts:
        logging.info("Making sure the testbed server is running on the hosts")
        ssh_command(f"sudo systemctl restart testbedos-server.service",
                    harness_settings.base_ssh_key,
                    host.hostname,
                    )
    # need to give the server a second to start before it can accept commands
    time.sleep(2)
    # on the main testbed, run the up command for the test case in the correct location
    logging.info(f"Run up command on {test_case}")
    up_result = ssh_command(f"cd {test_case} && kvm-compose up",
                harness_settings.base_ssh_key,
                linked_clone_hosts[0].hostname,  # first host will be main
                )

    if up_result.returncode != 0:
        return False
    return True

def destroy_test_case(example_name: str, linked_clone_hosts: List[LinkedCloneHost]) -> bool:
    # re-usable function to destroy a test harness test case on the testbed ...

    # get the location of the test, where the kvm-compose.yaml exists
    test_case = f"{harness_settings.test_case_location}/{example_name}"
    # on the main testbed, run the down command for the test case in the correct location
    logging.info(f"Run down command on {test_case}")
    down_result = ssh_command(f"cd {test_case} && kvm-compose down",
                            harness_settings.base_ssh_key,
                            linked_clone_hosts[0].hostname,  # first host will be main
                            )

    if down_result.returncode != 0:
        return False
    return True