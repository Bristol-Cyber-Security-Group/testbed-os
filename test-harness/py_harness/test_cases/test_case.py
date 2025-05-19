import time
import logging
import harness_settings
from abc import ABC, abstractmethod
from typing import List, Optional
from host import LinkedCloneHost
from guest_control import ssh_command

# register the test cases here in the files they are written
registered_test_cases = []


class TestCase(ABC):

    """
    This class represents a test case. This should be inherited by each implementation for a test case, and
    implement ``test_case``.
    """

    test_case_name: str
    linked_clone_hosts: List[LinkedCloneHost]

    # results
    deploy_result: Optional[bool] = None
    test_result: Optional[bool] = None
    destroy_result: Optional[bool] = None
    cleanup_result: Optional[bool] = None

    def __init__(self, test_case_name: str, linked_clone_hosts: List[LinkedCloneHost]):
        self.test_case_name = test_case_name
        self.linked_clone_hosts = linked_clone_hosts

    def run(self) -> bool:
        logging.info(f"Running test case '{self.test_case_name}'")

        logging.info(f"Deploying '{self.test_case_name}' test")
        self.deploy_result = deploy_test_case("base", self.linked_clone_hosts)
        if not self.deploy_result:
            return False

        # subclass will implement the runtime tests
        self.test_result = self.test_case()

        logging.info(f"Destroying '{self.test_case_name}' test")
        self.destroy_result = destroy_test_case("base", self.linked_clone_hosts)
        if not self.destroy_result:
            return False

        logging.info(f"Clearing up artefacts for '{self.test_case_name}'")
        self.cleanup_result = clear_artefacts("base", self.linked_clone_hosts)
        if not self.cleanup_result:
            return False

        return self.test_result

    @abstractmethod
    def test_case(self) -> bool:
        # test case to be implemented per test case, this should contain all runtime tests
        raise NotImplementedError()


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
    logging.info(f"Run up command on '{test_case}'")
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
    logging.info(f"Run down command on '{test_case}'")
    down_result = ssh_command(f"cd {test_case} && kvm-compose down",
                            harness_settings.base_ssh_key,
                            linked_clone_hosts[0].hostname,  # first host will be main
                            )

    if down_result.returncode != 0:
        return False
    return True

def clear_artefacts(example_name: str, linked_clone_hosts: List[LinkedCloneHost]) -> bool:
    # re-usable function to remove the test case artefacts (vm images etc) once the test case has been destroyed

    # get the location of the test, where the kvm-compose.yaml exists
    test_case = f"{harness_settings.test_case_location}/{example_name}"

    # on the main testbed, run the clear artefacts command for the test case in the correct location
    logging.info(f"Run clear-artefacts command on '{test_case}'")
    clear_artefacts_result = ssh_command(f"cd {test_case} && kvm-compose clear-artefacts",
                              harness_settings.base_ssh_key,
                              linked_clone_hosts[0].hostname,  # first host will be main
                              )

    # TODO - if this fails, should we still try to remote the state json? relevant for reporting as well
    if clear_artefacts_result.returncode != 0:
        return False

    # finally also remove the state json, as this would prevent future up commands from working
    logging.info(f"Remove state json for '{test_case}'")
    rm_state_json_result = ssh_command(f"cd {test_case} && rm {test_case}-state.json",
                              harness_settings.base_ssh_key,
                              linked_clone_hosts[0].hostname,  # first host will be main
                              )

    if rm_state_json_result.returncode != 0:
        return False

    return True