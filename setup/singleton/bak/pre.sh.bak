#!/bin/bash

### install all the dependencies before installing the testbedOS tools

## general dependencies
sudo apt update
# compile
sudo apt install libvirt-dev libssl-dev gcc make zlib1g zlib1g-dev libssl-dev libbz2-dev libsqlite3-dev libffi-dev libncurses5 libncurses5-dev libncursesw5 libreadline-dev lzma liblzma-dev libbz2-dev libtool autoconf git pip python3.10-venv -y
# runtime
sudo apt install qemu-kvm libvirt-daemon-system libvirt-clients libnss-libvirt genisoimage -y

# OVN - we will build from source and place the git repo in the local testbed folder
# we will also build OVS so that the versions match
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

# docker
sudo apt-get install ca-certificates curl gnupg -y
sudo install -m 0755 -d /etc/apt/keyrings
curl -fsSL https://download.docker.com/linux/ubuntu/gpg | sudo gpg --dearmor -o /etc/apt/keyrings/docker.gpg
sudo chmod a+r /etc/apt/keyrings/docker.gpg
echo \
  "deb [arch="$(dpkg --print-architecture)" signed-by=/etc/apt/keyrings/docker.gpg] https://download.docker.com/linux/ubuntu \
  "$(. /etc/os-release && echo "$VERSION_CODENAME")" stable" | \
  sudo tee /etc/apt/sources.list.d/docker.list > /dev/null
sudo apt-get update
sudo apt-get install docker-ce docker-ce-cli containerd.io docker-buildx-plugin docker-compose-plugin docker-compose -y

# avd
CMDLINETOOLS_URL=https://dl.google.com/android/repository/commandlinetools-linux-11076708_latest.zip
! [[ $(which java) ]] && sudo apt-get -y install openjdk-17-jdk
! [[ $(which adb) ]] && sudo apt-get -y install android-tools-adb
sudo mkdir -p /opt/android-sdk/cmdline-tools/
sudo chown $USER: -R /opt/android-sdk/
wget $CMDLINETOOLS_URL -O /opt/android-sdk/cmdline-tools/tools.zip
unzip /opt/android-sdk/cmdline-tools/tools.zip -d /opt/android-sdk/cmdline-tools/
mv /opt/android-sdk/cmdline-tools/cmdline-tools /opt/android-sdk/cmdline-tools/latest
rm /opt/android-sdk/cmdline-tools/tools.zip
if ! [[ $(which sdkmanager) ]]; then
  echo 'export PATH=$PATH:/opt/android-sdk/cmdline-tools/latest/bin' >> ~/.bashrc
  echo 'export ANDROID_HOME=/opt/android-sdk/' >> ~/.bashrc
  echo 'export ANDROID_SDK_ROOT=/opt/android-sdk/' >> ~/.bashrc
  source ~/.bashrc
fi
yes | /opt/android-sdk/cmdline-tools/latest/bin/sdkmanager --licenses

echo y | /opt/android-sdk/cmdline-tools/latest/bin/sdkmanager --install "emulator" "platform-tools"

# need to create the platform folder otherwise the sdk will claim there is no sdk root
mkdir /opt/android-sdk/platforms

# optional
sudo apt install virt-manager -y

# rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y

# python through pyenv
curl https://pyenv.run | bash
which pyenv
if [ $? -eq 1 ]; then
  echo '# pyenv'
  # bashrc
  echo 'export PYENV_ROOT="$HOME/.pyenv"' >> ~/.bashrc
  echo 'command -v pyenv >/dev/null || export PATH="$PYENV_ROOT/bin:$PATH"' >> ~/.bashrc
  echo 'eval "$(pyenv init -)"' >> ~/.bashrc
  # bash profile TODO only run this if in test harness mode
  echo 'export PYENV_ROOT="$HOME/.pyenv"' >> ~/.profile
  echo 'command -v pyenv >/dev/null || export PATH="$PYENV_ROOT/bin:$PATH"' >> ~/.profile
  echo 'eval "$(pyenv init -)"' >> ~/.profile
fi
~/.pyenv/bin/pyenv install 3.10.5

# poetry
curl -sSL https://install.python-poetry.org | python3 -
! [[ $(which poetry) ]] && echo 'export PATH=$PATH:/home/$USER/.local/bin' >> ~/.bashrc

# local dns
sudo sed -i 's/files dns mymachines/files dns mymachines libvirt/g' /etc/nsswitch.conf

# libvirt user permission
sudo adduser $USER libvirt
sudo systemctl restart libvirtd

echo "Script has completed."
echo "You should run 'source ~/.bashrc' to load any new environment variables, or reopen your terminal."
