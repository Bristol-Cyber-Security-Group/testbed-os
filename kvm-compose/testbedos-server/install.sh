# create and install the server and configuration into /var/lib/

set -e

sudo mkdir -p /var/lib/testbedos/config/
sudo mkdir -p /var/lib/testbedos/server/
sudo mkdir -p /var/lib/testbedos/deployments/
sudo mkdir -p /var/lib/testbedos/keys/
sudo mkdir -p /var/lib/testbedos/tools/

cargo build --release

# place the mode.json in place if there is not one already there
if [ ! -f /var/lib/testbedos/config/mode.json ]; then
    echo "Placing a mode.json with master mode set"
    sudo cp mode.json /var/lib/testbedos/config/mode.json
fi

# stop and remove daemon if it already exists, otherwise the following copy wont work is server running
sudo systemctl status testbedos-server.service && sudo systemctl stop testbedos-server.service
sudo systemctl status testbedos-server.service && sudo systemctl disable testbedos-server.service

sudo cp ../target/release/testbedos-server /usr/local/bin/testbedos-server
sudo chown root /usr/local/bin/testbedos-server

# setup the daemon
sudo cp testbedos-server.service /etc/systemd/system/ || exit 1
sudo systemctl daemon-reload

# place the testbed default insecure keys into the testbed server directory
sudo cp ../kvm-compose/assets/id_ed25519_testbed_insecure_key /var/lib/testbedos/keys/
sudo cp ../kvm-compose/assets/id_ed25519_testbed_insecure_key.pub /var/lib/testbedos/keys/

# place templates and assets in testbed folder
sudo cp -r assets/ /var/lib/testbedos/

# copy documentation into the server assets folder for development when running in debug
rm -rf assets/documentation/
cp -r ../../build/html/ assets/documentation/
