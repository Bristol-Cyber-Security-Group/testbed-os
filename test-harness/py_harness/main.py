import json
import shutil
import sys
import time

import libvirt
import logging
import harness_settings
from pathlib import Path
from host_images import BaseOperatingSystem
from harness_network import HarnessNetwork
from host import BaseHost, LinkedCloneHost
from run_tests import run_tests
from reporting import TestHarnessReport, TestHarnessState, OSReport, NHostReport


def get_libvirt_connection() -> libvirt.virConnect:
    try:
        libvirt_conn = libvirt.open('qemu:///system')

        logging.info("Connected to libvirt")
        return libvirt_conn

    except libvirt.libvirtError as e:
        logging.error(f"Connection error: {e}"
              f"\n\nThe test harness needs a connection to the host's libvirt to continue")
        sys.exit(1)


def main(connection: libvirt.virConnect) -> TestHarnessReport:
    # TODO initialise report - capture the different stages that will follow and accept if/when/where there is a failure
    #  and also how/when to capture the early terminations due to failure gracefully

    test_harness_report = TestHarnessReport()

    # initialise working area for the test harness in the libvirt images folder
    try:
        workspace_folder = Path(harness_settings.workspace)
        if not workspace_folder.exists():
            workspace_folder.mkdir(parents=True, exist_ok=True)
        # TODO make sure workspace is clean before starting, but do we re-use images downloaded?
    except OSError as e:
        logging.error(f"Error creating workspace folder: {e}")
        # set test harness state to failed workspace
        test_harness_report.test_harness_state = TestHarnessState.CREATE_WORKSPACE
        return test_harness_report


    # set up the libvirt network for the test harness
    harness_network = HarnessNetwork(connection)
    # don't reset the network if dev mode is on (assuming network was made before), otherwise down then up
    network_result = harness_network.ensure_on() if harness_settings.dev_mode else harness_network.reload()
    if not network_result:
        # there was a problem in creating the network for this instance of the test harness
        logging.error("Failed to establish a libvirt network for the test harness, cannot continue")
        # TODO log network error in report
        test_harness_report.test_harness_state = TestHarnessState.CREATE_NETWORK
        return test_harness_report

    test_harness_report.test_harness_state = TestHarnessState.SUCCESS

    # only continue if the network creation was successful
    if network_result:
        # for each base image type, run test harness
        for base_os in BaseOperatingSystem:
            logging.info(f"Testing on base image: {base_os.name}")

            # init report wrapper for this run, add to parent report
            os_report = OSReport(base_os)
            test_harness_report.os_reports.append(os_report)

            # we will pre-prepare the base host and clone data before continuing, we will only use the number of clone
            # hosts we need per n_hosts iteration later on - this does not yet create the VMs
            base_host = BaseHost(conn, base_os)
            linked_clone_hosts = []
            for n_host in range(1, harness_settings.max_n_hosts + 1):
                linked_clone_hosts.append(LinkedCloneHost(conn, base_host, n_host))
            # now we know what will be built, we can check to make sure there is nothing that will block the progression
            # of the following by making sure things don't exist ...
            # first make sure the clones don't exist, as these would prevent the base host from starting
            for host in linked_clone_hosts:
                host.ensure_destroyed()
            # now we can make sure the base doesn't exist, but if in dev mode we will leave it there
            if not harness_settings.dev_mode and base_host.exists():
                base_host.ensure_destroyed()

            # now we can start the process of setting up the base VM and install the testbed, if it exists still that
            # is because dev mode has allowed it and we just continue
            base_host_exists = base_host.exists()  # this contains the libvirt reference to the domain
            if base_host_exists is None:
                create_base_host_result = base_host.create()
                if not create_base_host_result:
                    # report failed result
                    os_report.create_base_host = False

                    # go to next test
                    continue
            elif not base_host_exists.isActive():
                # base host already exists, due to dev mode so just start it
                base_host.start()
                base_host.check_if_ready(harness_settings.base_ssh_key)
            os_report.create_base_host = True

            # install testbed code, in dev mode this just re-runs the ansible on top of the existing install
            install_best_host_result = base_host.install_testbed()
            if not install_best_host_result:
                # report failure, and where in the installation it failed
                os_report.install_testbed = False
                continue
            else:
                os_report.install_testbed = True

            # turn off base host before creating linked clones
            base_host.stop()
            # sleep a bit, the VM won't shut down quickly enough as the libvirt shutdown command is non-blocking
            # TODO - check with libvirt directly for off status
            time.sleep(5)

            # TODO check if the base host has turned off

            # begin n number of host loop, we will assign the linked clone hosts a number from 1 to 3, where the first
            # host will be the 'main' host in a cluster deployment if there is more than one host
            for n_hosts in range(1, harness_settings.max_n_hosts + 1):
                logging.info(f"Testing on {n_hosts} hosts")

                # create the report for this n hosts, add to parent report
                n_host_report = NHostReport(n_hosts)
                os_report.n_host_reports.append(n_host_report)

                # create n number of hosts
                create_results = []
                for linked_clone_host in linked_clone_hosts[0:n_hosts]:
                    logging.info(f"Creating linked clone host: {linked_clone_host.name}")
                    create_results.append(linked_clone_host.create())
                # if creating linked clones failed
                if not all(create_results):
                    logging.error(f"Failed to create linked clone hosts")
                    # cannot continue with this run, report and move on
                    n_host_report.create_linked_clone_hosts = False
                    continue
                else:
                    n_host_report.create_linked_clone_hosts = True


                # TODO - configure testbed settings, first host will be 'main' for cluster mode, the others should be
                #  in 'client' mode, pointing to the 'main' testbed host


                # TODO run all test cases, for now we will re-use the same guests for the whole test suite
                logging.info("Begin integration tests")
                run_tests_result, test_case_results = run_tests(linked_clone_hosts)
                if not run_tests_result:
                    logging.error(f"Test suite failed")
                else:
                    logging.info(f"Test suite succeeded")
                # collect report for tests
                n_host_report.test_case_reports.extend(test_case_results)

                # clean up linked clones
                destroy_results = []
                for linked_clone_host in linked_clone_hosts:
                    logging.info(f"Destroying linked clone host: {linked_clone_host.name}")
                    destroy_results.append(linked_clone_host.ensure_destroyed())
                # if destroying any linked clones failed
                if not all(destroy_results):
                    logging.error(f"Failed to destroy linked clone hosts")
                    n_host_report.clear_linked_clone_hosts = False
                else:
                    n_host_report.clear_linked_clone_hosts = True


    # clean up the test harness working area in the libvirt images folder
    if not harness_settings.dev_mode:
        try:
            shutil.rmtree(workspace_folder)
        except OSError as e:
            logging.error(f"Error deleting workspace folder: {e}")
            test_harness_report.test_harness_state = TestHarnessState.CLEAR_WORKSPACE
            return test_harness_report

    # TODO - we might not have been able to clear the workspace, but we might have been able to clear the network, and
    #  ideally we should despite the workspace failing to clear, but at the moment we wont try as the previous would
    #  return on a failure

    # we are done with the test harness network, we can destroy
    if not harness_settings.dev_mode:
        try:
            harness_network.net_destroy()
            harness_network.net_undefine()
        except libvirt.libvirtError as e:
            logging.error(f"Error destroying network: {e}")
            test_harness_report.test_harness_state = TestHarnessState.CLEAR_NETWORK
            return test_harness_report

    return test_harness_report


if __name__ == '__main__':
    # TODO dev/debug mode where it pauses on failed test to allow inspection

    logging.info(f"Starting harness with ID: {harness_settings.test_id}")
    logging.info(f"Workspace: {harness_settings.workspace}")

    logging.info("Getting libvirt connection")
    conn = get_libvirt_connection()

    # run test harness
    result_report = main(conn)
    logging.info(f"REPORT:\n{json.dumps(result_report.to_dict(), indent=4)}")

    logging.info("Closing connection to libvirt")
    conn.close()
    logging.info("End of test harness")

    if not result_report.get_success():
        sys.exit(1)
