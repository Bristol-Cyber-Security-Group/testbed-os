import libvirt
import logging
import harness_settings
from typing import Optional

class HarnessNetwork:

    """
    This class manages the libvirt network that will be used for the test harness. This is just a simple NAT network
    that allows the host(s) to communicate with each-other, and provides network connectivity.
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
        self.network.destroy()

    def net_undefine(self):
        if self.network is None:
            return
        logging.info("Undefining network")
        self.network.undefine()

    def net_define(self):
        logging.info("Defining network")

    def net_start(self):
        logging.info("Starting network")

    def reload(self):
        # Use this function to just destroy and then start the network before running the test harness
        logging.info("Making sure the test harness network is running")
        self.get_network()
        # TODO currently not destroying the network as we might want to leave it running for parallel test harness runs
        # self.net_destroy()
        # self.net_undefine()
        self.net_define()
        self.net_start()
        logging.info("Reloading network done.")
