import logging
from typing import List
from host import LinkedCloneHost
from test_cases.base import BaseTestCase


def run_tests(linked_clone_hosts: List[LinkedCloneHost]) -> bool:
    # this is the entrypoint to run all the integration tests
    logging.info("Running tests")

    # TODO - register the test cases either programmatically or statically

    # TODO - run the appropriate tests on each test case

    # TODO for now we have an example test case and test, this should be removed once we start to implement real tests
    base_test = BaseTestCase("base", linked_clone_hosts)
    base_test_result = base_test.run()
    if not base_test_result:
        return False

    return True

