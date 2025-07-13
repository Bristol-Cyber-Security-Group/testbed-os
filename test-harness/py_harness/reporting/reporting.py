import enum
from datetime import datetime
from typing import Optional, List
from config.host_images import BaseOperatingSystem
from test_cases.test_case import TestCase


class NHostReport:

    timestamp: datetime
    n_hosts: int

    create_linked_clone_hosts: Optional[bool]
    clear_linked_clone_hosts: Optional[bool]

    # direct test case results
    test_case_reports: List[TestCase]

    def __init__(self, n_hosts: int):
        self.timestamp = datetime.now()
        self.n_hosts = n_hosts
        self.test_case_reports = []
        self.create_linked_clone_hosts = False
        self.clear_linked_clone_hosts = False

    def to_dict(self) -> dict:
        return {
            "timestamp": str(self.timestamp.isoformat()),
            "n_hosts": self.n_hosts,
            "create_linked_clone_hosts": self.create_linked_clone_hosts,
            "clear_linked_clone_hosts": self.clear_linked_clone_hosts,
            "test_case_reports": [test_case_report.to_dict() for test_case_report in self.test_case_reports],
        }


class OSReport:

    operating_system: BaseOperatingSystem
    timestamp: datetime

    n_host_reports: List[NHostReport]

    # optional as they can be null if the test harness didn't reach this step
    create_base_host: Optional[bool]
    install_testbed: Optional[bool]

    def __init__(self, os: BaseOperatingSystem):
        self.timestamp = datetime.now()
        self.os = os
        self.n_host_reports = []
        self.create_base_host = None
        self.install_testbed = None

    def to_dict(self) -> dict:
        return {
            "timestamp": str(self.timestamp.isoformat()),
            "operating_system": self.os.name,
            "create_base_host": self.create_base_host,
            "install_testbed": self.install_testbed,
            "n_host_reports": [n_host_report.to_dict() for n_host_report in self.n_host_reports],
        }


class TestHarnessState(enum.Enum):
    """
    This enum represents whether the base infrastructure of the test harness was successful or not
    """

    # TODO - given the todo in main.py around failing cleanup, chance this from an enum to just bools

    # success will == 0, otherwise the other values will record where we failed
    SUCCESS = 0

    CREATE_WORKSPACE = 1
    CREATE_NETWORK = 2
    CLEAR_WORKSPACE = 3
    CLEAR_NETWORK = 4


class TestHarnessReport:
    """
    Contains all the reports
    """

    timestamp: datetime
    os_reports: List[OSReport]
    test_harness_state: TestHarnessState

    def __init__(self):
        self.timestamp = datetime.now()
        self.os_reports = []

    def to_dict(self) -> dict:
        return {
            "timestamp": str(self.timestamp.isoformat()),
            "test_harness_state": self.test_harness_state.value,
            "os_reports": [os_report.to_dict() for os_report in self.os_reports],
        }

    def get_success(self):
        # just check all stages of the report, if anything is in a failed state then fail the whole
        # test harness run

        # check test harness state
        if self.test_harness_state != TestHarnessState.SUCCESS:
            return False
        # then check the OS reports
        for os in self.os_reports:
            if os.create_base_host == False or os.install_testbed == False:
                return False
            # building base images okay, then now check the n host setup
            for n_host_report in os.n_host_reports:
                if n_host_report.create_linked_clone_hosts == False \
                        or n_host_report.clear_linked_clone_hosts == False:
                    return False
                # building clones okay, now check reports
                for test_case_report in n_host_report.test_case_reports:
                    if not test_case_report.success():
                        return False

        return True
