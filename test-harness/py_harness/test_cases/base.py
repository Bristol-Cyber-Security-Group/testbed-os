import logging
import time
from typing import List
from host import LinkedCloneHost
from . import TestCase


class BaseTestCase(TestCase):

    def __init__(self, test_case_name: str, linked_clone_hosts: List[LinkedCloneHost]):
        super().__init__(test_case_name, linked_clone_hosts)

    def test_case(self) -> bool:
        logging.info("sleeping momentarily to simulate tests ...")
        time.sleep(5)
        return True
