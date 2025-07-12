from .test_case import TestCase, registered_test_cases
from .shared_runtime_tests import *
from .test_case import TestReport


class BaseTestCase(TestCase):

    def __init__(self, linked_clone_hosts: List[LinkedCloneHost]):
        super().__init__("base", linked_clone_hosts)

    def test_case(self) -> List[TestReport]:

        libvirt_guests = ["server", "client1", "client2"]
        docker_guests = ["nginx"]

        merged_guests = []
        merged_guests += libvirt_guests
        merged_guests += docker_guests

        up_result_report = check_if_guests_are_up(self.test_case_name, self.linked_clone_hosts, merged_guests)

        # try internet connectivity tests
        libvirt_net_test_report = check_internet_connectivity(self.test_case_name, self.linked_clone_hosts, merged_guests)


        return [
            up_result_report,
            libvirt_net_test_report,
        ]

registered_test_cases.append(BaseTestCase)
