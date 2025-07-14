import logging
import subprocess
import time

from harness_settings import test_case_location, base_ssh_key
from config.host import LinkedCloneHost
from config.guest_control import ssh_command, long_running_ssh_command
from .test_case import TestReport
from typing import List
import inspect


def check_if_guests_are_up(
    test_case_name: str,
    linked_clone_hosts: List[LinkedCloneHost],
    libvirt_guest_names: List[str],
) -> TestReport:
    report = TestReport(inspect.currentframe().f_code.co_name + "_" + test_case_name)
    logging.info(f"Running test {report.test_name}")

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
        # check if there are the right number of hosts up
        n_hosts_up = len(list(filter(lambda x: x[1].returncode == 0, results)))
        if all_ok and n_hosts_up == len(libvirt_guest_names):
            report.success = True
            report.info = f"The ({n_hosts_up}) guests are all up"
            return report

        time.sleep(timeout_increment)
        timeout_counter += timeout_increment

        if timeout_counter > timeout:
            # failed, set up test result failure
            report.success = False
            msg = f"There were ({n_hosts_up}) up, "
            for guest, err_msg in results:
                msg = msg + f"guest: {guest}, stderr: {err_msg.stderr}\n"
            report.info = msg
            return report


def check_internet_connectivity(
    test_case_name: str,
    linked_clone_hosts: List[LinkedCloneHost],
    guest_names: List[str],
) -> TestReport:
    report = TestReport(inspect.currentframe().f_code.co_name + "_" + test_case_name)
    logging.info(f"Running test {report.test_name}")

    test_case = f"{test_case_location}/{test_case_name}"
    # for each guest supplied (by names in the yaml file), try to curl google as a network test
    results = []
    for guest_name in guest_names:
        curl_result: subprocess.CompletedProcess = ssh_command(f"cd {test_case} && kvm-compose exec {guest_name} shell-command curl google.com",
                            base_ssh_key,
                            linked_clone_hosts[0].hostname,  # first host will be main
                            )
        results.append([guest_name, curl_result])

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


def test_connection_between_guests(
    test_case_name: str,
    linked_clone_hosts: List[LinkedCloneHost],
    from_guest: str,
    to_guest: str,
    to_guest_ip_and_port: str,
    test_report_name: str,
    start_python_server: bool,
) -> TestReport:
    # send a curl request from one guest to another
    report = TestReport(inspect.currentframe().f_code.co_name + test_report_name + "_" + test_case_name)
    logging.info(f"Running test {report.test_name}")

    test_case = f"{test_case_location}/{test_case_name}"
    if start_python_server:
        python_server_process: subprocess.Popen[str] = long_running_ssh_command(
            f"cd {test_case} && kvm-compose exec {to_guest} shell-command python3 -m http.server",
            base_ssh_key,
            linked_clone_hosts[0].hostname,  # first host will be main
            )

        # wait a second to let the command and server start
        time.sleep(5)
    # run the curl from the other guest
    curl_process: subprocess.CompletedProcess = ssh_command(
        f"cd {test_case} && kvm-compose exec {from_guest} shell-command curl {to_guest_ip_and_port}",
        base_ssh_key,
        linked_clone_hosts[0].hostname,  # first host will be main
        )

    if start_python_server:
        # consume the lines to get to the statuscode
        list(python_server_process.stdout.readlines())

        python_server_process.terminate()

    report.success = True if curl_process.returncode == 0 else False
    report.info = ""
    return report


def run_command(
    target_guest: str,
    command: str,
    test_case_name: str,
    linked_clone_hosts: List[LinkedCloneHost],
    test_report_name: str,
) -> TestReport:
    # run a command on a guest
    report = TestReport(inspect.currentframe().f_code.co_name + test_report_name + "_" + test_case_name)
    logging.info(f"Running test {report.test_name}")

    test_case = f"{test_case_location}/{test_case_name}"
    command_process: subprocess.CompletedProcess = ssh_command(
        f"cd {test_case} && kvm-compose exec {target_guest} shell-command {command}",
        base_ssh_key,
        linked_clone_hosts[0].hostname,  # first host will be main
        )

    report.success = True if command_process.returncode == 0 else False
    report.info = ""
    return report
