import logging
logging.basicConfig(level=logging.INFO, format='%(asctime)s - %(levelname)s - %(message)s')

# harness settings
test_id = "dev" # TODO set to commit hash from ENV if present, default to dev
workspace = f"/var/lib/libvirt/images/testbed-os/test-harness/{test_id}"
max_n_hosts = 1  # how many hosts the test harness will go up to before stopping

# libvirt settings
harness_network_name = "test-harness-network"
