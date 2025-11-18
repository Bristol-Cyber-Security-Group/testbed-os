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
        # this is here so that we don't run the tests following before cloud-init finishes, meaning the context folder
        # might not be available yet for example
        cloud_init_wait = run_command(
            "client1",
            "timeout 300 cloud-init status --wait",
            self.test_case_name,
            self.linked_clone_hosts,
            "_ensure_cloud_init_finished_running",
        )
        # the context folder should be made, but also check if a file inside it exists to catch any bugs relating to
        # zipping the context payload to be mounted into the guest
        context_artefact = run_command(
            "client1",
            "ls /etc/nocloud/context/context.txt",
            self.test_case_name,
            self.linked_clone_hosts,
            "_check_context_artefact_exists",
        )
        # this checks against the cloud-init environment tooling, we have not configured this to load permanently as
        # that would require a VM restart after placing into /etc/environment
        # ... this string here is a bit weird, needing to work around the limitations of how the kvm-compose exec
        # command and python escape strings work - you need to send a string so that the $ isn't evaluated before being
        # sent to the guest
        env_var = run_command(
            "client1",
            r""" "test \$(cloud-init query ds.meta_data.environment.MOUNT_ENV_TEST) = 123" """,
            self.test_case_name,
            self.linked_clone_hosts,
            "_check_env_var_exists",
        )

        ## docker
        # env_file_var_docker = run_command(
        #     "client2",
        #     """[ -n "${DOCKER_ENV}" ] && exit 0 || exit 1""",
        #     self.test_case_name,
        #     self.linked_clone_hosts,
        #     "_check_env_file_var_exists_docker",
        # )
        env_var_docker = run_command(
            "client2",
            """'test $TWO = 2'""",
            self.test_case_name,
            self.linked_clone_hosts,
            "_check_env_var_exists_docker",
        )
        # mount_docker = run_command(
        #     "client2",
        #     "ls /opt/context",
        #     self.test_case_name,
        #     self.linked_clone_hosts,
        #     "_check_mount_exists_docker",
        # )

        return [
            up_result_report,
            run_script_artefact,
            setup_script_artefact,
            cloud_init_wait,
            context_artefact,
            env_var,
            # env_file_var_docker,
            env_var_docker,
            # mount_docker,
        ]

registered_test_cases.append(MountTestCase)
