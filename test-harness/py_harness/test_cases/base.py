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

        inter_guest_connection = test_connection_between_guests(
            self.test_case_name,
            self.linked_clone_hosts,
            "client1",
            "server",
            "10.0.0.11:8000",
            "curl between two libvirt guests",
        )

        return [
            up_result_report,
            libvirt_net_test_report,
            inter_guest_connection,
        ]

registered_test_cases.append(BaseTestCase)
