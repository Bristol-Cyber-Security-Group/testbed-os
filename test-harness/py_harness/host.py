import libvirt
import logging
import harness_settings
from typing import Optional
from host_images import BaseOperatingSystem

class BaseImage:
    """
    This is the base image used for the test harness, which will contain the (unconfigured) testbed installation. Once
    this image has been built and the testbed installation was successful, it will then be used as a base image for
    linked clones for all the test harness tests. This means that we can create linked clones from this image quickly,
    with efficient use of disk space, and then destroy the clones once we are done with a specific test case.
    """

    conn: libvirt.virConnect
    image_os: BaseOperatingSystem
    name: str

    def __init__(self, conn: libvirt.virConnect, image_os: BaseOperatingSystem):
        self.conn = conn
        self.image_os = image_os
        self.name = f"testbed-host-base-{harness_settings.test_id}"

    def exists(self) -> Optional[libvirt.virDomain]:
        logging.info(f"Checking if {self.name} exists")
        try:
            # check to see if it exists
            return self.conn.lookupByName(self.name)
        except libvirt.libvirtError as e:
            return None

    def create(self):
        # make sure we have the base image downloaded
        self.image_os.download_image()
        # make sure there is no previous base VM
        domain = self.exists()
        if domain is not None:
            logging.info("Base VM is already defined, will destroy current VM to start fresh")
            domain.destroy()
            domain.undefine()

        # define and create the VM in libvirt
        # first, get the xml from assets

    def start(self):
        pass

    def install_testbed(self):
        pass

    def stop(self):
        pass

