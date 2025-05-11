import logging
logging.basicConfig(level=logging.INFO, format='%(asctime)s - %(levelname)s - %(message)s')

# harness settings
test_id = "dev" # TODO set to commit hash from ENV if present, default to dev
workspace = f"/var/lib/libvirt/images/testbed-os/test-harness/{test_id}"
max_n_hosts = 1  # how many hosts the test harness will go up to before stopping, max 3 supported

host_ready_increment_seconds = 10
host_ready_timeout_seconds = 120

# libvirt settings
harness_network_name = f"test-harness-network-{test_id}"

base_vm_mem = 2048
base_vm_disk = "+20G"
base_vm_cpu = 2

# asset locations
testbed_network_location = "/app/assets/testbed-network.xml"
# cloud-init
cloud_init_meta_data = "/app/assets/iso/meta-data"
cloud_init_user_data = "/app/assets/iso/user-data"
cloud_init_network_config = "/app/assets/iso/network-config"
# VM keys
base_ssh_key = "/app/assets/ssh_key/id_ed25519"
