RELEASE_VERSION=v1.0.2

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
    sudo git clone -b $RELEASE_VERSION https://github.com/Bristol-Cyber-Security-Group/Frida-Tools.git
    cd Frida-Tools || exit
fi

# set up a pip environment just for the frida tools
FRIDA_VENV=/var/lib/testbedos/tools/frida_tools_venv
sudo mkdir $FRIDA_VENV
sudo chmod 777 $FRIDA_VENV
python3 -m venv $FRIDA_VENV
$FRIDA_VENV/bin/pip install -U pip setuptools
$FRIDA_VENV/bin/pip install poetry

# set up the python environment for the Frida-Tools
# need to ignore keyring https://github.com/python-poetry/poetry/issues/8623
sudo PYTHON_KEYRING_BACKEND=keyring.backends.null.Keyring $FRIDA_VENV/bin/poetry install

# update the python path in test privacy script to the environment created just now
sudo sed -i 's#PYTHON="/path/to/poetry/env/bin/python"#PYTHON="/var/lib/testbedos/tools/frida_tools_venv/bin/python"#g' /var/lib/testbedos/tools/Frida-Tools/test-privacy.sh
