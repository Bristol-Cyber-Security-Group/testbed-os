import shutil
import sys
import libvirt
import logging
import harness_settings
from pathlib import Path
from host_images import BaseOperatingSystem
from harness_network import HarnessNetwork
from host import BaseHost


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
    network_result = harness_network.reload()
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

            # create base VM in the default libvirt network
            base_host = BaseHost(conn, base_os)
            create_base_host_result = base_host.create()
            if not create_base_host_result:
                base_host.ensure_destroyed()
                # TODO - report failed result

                # go to next test
                continue

            # install testbed code
            install_best_host_result = base_host.install_testbed()
            if not install_best_host_result:
                # TODO report failure, and where in the install it failed
                continue

            # TODO turn off base host before creating linked clones
            base_host.stop()

            # begin n number of host loop
            for n_hosts in range(1, harness_settings.max_n_hosts + 1):
                logging.info(f"Testing on {n_hosts} hosts")

                # TODO create n number of hosts, check if any already exist and destroy

                # TODO for each test case, create the one to three linked clone VMs and run tests

    # TODO prepare report from test harness results

    # TODO clean up the test harness working area in the libvirt images folder
    shutil.rmtree(workspace_folder)

    # TODO we could turn off the test harness network, leaving it for parallel harness runs for now
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

