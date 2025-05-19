import logging
import time
from typing import List
from host import LinkedCloneHost
from .test_case import TestCase, registered_test_cases


class BaseTestCase(TestCase):

    def __init__(self, linked_clone_hosts: List[LinkedCloneHost]):
        super().__init__("base", linked_clone_hosts)

    def test_case(self) -> bool:
        logging.info("sleeping momentarily to simulate tests ...")
        time.sleep(5)
        return True

registered_test_cases.append(BaseTestCase)
