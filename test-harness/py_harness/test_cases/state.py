import json
from subprocess import CompletedProcess

from .test_case import TestCase, registered_test_cases
from .shared_runtime_tests import *
from .test_case import TestReport

def run_cli_command(
    test_case_name: str,
    linked_clone_hosts: List[LinkedCloneHost],
    cmd: str,
) -> CompletedProcess:


    test_case = f"{test_case_location}/{test_case_name}"
    command_process: subprocess.CompletedProcess = ssh_command(
        f"cd {test_case} && {cmd}",
        base_ssh_key,
        linked_clone_hosts[0].hostname,  # first host will be main
        True,
    )

    return command_process

def state_does_not_exist(
    test_case_name: str,
    test_report_name: str,
    linked_clone_hosts: List[LinkedCloneHost],
) -> TestReport:
    report = TestReport(inspect.currentframe().f_code.co_name + test_report_name + "_" + test_case_name)
    logging.info(f"Running test {report.test_name}")

    # we are running this on a deployment that does not exist in this test case, called dne
    state_does_not_exist_result = run_cli_command(
        test_case_name,
        linked_clone_hosts,
        "curl -s -o /dev/null -w \"%{http_code}\" localhost:3355/api/state/dne",
    )
    # for now since the server has not got proper error codes, it is either 200 or 500
    if state_does_not_exist_result.stdout != 200:
        report.success = True
    else:
        report.success = False
    report.info = str(state_does_not_exist_result.stdout)
    return report

def state_exists(
    test_case_name: str,
    test_report_name: str,
    linked_clone_hosts: List[LinkedCloneHost],
) -> TestReport:
    report = TestReport(inspect.currentframe().f_code.co_name + test_report_name + "_" + test_case_name)
    logging.info(f"Running test {report.test_name}")

    # make sure the code is 200 (underscore to prevent shadowing)
    state_exists_ = run_cli_command(
        test_case_name,
        linked_clone_hosts,
        "curl -s -o /dev/null -w \"%{http_code}\" localhost:3355/api/state/state",
    )
    # make sure that the json returns an "up" status for the deployment
    state_body = run_cli_command(
        test_case_name,
        linked_clone_hosts,
        "curl localhost:3355/api/state/state",
    )
    json_body = json.loads(state_body.stdout.strip())

    if state_exists_.stdout == b'200' and json_body["status"] == "up":
        report.success = True
        report.info = ""
    else:
        report.success = False
        report.info = f"http_code: {state_exists_.stdout}, status: {json_body['status']}"

    return report

def partial_off(
    test_case_name: str,
    test_report_name: str,
    linked_clone_hosts: List[LinkedCloneHost],
) -> TestReport:
    report = TestReport(inspect.currentframe().f_code.co_name + test_report_name + "_" + test_case_name)
    logging.info(f"Running test {report.test_name}")

    # switch off a vm
    run_cli_command(
        test_case_name,
        linked_clone_hosts,
        "sudo virsh destroy state-client1",
    )
    # make sure the code is 200
    partial_state_exists = run_cli_command(
        test_case_name,
        linked_clone_hosts,
        "curl -s -o /dev/null -w \"%{http_code}\" localhost:3355/api/state/state",
    )
    # make sure that the json returns an "partial" status for the deployment
    state_body = run_cli_command(
        test_case_name,
        linked_clone_hosts,
        "curl localhost:3355/api/state/state",
    )
    json_body = json.loads(state_body.stdout.strip())

    if partial_state_exists.stdout == b'200' and json_body["status"] == "partial":
        report.success = True
        report.info = ""
    else:
        report.success = False
        report.info = f"http_code: {partial_state_exists.stdout}, status: {json_body['status']}"

    return report

def state_down(
    test_case_name: str,
    test_report_name: str,
    linked_clone_hosts: List[LinkedCloneHost],
) -> TestReport:
    test_case = f"{test_case_location}/{test_case_name}"
    report = TestReport(inspect.currentframe().f_code.co_name + test_report_name + "_" + test_case_name)
    logging.info(f"Running test {report.test_name}")

    down_result: subprocess.CompletedProcess = ssh_command(
        f"cd {test_case} && kvm-compose down",
        base_ssh_key,
        linked_clone_hosts[0].hostname,  # first host will be main
    )

    if down_result.returncode != 0:
        report.success = False
        report.info = "down failed"
        return report

    # make sure the code is 200
    down_state_exists = run_cli_command(
        test_case_name,
        linked_clone_hosts,
        "curl -s -o /dev/null -w \"%{http_code}\" localhost:3355/api/state/state",
    )
    # make sure that the json returns an "partial" status for the deployment
    state_body = run_cli_command(
        test_case_name,
        linked_clone_hosts,
        "curl localhost:3355/api/state/state",
    )
    json_body = json.loads(state_body.stdout.strip())

    if down_state_exists.stdout == b'200' and json_body["status"] == "down":
        report.success = True
        report.info = ""
    else:
        report.success = False
        report.info = f"http_code: {down_state_exists.stdout}, status: {json_body['status']}"

    return report



class StateTestCase(TestCase):

    def __init__(self, linked_clone_hosts: List[LinkedCloneHost]):
            super().__init__("state", linked_clone_hosts)

    def test_case(self) -> List[TestReport]:

        libvirt_guests = ["client1"]
        docker_guests = ["client2", "client3"]
        merged_guests = []
        merged_guests += libvirt_guests
        merged_guests += docker_guests

        # before a deployment, expect errors on deployment state and individual
        state_does_not_exist_report = state_does_not_exist(
            self.test_case_name,
            "_assert_does_not_exist",
            self.linked_clone_hosts,
        )

        # after deployment, expect all components to be up
        state_exists_report = state_exists(
            self.test_case_name,
            "_assert_exists",
            self.linked_clone_hosts,
        )

        # todo run non-destructive commands in parallel, expect no problem

        # todo run destructive commands in parallel, expect second in order to error
        # todo run destructive test (above), expect state endpoint to report running state in progress

        # todo deploy another deployment in parallel, expect the different deployments to not interfere

        # after turning off one virtual machine and destroy some ovn components, expect partial up
        partial_off_report = partial_off(
            self.test_case_name,
            "_assert_partial_state",
            self.linked_clone_hosts,
        )

        # after a down, expect deployment state to show down
        state_down_report = state_down(
            self.test_case_name,
            "_assert_down_state",
            self.linked_clone_hosts,
        )

        return [
            state_does_not_exist_report,
            state_exists_report,

            partial_off_report,
            state_down_report,
        ]



registered_test_cases.append(StateTestCase)
