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


def get_libvirt_connection() -> libvirt.virConnect:
    try:
        libvirt_conn = libvirt.open('qemu:///system')

        logging.info("Connected to libvirt")
        return libvirt_conn

    except libvirt.libvirtError as e:
        logging.error(f"Connection error: {e}"
              f"\n\nThe test harness needs a connection to the host's libvirt to continue")
        sys.exit(1)


def main(connection: libvirt.virConnect) -> bool:
    # TODO initialise report - capture the different stages that will follow and accept if/when/where there is a failure
    #  and also how/when to capture the early terminations due to failure gracefully

    # initialise working area for the test harness in the libvirt images folder
    workspace_folder = Path(harness_settings.workspace)
    if not workspace_folder.exists():
        workspace_folder.mkdir(parents=True, exist_ok=True)
    # TODO make sure workspace is clean before starting, but do we re-use images downloaded?

    # set up the libvirt network for the test harness
    harness_network = HarnessNetwork(connection)
    # don't reset the network if dev mode is on (assuming network was made before), otherwise down then up
    network_result = harness_network.ensure_on() if harness_settings.dev_mode else harness_network.reload()
    if not network_result:
        # there was a problem in creating the network for this instance of the test harness
        logging.error("Failed to establish a libvirt network for the test harness, cannot continue")
        # TODO log network error in report

    # only continue if the network creation was successful
    if network_result:
        # for each base image type, run test harness
        for base_os in BaseOperatingSystem:
            logging.info(f"Testing on base image: {base_os.name}")

            # TODO - init report wrapper for this run

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
                    # TODO - report failed result

                    # go to next test
                    continue
            elif not base_host_exists.isActive():
                # base host already exists, due to dev mode so just start it
                base_host.start()
                base_host.check_if_ready(harness_settings.base_ssh_key)

            # install testbed code, in dev mode this just re-runs the ansible on top of the existing install
            install_best_host_result = base_host.install_testbed()
            if not install_best_host_result:
                # TODO report failure, and where in the install it failed
                continue

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

                # create n number of hosts
                for linked_clone_host in linked_clone_hosts[0:n_hosts]:
                    logging.info(f"Creating linked clone host: {linked_clone_host.name}")
                    clone_create_result = linked_clone_host.create()

                # TODO - if creating linked clones failed


                # TODO - configure testbed settings, first host will be 'main' for cluster mode, the others should be
                #  in 'client' mode, pointing to the 'main' testbed host


                # TODO run all test cases, for now we will re-use the same guests for the whole test suite
                logging.info("Begin integration tests")
                run_tests_result = run_tests(linked_clone_hosts)
                if not run_tests_result:
                    logging.error(f"Test suite failed")
                else:
                    logging.info(f"Test suite succeeded")
                # TODO - collect report for tests

                # clean up linked clones
                for linked_clone_host in linked_clone_hosts:
                    logging.info(f"Destroying linked clone host: {linked_clone_host.name}")
                    clone_destroy_result = linked_clone_host.ensure_destroyed()


    # TODO prepare report from test harness results

    # TODO clean up the test harness working area in the libvirt images folder
    if not harness_settings.dev_mode:
        shutil.rmtree(workspace_folder)

    # TODO we could turn off the test harness network, leaving it for parallel harness runs for now
    if not harness_settings.dev_mode:
        harness_network.net_destroy()
        harness_network.net_undefine()

    # TODO - return based on success of harness, so take all result bools and only return True if all True
    return network_result


if __name__ == '__main__':
    # TODO dev/debug mode where it pauses on failed test to allow inspection

    logging.info(f"Starting harness with ID: {harness_settings.test_id}")
    logging.info(f"Workspace: {harness_settings.workspace}")

    logging.info("Getting libvirt connection")
    conn = get_libvirt_connection()

    # run test harness
    result = main(conn)

    logging.info("Closing connection to libvirt")
    conn.close()
    logging.info("End of test harness")

    if not result:
        sys.exit(1)

