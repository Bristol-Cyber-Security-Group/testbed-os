import libvirt
import logging
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
        logging.info("Destroying network")
        if self.network.isActive():
            self.network.destroy()

    def net_undefine(self):
        if self.network is None:
            return
        logging.info("Undefining network")
        self.network.undefine()

    def net_define(self):
        logging.info("Defining network")
        # TODO this should be upgraded to a templating system rather than crude string replacements

        # get xml from assets folder in container as a string
        with open(harness_settings.testbed_network_location, "r") as f:
            network_xml = f.read()
        # replace the network name with the unique name for this harness execution
        network_xml = network_xml.replace("test-harness-network", harness_settings.harness_network_name)
        # now define
        self.network = self.conn.networkDefineXML(network_xml)

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
            self.net_start()
            logging.info("Reloading network done.")
        except libvirt.libvirtError as e:
            logging.error("Failed to reload network due to: {}".format(e))
            self.net_destroy()
            return False
        return True
