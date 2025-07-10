import subprocess
from py_harness.harness_settings import test_case_location, base_ssh_key
from py_harness.config.host import LinkedCloneHost
from py_harness.config.guest_control import ssh_command
from .test_case import TestReport
from typing import List
import inspect


def check_internet_connectivity(
        test_case_name: str,
        linked_clone_hosts: List[LinkedCloneHost],
        guest_names: List[str],
) -> TestReport:
    test_case = f"{test_case_location}/{test_case_name}"
    # for each guest supplied (by names in the yaml file), try to curl google as a network test
    results = []
    for guest_name in guest_names:
        curl_result: subprocess.CompletedProcess = ssh_command(f"cd {test_case} && kvm-compose exec {guest_name} shell-command curl google.com",
                            base_ssh_key,
                            linked_clone_hosts[0].hostname,  # first host will be main
                            )
        results.append([guest_name, curl_result])

    report = TestReport(inspect.currentframe().f_code.co_name)
    # check test results
    all_ok = all(item[1].returncode == 0 for item in results)
    report.success = all_ok
    if not all_ok:
        # store the error message for the failed test(s)
        msg = ""
        for guest, err_msg in results:
            msg = msg + f"guest: {guest}, stderr: {err_msg.stderr}\n"
        report.info = msg

    return report
