#!/bin/bash

if ! poetry --version &> /dev/null && test -f ~/.local/bin/poetry
then
  echo -e "\e[31mERROR\e[0m Please place Poetry in your shell PATH, as it is already installed."
  echo -e "You can add the following line to your ~/.bashrc\n"
  echo 'export PATH=$PATH:/home/$USER/.local/bin'
  echo "This script requires Poetry to be in the PATH to run ... Exiting."
  exit 1
elif ! poetry --version &> /dev/null && ! test -f ~/.local/bin/poetry
then
  echo "Poetry not installed, cannot continue."
else
  echo "Poetry installed"
fi


echo "installing testbed os poetry environment"
# create poetry environment
# TODO need to run poetry update on an existing environment
poetry env use 3.10.5 || exit
poetry install || exit
#poetry update || exit

echo "building man pages"
# remove old doc build
rm -rf build/*
# build man pages (use kvm-orchestrate venv as it has sphinx and sphinx-click installed)
poetry run sphinx-build -M man docs build
# build html pages of documentation which can be useful
poetry run sphinx-build -M html docs build
# place documentation in server assets
sudo rm -rf /var/lib/testbedos/assets/documentation/
sudo mkdir /var/lib/testbedos/assets/documentation/
sudo cp -r build/html/ /var/lib/testbedos/assets/documentation/

# install man pages TODO

echo "installing textual user interface"
cd util/tui/ || exit
# push UI code into a PATH accessible location
./install.sh

# build rust code after python, as we need the documentation to be compiled for the server to embed into assets

echo "installing kvm-compose"
# compile and install testbed
cd ../../kvm-compose/kvm-compose-cli/ || exit
./install.sh || exit

echo "installing testbedos-server"
# compile and install testbed
cd ../../kvm-compose/testbedos-server/ || exit
./install.sh || exit

# install the privacy testbed tooling
echo "installing privacy tools"
cd ../../util/privacy_tools/
./setup.sh || exit

echo "done."