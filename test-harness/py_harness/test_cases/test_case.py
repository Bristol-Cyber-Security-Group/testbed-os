import time
import logging
import harness_settings
from datetime import datetime
from abc import ABC, abstractmethod
from typing import List, Optional
from host import LinkedCloneHost
from guest_control import ssh_command

# register the test cases here in the files they are written
registered_test_cases = []


class TestReport:

    success = False
    info: Optional[str]
    test_name: str
    timestamp: datetime

    def __init__(self, test_name: str, info: Optional[str] = None):
        self.test_name = test_name
        self.info = info
        self.timestamp = datetime.now()

    def to_dict(self) -> dict:
        return {
            "test_name": self.test_name,
            "success": self.success,
            "info": self.info,
            "timestamp": str(self.timestamp.isoformat()),
        }


class TestCase(ABC):

    """
    This class represents a test case. This should be inherited by each implementation for a test case, and
    implement ``test_case``.
    """

    test_case_name: str
    linked_clone_hosts: List[LinkedCloneHost]
    timestamp: datetime

    # results
    deploy_result: Optional[bool] = None
    test_result: Optional[List[TestReport]] = None
    destroy_result: Optional[bool] = None
    cleanup_result: Optional[bool] = None

    def __init__(self, test_case_name: str, linked_clone_hosts: List[LinkedCloneHost]):
        self.timestamp = datetime.now()
        self.test_case_name = test_case_name
        self.linked_clone_hosts = linked_clone_hosts
        self.test_result = []

    def run(self) -> bool:
        logging.info(f"Running test case '{self.test_case_name}'")

        logging.info(f"Deploying '{self.test_case_name}' test")
        self.deploy_result = deploy_test_case(self.test_case_name, self.linked_clone_hosts)
        if not self.deploy_result:
            return False

        # test case will return a list of TestReports, containing success and any info
        self.test_result.extend(self.test_case())

        logging.info(f"Destroying '{self.test_case_name}' test")
        self.destroy_result = destroy_test_case(self.test_case_name, self.linked_clone_hosts)
        if not self.destroy_result:
            return False

        logging.info(f"Clearing up artefacts for '{self.test_case_name}'")
        self.cleanup_result = clear_artefacts(self.test_case_name, self.linked_clone_hosts)
        if not self.cleanup_result:
            return False

        return True

    def success(self) -> bool:
        # check all the results, if any failed then return a false
        # test_result needs to have the inner tests checked
        all_test_result = all(item.success for item in self.test_result)
        return self.deploy_result and all_test_result and self.destroy_result and self.cleanup_result

    @abstractmethod
    def test_case(self) -> List[TestReport]:
        # test case to be implemented per test case, this should contain all runtime tests
        raise NotImplementedError()

    def to_dict(self):
        return {
            "timestamp": str(self.timestamp.isoformat()),
            "test_case_name": self.test_case_name,
            "deploy_result": self.deploy_result,
            "test_result": [res.to_dict() for res in self.test_result],
            "destroy_result": self.destroy_result,
            "cleanup_result": self.cleanup_result,
        }


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
    rm_state_json_result = ssh_command(f"cd {test_case} && rm {example_name}-state.json",
                              harness_settings.base_ssh_key,
                              linked_clone_hosts[0].hostname,  # first host will be main
                              )

    if rm_state_json_result.returncode != 0:
        return False

    return True
