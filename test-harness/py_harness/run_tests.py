import logging
from typing import List
from py_harness.config.host import LinkedCloneHost
from test_cases.test_case import TestCase, registered_test_cases


def run_tests(linked_clone_hosts: List[LinkedCloneHost]) -> (bool, List[TestCase]):
    # this is the entrypoint to run all the integration tests
    logging.info("Running tests")

    # Every test case will self-register into `registered_test_cases`, and this loop just instantiates the test with the
    # list of hosts for this infrastructure deployment
    executed_test_cases: List[TestCase] = []
    # iterate through the registered classes
    for test_case in registered_test_cases:
        # instantiate the class with the list of hosts
        instantiated_test_case: TestCase = test_case(linked_clone_hosts)
        # run and get result of the tests
        test_case_result = instantiated_test_case.run()
        # store the test case, which has all the results for this test case
        executed_test_cases.append(instantiated_test_case)

    # TODO - collate the test case results

    # for all test cases, check if any have a failed result so that we can change the test harness outcome exit code
    # we call .success() which will return False only if one of the test cases failed
    all_results = all([tt.success() for tt in executed_test_cases])

    # TODO - return the report with the success/fail value of all_results, to then be checked later by the test harness

    return all_results, executed_test_cases

