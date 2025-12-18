from .test_case import TestCase, registered_test_cases
from .shared_runtime_tests import *
from .test_case import TestReport


class OvnTestCase(TestCase):

    def __init__(self, linked_clone_hosts: List[LinkedCloneHost]):
        super().__init__("ovn", linked_clone_hosts)

    def test_case(self) -> List[TestReport]:

        libvirt_guests = ["client1"]
        docker_guests = ["client2", "client3"]
        merged_guests = []
        merged_guests += libvirt_guests
        merged_guests += docker_guests

        up_result_report = check_if_guests_are_up(self.test_case_name, self.linked_clone_hosts, merged_guests)

        # test curl from
        sw0_to_sw1 = test_connection_between_guests(
            self.test_case_name,
            self.linked_clone_hosts,
            "client1",
            "client3",
            "10.0.2.13",
            "_from_sw0_to_sw1",
            False,
        )

        sw1_to_sw0 = test_connection_between_guests(
            self.test_case_name,
            self.linked_clone_hosts,
            "client3",
            "client2",
            "10.0.0.11",
            "_from_sw1_to_sw0",
            False,
        )

        internet_test = confirm_no_internet_connectivity(self.test_case_name, self.linked_clone_hosts, merged_guests)

        consecutive_up = down_then_up(
            self.test_case_name,
            "_consecutive_up",
            self.linked_clone_hosts,
        )

        return [
            up_result_report,
            sw0_to_sw1,
            sw1_to_sw0,
            internet_test,
            consecutive_up,
        ]

registered_test_cases.append(OvnTestCase)
