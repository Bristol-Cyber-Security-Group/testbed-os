import subprocess
import time

from py_harness.harness_settings import test_case_location, base_ssh_key
from py_harness.config.host import LinkedCloneHost
from py_harness.config.guest_control import ssh_command
from .test_case import TestReport
from typing import List
import inspect


def check_if_libvirt_guests_are_up(
    test_case_name: str,
    linked_clone_hosts: List[LinkedCloneHost],
    libvirt_guest_names: List[str],
) -> TestReport:
    report = TestReport(inspect.currentframe().f_code.co_name)
    test_case = f"{test_case_location}/{test_case_name}"
    # for each guest up to a timeout, use the exec command to test if the guest can run the command, meaning it is
    # up and ready to be used - the command will only work if you can log in, which is a satisfactory 'ready' state
    timeout = 60  # seconds
    timeout_counter = 0
    timeout_increment = 5
    while True:
        # check if host is up, use pexpect timeout, if this keeps happening until timeout then host not up

        results = []
        # we will use the `ls` command to check if the host is up, if this times out then we are not logged in
        for guest_name in libvirt_guest_names:
            ls_result: subprocess.CompletedProcess = ssh_command(
                f"cd {test_case} && kvm-compose exec {guest_name} shell-command ls",
                base_ssh_key,
                linked_clone_hosts[0].hostname,  # first host will be main
                )
            results.append([guest_name, ls_result])

        # collect all three results, if all three up then break with test report
        all_ok = all(item[1].returncode == 0 for item in results)
        if all_ok:
            report.success = True
            report.info = "Libvirt guests are all up"
            return report

        time.sleep(timeout_increment)
        timeout_counter += timeout_increment

        if timeout_counter > timeout:
            # failed, set up test result failure
            report.success = False
            msg = ""
            for guest, err_msg in results:
                msg = msg + f"guest: {guest}, stderr: {err_msg.stderr}\n"
            report.info = msg
            return report


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
