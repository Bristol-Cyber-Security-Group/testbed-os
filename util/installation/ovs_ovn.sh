## OVN - we will build from source and place the git repo in the local testbed folder
## we will also build OVS so that the versions match
git clone https://github.com/ovn-org/ovn.git
cd ovn
git checkout v24.03.1
./boot.sh
git submodule update --init

cd ovs
./boot.sh
./configure
make
sudo make install
cd ..

./configure
make
sudo make install
