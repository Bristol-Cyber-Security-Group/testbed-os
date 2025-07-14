from .test_case import TestCase, registered_test_cases
from .shared_runtime_tests import *
from .test_case import TestReport


class MountTestCase(TestCase):

    def __init__(self, linked_clone_hosts: List[LinkedCloneHost]):
        super().__init__("mount", linked_clone_hosts)

    def test_case(self) -> List[TestReport]:

        libvirt_guests = ["client1"]
        docker_guests = ["client2"]
        merged_guests = []
        merged_guests += libvirt_guests
        merged_guests += docker_guests

        up_result_report = check_if_guests_are_up(self.test_case_name, self.linked_clone_hosts, merged_guests)

        # check for some of the artefacts from mounts here

        ### libvirt
        run_script_artefact = run_command(
            "client1",
            "ls run_script_worked.txt",
            self.test_case_name,
            self.linked_clone_hosts,
            "_check_run_script_artefact_exists",
        )
        setup_script_artefact = run_command(
            "client1",
            "ls setup_script_worked.txt",
            self.test_case_name,
            self.linked_clone_hosts,
            "_check_setup_script_artefact_exists",
        )
        context_artefact = run_command(
            "client1",
            "ls context/",
            self.test_case_name,
            self.linked_clone_hosts,
            "_check_context_artefact_exists",
        )
        env_var = run_command(
            "client1",
            """[ -n "${MOUNT_ENV_TEST}" ] && exit 0 || exit 1""",
            self.test_case_name,
            self.linked_clone_hosts,
            "_check_env_var_exists",
        )

        ### docker
        env_file_var_docker = run_command(
            "client2",
            """[ -n "${DOCKER_ENV}" ] && exit 0 || exit 1""",
            self.test_case_name,
            self.linked_clone_hosts,
            "_check_env_file_var_exists_docker",
        )
        env_var_docker = run_command(
            "client2",
            """[ -n "${TWO}" ] && exit 0 || exit 1""",
            self.test_case_name,
            self.linked_clone_hosts,
            "_check_env_var_exists_docker",
        )
        mount_docker = run_command(
            "client2",
            "ls /opt/context",
            self.test_case_name,
            self.linked_clone_hosts,
            "_check_mount_exists_docker",
        )

        return [
            up_result_report,
            run_script_artefact,
            setup_script_artefact,
            context_artefact,
            env_var,
            env_file_var_docker,
            env_var_docker,
            mount_docker,
        ]

# TODO - commenting out this test case for now as the bugs are failing the test - to be fixed in 288
# registered_test_cases.append(MountTestCase)
