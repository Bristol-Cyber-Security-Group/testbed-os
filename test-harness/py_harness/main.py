import sys
import libvirt
import logging
import harness_settings
from pathlib import Path
from host_images import BaseOperatingSystem
from harness_network import HarnessNetwork
from host import BaseImage


def get_libvirt_connection() -> libvirt.virConnect:
    try:
        libvirt_conn = libvirt.open('qemu:///system')

        logging.info("Connected to libvirt")
        return libvirt_conn

    except libvirt.libvirtError as e:
        logging.error(f"Connection error: {e}"
              f"\n\nThe test harness needs a connection to the host's libvirt to continue")
        sys.exit(1)


def main(connection: libvirt.virConnect):
    # TODO initialise report

    # initialise working area for the test harness in the libvirt images folder
    workspace_folder = Path(harness_settings.workspace)
    if not workspace_folder.exists():
        workspace_folder.mkdir(parents=True, exist_ok=True)
    # TODO make sure workspace is clean before starting, but do we re-use images downloaded?

    # set up the libvirt network for the test harness
    harness_network = HarnessNetwork(connection)
    harness_network.reload()

    # for each base image type, run test harness
    for base_os in BaseOperatingSystem:
        logging.info(f"Testing on base image: {base_os.name}")

        base_image = BaseImage(conn, base_os)

        # TODO - destroy previous base image if it exists

        # create base VM in the default libvirt network
        base_image.create()

        # TODO create n number of hosts, check if any already exist and destroy

        # TODO install the testbed and ensure it worked, if it doesn't report and continue to next OS

        # TODO turn off base image

        # TODO for each test case, create the one to three linked clone VMs and run tests

    # TODO prepare report from test harness results

    # TODO clean up the test harness working area in the libvirt images folder

    # TODO we could turn off the test harness network, leaving it for parallel harness runs for now


if __name__ == '__main__':
    # TODO dev/debug mode where it pauses on failed test to allow inspection

    logging.info("Getting libvirt connection")
    conn = get_libvirt_connection()

    # run test harness
    main(conn)

    logging.info("Closing connection to libvirt")
    conn.close()
    logging.info("End of test harness")
