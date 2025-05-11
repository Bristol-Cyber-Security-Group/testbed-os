import os
import time
import libvirt
import logging
import harness_settings
import shutil
import subprocess
from typing import Optional
from host_images import BaseOperatingSystem, OSInit


class Host:

    """
    This is the abstract base class that all host classes inherit from. Contains shared code for managing the host via
    libvirt.
    """

    conn: libvirt.virConnect
    name: str
    img_location: str

    def __init__(self, conn):
        self.conn = conn

    def set_name(self, name: str):
        self.name = f"testbed-host-{name}"

    def set_location(self, img_name: str):
        self.img_location = f"{harness_settings.workspace}/{img_name}"

    def exists(self) -> Optional[libvirt.virDomain]:
        logging.info(f"Checking if {self.name} exists")
        try:
            # check to see if it exists
            return self.conn.lookupByName(self.name)
        except libvirt.libvirtError as e:
            return None

    def ensure_destroyed(self):
        # make sure there is no previous base VM
        domain = self.exists()
        logging.info("Ensuring the VM is destroyed")
        if domain is not None:
            logging.info("VM is defined, destroying")
            if domain.isActive():
                domain.destroy()
            domain.undefine()
        # make sure the img doesn't exist
        if os.path.exists(self.img_location):
            logging.info(f"Deleting {self.img_location}")
            os.remove(self.img_location)

    def start(self):
        pass

    @staticmethod
    def check_if_ready(ssh_key: str, hostname: str) -> bool:
        # depending on the timeout configuration, we will check every 10 seconds up to the timeout
        timeout_increment = 0
        while timeout_increment < harness_settings.host_ready_timeout_seconds:

            res = subprocess.run(["ssh", "-i", ssh_key,
                            "-o", "StrictHostKeyChecking no",
                            "-o", "UserKnownHostsFile /dev/null",
                            hostname,
                            "true"  # this just checks the connection status and returns
                            ])

            if res.returncode == 0:
                logging.info("Host is up and ready")
                return True

            logging.info("Host not up yet, waiting ...")

            time.sleep(harness_settings.host_ready_increment_seconds)
            timeout_increment += harness_settings.host_ready_increment_seconds

        logging.error("We reached the timeout, this host ready check has failed")
        return False

    def stop(self):
        logging.info(f"Stopping {self.name}")
        domain = self.exists()
        if domain is not None:
            domain.shutdown()
        else:
            logging.warning(f"Domain {self.name} doesn't exist")


class BaseHost(Host):
    """
    This is the base image used for the test harness, which will contain the (unconfigured) testbed installation. Once
    this image has been built and the testbed installation was successful, it will then be used as a base image for
    linked clones for all the test harness tests. This means that we can create linked clones from this image quickly,
    with efficient use of disk space, and then destroy the clones once we are done with a specific test case.

    For provisioning the base host, we will use the tool `virt-install`, as this contains the high level commands to
    facilitate the installation of cloud-init.
    """

    conn: libvirt.virConnect
    image_os: BaseOperatingSystem
    name: str

    def __init__(self, conn: libvirt.virConnect, image_os: BaseOperatingSystem):
        super().__init__(conn)
        self.image_os = image_os
        self.set_name(f"base-{harness_settings.test_id}")
        self.set_location("testbedhost_base.img")

    def create(self) -> bool:
        # make sure we have the base image downloaded
        self.image_os.download_image()
        # make sure any previous VM is down and undefined
        super().ensure_destroyed()

        # create a copy of the base image to be used as the live image for this base host
        logging.info("Create a copy of the base image for the live VM")
        shutil.copy(self.image_os.get_base_img_location(), self.img_location)
        # we must use qemu-img to resize the disk before we start it
        logging.info("Resize the base image")
        subprocess.run(["qemu-img", "resize", self.img_location, harness_settings.base_vm_disk])

        # seed the VM installation depending on the tech being used
        match self.image_os.value.os_init:
            case OSInit.cloud_init:
                logging.info("Provisioning base image with cloud-init configuration")
                # we need to gather and customise the cloud-init config files
                shutil.copy(harness_settings.cloud_init_meta_data, harness_settings.workspace)
                shutil.copy(harness_settings.cloud_init_user_data, harness_settings.workspace)
                shutil.copy(harness_settings.cloud_init_network_config, harness_settings.workspace)
                # replace the host name with the unique host name in the meta data
                subprocess.run(["sed", "-i", f"s/testbed-server/testbedhost-base/g", f"{harness_settings.workspace}/meta-data"])
                # now deploy with virt-install, this is a known working template for our use case in cloud-init
                result = subprocess.run(["virt-install",
                                "--name", self.name,
                                "--memory", str(harness_settings.base_vm_mem),
                                "--vcpus", str(harness_settings.base_vm_cpu),
                                "--disk", self.img_location,
                                "--import",
                                "--os-variant", self.image_os.value.os_variant,
                                "--network", f"network={harness_settings.harness_network_name}",
                                "--cloud-init", f'user-data="{harness_settings.workspace}/user-data",meta-data="{harness_settings.workspace}/meta-data",network-config="{harness_settings.workspace}/network-config"',
                                "--graphics", "none",
                                "--noautoconsole",
                                "--noreboot"
                                ])

                if result.returncode != 0:
                    logging.error("Failed to provision base image")
                    return False

                # libvirt will be initialising the VM, we need to wait until the VM is up by testing the SSH connection using
                # the keys we have pushed in the configuration
                logging.info("Waiting for cloud-init guest to start")
                self.check_if_ready(harness_settings.base_ssh_key, "nocloud@testbedhost-base")
        
        return True

    def install_testbed(self):
        pass


class LinkedCloneHost(Host):

    """
    This is the linked clone of the base image used in the test harness.
    """

    host_number: str
    base_host: BaseHost

    def __init__(self, conn: libvirt.virConnect, base_host: BaseHost, number: str):
        super().__init__(conn)
        self.base_host = base_host
        self.host_number = number
        self.set_name(f"{number}-{harness_settings.test_id}")

    def create(self):
        # TODO create linked clone
        super()._create()

