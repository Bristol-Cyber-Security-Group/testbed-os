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

        ### try internet connectivity tests
        # test against internet, which implicitly tests the DNS resolution
        libvirt_net_test_report = check_internet_connectivity(self.test_case_name, self.linked_clone_hosts, merged_guests)
        # test internal connection between two libvirt guests on the same logical switch
        inter_libvirt_guest_connection = test_connection_between_guests(
            self.test_case_name,
            self.linked_clone_hosts,
            "client1",
            "server",
            "10.0.0.11:8000",
            "_both_libvirt",
            True,
        )
        # test internal connection between a libvirt guest and docker guest, where connection initiated from docker guest
        inter_docker_guest_connection = test_connection_between_guests(
            self.test_case_name,
            self.linked_clone_hosts,
            "nginx",
            "server",
            "10.0.0.11:8000",
            "_from_docker",
            True,
        )
        # TODO - to a docker guest

        return [
            up_result_report,
            libvirt_net_test_report,
            inter_libvirt_guest_connection,
            inter_docker_guest_connection,
        ]

registered_test_cases.append(BaseTestCase)
