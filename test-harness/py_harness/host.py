import os
import time
from urllib.request import urlretrieve

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
    hostname: str

    def __init__(self, conn):
        self.conn = conn

    def set_name(self, name: str):
        self.name = f"testbed-host-{name}"

    def set_location(self, img_name: str):
        self.img_location = f"{harness_settings.workspace}/{img_name}"

    def set_hostname(self, hostname: str):
        self.hostname = hostname

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

            # this just checks the connection status and returns immediately
            res = ssh_command("true", ssh_key, hostname)

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
            try:
                domain.shutdown()
            except libvirt.libvirtError as e:
                logging.error(f"Failed to shutdown host, with error: {e}")
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
        # the hostname is dependent on the type of provisioning tool used i.e. nocloud for cloud-init
        match image_os:
            case BaseOperatingSystem.Ubuntu22_04: # | BaseOperatingSystem.Ubuntu20_04 | BaseOperatingSystem.Ubuntu24_04:
                self.set_hostname(f"nocloud@192.168.{harness_settings.harness_subnet_octet}.10")

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
                subprocess.run(["sed", "-i", f"s/testbed-server/testbed-host-base-{harness_settings.test_id}/g", f"{harness_settings.workspace}/meta-data"])
                # now deploy with virt-install, this is a known working template for our use case in cloud-init
                result = subprocess.run(["virt-install",
                                "--name", self.name,
                                "--memory", str(harness_settings.base_vm_mem),
                                "--vcpus", str(harness_settings.base_vm_cpu),
                                "--disk", self.img_location,
                                "--import",
                                "--mac", "52:54:00:00:00:00",
                                "--os-variant", self.image_os.value.os_variant,
                                "--network", f"network={harness_settings.harness_network_name}",
                                "--cloud-init", f'user-data="{harness_settings.workspace}/user-data",meta-data="{harness_settings.workspace}/meta-data",network-config="{harness_settings.workspace}/network-config"',
                                "--graphics", "none",
                                "--noautoconsole",
                                "--noreboot",
                                "--check", "mac_in_use=off", # we need this as we have clashing mac addresses
                                ])

                if result.returncode != 0:
                    logging.error("Failed to provision base image")
                    return False

                # libvirt will be initialising the VM, we need to wait until the VM is up by testing the SSH connection using
                # the keys we have pushed in the configuration
                logging.info("Waiting for cloud-init guest to start")
                # the base guest will have the first IP in the network range for the third octet
                return self.check_if_ready(harness_settings.base_ssh_key, self.hostname)
        
        return False

    def install_testbed(self) -> bool:
        # we will go through the whole installation process of the testbed and stop before configuring the host, as that
        # will be finished when we create the linked clones of this base VM

        logging.info("Installing testbed code on base guest")

        # get code for this commit
        try:
            # TODO - get current state of dev code from host if test_id is dev
            branch = harness_settings.test_id if harness_settings.test_id != "dev" else "develop"
            git_clone_result = ssh_command(
                f"git clone https://github.com/Bristol-Cyber-Security-Group/testbed-os.git",
                harness_settings.base_ssh_key,
                self.hostname,
            )
            if git_clone_result.returncode != 0:
                logging.error("Failed to clone testbed repo from GitHub")
                return False
            git_checkout_result = ssh_command(f"cd testbed-os && git checkout {branch}",
                harness_settings.base_ssh_key,
                self.hostname,)
            if git_checkout_result.returncode != 0:
                logging.error(f"Failed to checkout {branch} from GitHub")
                return False
            logging.info(f"Testbed code cloned to commit: {branch}")
        except Exception as e:
            logging.error(e)
            return False

        # run the ansible install
        try:
            ssh_command("sudo apt update && sudo apt install ansible -y", harness_settings.base_ssh_key, self.hostname)
            # TODO how to handle ask become pass, and the confirmation
            # since we are not using an interactive shell, the prompt will skip, so we must provide the variable as an
            # extra var, which will then be used as if the prompt accepted a yes from the user
            ssh_command("cd ~/testbed-os/setup/singleton && ansible-playbook setup.yml --extra-vars 'install_bool=yes'", harness_settings.base_ssh_key, self.hostname)
        except Exception as e:
            logging.error(e)
            return False

        # brief check for installed artefacts such as kvm-compose
        try:
            # TODO - check the results for reporting
            poetry_result = ssh_command("cd ~/testbed-os/ && bash -l which poetry || exit", harness_settings.base_ssh_key, self.hostname)
            pyenv_result = ssh_command("cd ~/testbed-os/ && bash -l which pyenv || exit", harness_settings.base_ssh_key, self.hostname)
            kvm_compose_result = ssh_command("cd ~/testbed-os/ && bash -l which kvm-compose || exit", harness_settings.base_ssh_key, self.hostname)
            kvm_ui_cli_result = ssh_command("cd ~/testbed-os/ && bash -l which kvm-ui-cli || exit", harness_settings.base_ssh_key, self.hostname)
            docker_result = ssh_command("cd ~/testbed-os/ && bash -l which docker || exit", harness_settings.base_ssh_key, self.hostname)
            avdmanager_result = ssh_command("cd ~/testbed-os/ && bash -l which avdmanager || exit", harness_settings.base_ssh_key, self.hostname)
        except Exception as e:
            logging.error(e)
            return False

        # install a cloud-init image for the guests, we will just use one type across all tests
        try:
            logging.info("Pre-downloading guest image")
            # send download log to dev/null otherwise CICD logs will balloon with download progress
            ssh_command(
                "sudo mkdir -p /var/lib/testbedos/images/ && "
                "cd /var/lib/testbedos/images/ && "
                f"sudo wget {harness_settings.guest_vm_image_url} -O ubuntu_20_04.img -o /dev/null",
                harness_settings.base_ssh_key, self.hostname)
        except Exception as e:
            logging.error(e)
            return False

        # TODO - do we need guest keys, since testbed manages that already

        return True


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


def ssh_command(cmd: str, ssh_key: str, hostname: str) -> subprocess.CompletedProcess:
    return subprocess.run(["ssh", "-i", ssh_key,
                          "-o", "StrictHostKeyChecking no",
                          "-o", "UserKnownHostsFile /dev/null",
                          hostname,
                          cmd,
                          ])
