RELEASE_VERSION=ff9c43d271cc696795c35fdd69b8a33ef9b10466

echo "installing Frida-Tools repo"
cd /var/lib/testbedos/tools/
if [ -d "Frida-Tools" ]; then
    echo "Frida Tools already exists, checking out the current release for the TestbedOS version"
    cd Frida-Tools || exit
    sudo git fetch
    sudo git checkout $RELEASE_VERSION
    sudo git pull
else
    echo "Frida Tools does not exist, pulling from GitHub"
    sudo git clone https://github.com/Bristol-Cyber-Security-Group/Frida-Tools.git
    cd Frida-Tools || exit
    sudo git checkout $RELEASE_VERSION || exit
fi

# set up a pip environment just for the frida tools
FRIDA_VENV=/var/lib/testbedos/tools/frida_tools_venv
sudo mkdir -p $FRIDA_VENV
sudo chmod 777 $FRIDA_VENV
sudo python3 -m venv $FRIDA_VENV
sudo $FRIDA_VENV/bin/pip install -U pip setuptools

sudo $FRIDA_VENV/bin/pip install -r requirements.txt

# update the python path in test privacy script to the environment created just now
sudo sed -i 's#PYTHON="/path/to/poetry/env/bin/python"#PYTHON="/var/lib/testbedos/tools/frida_tools_venv/bin/python"#g' /var/lib/testbedos/tools/Frida-Tools/test-privacy.sh
