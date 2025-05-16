import time

import libvirt
import logging
import socket
import zlib
import harness_settings
from typing import Optional

class HarnessNetwork:

    """
    This class manages the libvirt network that will be used for the test harness. This is just a simple NAT network
    that allows the host(s) to communicate with each-other, and provides network connectivity.

    Each test harness will have its own network, so we are free to destroy and recreate it. The network name will be
    specific to the `test_id` which will relate to the test run trigger i.e. commit hash.
    """

    # hold a reference to the libvirt resources
    conn: libvirt.virConnect
    network: Optional[libvirt.virNetwork]
    network_xml: Optional[str]

    def __init__(self, conn: libvirt.virConnect):
        self.conn = conn

    def get_network(self):
        logging.info("Checking if test harness network exists")
        try:
            self.network = self.conn.networkLookupByName(harness_settings.harness_network_name)
            logging.info(f"Network {self.network} exists")
        except libvirt.libvirtError as e:
            logging.info("No existing network not found, skipping destroy step")
            self.network = None

    def net_destroy(self):
        if self.network is None:
            return
        if self.network.isActive():
            logging.info("Destroying network")
            self.network.destroy()

    def net_undefine(self):
        if self.network is None:
            return
        logging.info("Undefining network")
        self.network.undefine()

    def format_network_xml(self):
        # get xml from assets folder in container as a string
        with open(harness_settings.testbed_network_location, "r") as f:
            self.network_xml = f.read()
        # replace the network name with the unique name for this harness execution
        self.network_xml = self.network_xml.replace("test-harness-network", harness_settings.harness_network_name)
        # set up the portforwarding ports
        self.get_vm_port_range()

    def net_define(self):
        logging.info("Defining network")
        # TODO this should be upgraded to a templating system rather than crude string replacements

        # This function will format the base xml with the configuration needed to create the test harness
        # network. It will try to get a free port range for the portforwarding of VMs, and then also try
        # to get a free IP range for the network. This is a slightly awkward process, because we define the
        # network XML first, and libvirt will accept this even if there are clashing IPs. Only when you
        # try to start the network, will it then tell you if there was a clash. So if there is a clash,
        # this function will undefine the just attempted definition, increment the IP range and then
        # try again.

        # now try to define, the subnet might be in use so increment until we get a free one
        for octet in range(50, 255):
            try:
                logging.info(f"Will try to create a libvirt network with a free IP range")
                # start with a fresh version of the xml
                self.format_network_xml()
                # specify the ip range
                self.network_xml = self.network_xml.replace("subnetoctet", str(octet))
                self.network = self.conn.networkDefineXML(self.network_xml)

                # try to start it
                self.net_start()
                # successful network creation, store the octet for later
                harness_settings.harness_subnet_octet = octet
                logging.info(f"Network {self.network.name()} created successfully")
                return True
            except libvirt.libvirtError as e:
                logging.error(f"Failed to create network: {e}")
                logging.info(f"Will increment and try again")
                # we sleep here as there seems to be a bug if we do this all too quickly
                time.sleep(1)
                # we need to undefine to be able to change the ip subnet to try again
                self.network.undefine()
        # we were not able to assign a free ip range
        return False

    def net_start(self):
        logging.info("Starting network")
        self.network.create()

    def reload(self) -> bool:
        # Use this function to just destroy and then start the network before running the test harness
        logging.info("Making sure the test harness network is running")
        try:
            self.get_network()
            self.net_destroy()
            self.net_undefine()
            self.net_define()
            # self.net_start()
            logging.info("Reloading network done.")
        except libvirt.libvirtError as e:
            logging.error("Failed to reload network due to: {}".format(e))
            self.net_destroy()
            return False
        return True

    def get_vm_port_range(self):
        # we need to get a free port range for accessing the virtual machines we will create from the host, as
        # part of the portforwarding

        # we have up to 4 unique VMs per harness instance, base, tb-one, tb-two, tb-three
        port_range_increment = 4

        # use the test id of this harness instance as a unique source for the ports
        port_base = gen_port_base()
        logging.info(f"Port base: {port_base}")
        # check if this is unused
        result = check_port(port_base, port_range_increment)
        if not result:
            while True:
                logging.error("Port base in use, trying next range")
                port_base += port_range_increment
                result = check_port(port_base, port_range_increment)
                if result:
                    break
        # now we have a free port base, update xml
        self.network_xml = self.network_xml.replace("hostportbase", str(port_base))
        self.network_xml = self.network_xml.replace("hostportone", str(port_base + 1))
        self.network_xml = self.network_xml.replace("hostporttwo", str(port_base + 2))
        self.network_xml = self.network_xml.replace("hostportthree", str(port_base + 3))


def check_port(port: int, port_range_increment: int) -> bool:
    logging.info("Checking if port range is free")
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
        s.settimeout(0.1)
        # check all 4 ports to see if they are free
        for ii in range(0, port_range_increment):
            current_port = port + ii
            # TODO - this isn't doing what it is supposed to
            result = s.connect_ex(("localhost", current_port))
            logging.info(f"Port {current_port} is free: {result}")
            if result == 1:
                logging.error(f"A port in the range is not free")
                return False
        return True


def gen_port_base():
    pipeline_id = zlib.crc32(harness_settings.test_id.encode("utf-8"))
    port_base = 22000 + pipeline_id % 10000
    return port_base
