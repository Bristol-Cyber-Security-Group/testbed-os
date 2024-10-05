#!/bin/bash
echo "pre-requisite script will check for existing dependencies before continuing ..."

package_installed() {
    local package="$1"
    dpkg-query -l "$package" &> /dev/null
}

### NETWORK
echo -e "\nchecking for existing OVS and OVN installation ..."
OVS_EXISTS=
OVN_EXISTS=
OVS_VERSION=
OVN_VERSION=
EXPECTED_OVS_VERSION="3.3.0"
EXPECTED_OVN_VERSION="24.03.1"

if ovs-vsctl --version &> /dev/null
then
  echo "OVS exists"
  OVS_EXISTS=true

  # check the ovs version if it exists
  if [[ `ovs-vsctl --version` == *"$EXPECTED_OVS_VERSION"* ]]; then
    echo "OVS version is correct ($EXPECTED_OVS_VERSION)"
    OVS_VERSION=true
  else
    echo "OVS version is incorrect"
    OVS_VERSION=false
  fi

else
  echo "OVS missing"
  OVS_EXISTS=false
fi

if ovn-nbctl --version &> /dev/null
then
  echo "OVN exists"
  OVN_EXISTS=true

  # check the ovs version if it exists
    if [[ `ovn-nbctl --version` == *"$EXPECTED_OVN_VERSION"* ]]; then
      echo "OVN version is correct ($EXPECTED_OVN_VERSION)"
      OVN_VERSION=true
    else
      echo "OVN version is incorrect"
      OVN_VERSION=false
    fi

else
  echo "OVN missing"
  OVN_EXISTS=false
fi

### DOCKER
echo -e "\nchecking for docker installation ..."
DOCKER_EXISTS=
DOCKER_DESKTOP=

if docker --version &> /dev/null
then
  echo "Docker exists"
  DOCKER_EXISTS=true
else
  echo "Docker missing"
  DOCKER_EXISTS=false
fi

if [ -d "/opt/docker-desktop" ]
then
  echo "Docker desktop installed"
  DOCKER_DESKTOP=true
else
  echo "Docker desktop not installed"
  DOCKER_DESKTOP=false
fi

### ANDROID
echo -e "\nchecking for android emulator installation ..."

if sdkmanager --version &> /dev/null
then
  echo "Android SDK manager exists"
  ANDROID_SDK_MANAGER=true
else
  echo "Android SDK manager missing"
  ANDROID_SDK_MANAGER=false
fi

if avdmanager list &> /dev/null
then
  echo "Android AVD manager exists"
  ANDROID_AVD_MANAGER=true
else
  echo "Android AVD manager missing"
  ANDROID_AVD_MANAGER=false
fi

### RUST
echo -e "\nchecking for rust compiler installation ..."
CARGO_EXISTS=

if cargo --version &> /dev/null
then
  echo "Cargo exists"
  CARGO_EXISTS=true
else
  echo "Cargo missing"
  CARGO_EXISTS=false
fi

### LIBVIRT
echo -e "\nchecking for libvirt installation ..."
LIBVIRT_EXISTS=
MISSING_LIBVIRT_DEPS=false

libvirt_deps=("qemu-kvm" "libvirt-daemon-system" "virt-manager" "libvirt-dev")
for dep in "${libvirt_deps[@]}"; do
  if package_installed "$dep"; then
    echo "$dep is installed."
  else
    echo -e "$dep is \e[31mNOT\e[0m installed."
    MISSING_LIBVIRT_DEPS=true
  fi
done

if libvirtd --version &> /dev/null
then
  echo "Libvirt exists"
  LIBVIRT_EXISTS=true
else
  echo "Libvirt missing"
  LIBVIRT_EXISTS=false
fi

### PYTHON
echo -e "\nchecking for python installations ..."
PYTHON3_EXISTS=
PYENV_EXISTS=
POETRY_EXISTS=
PYENV_NOT_IN_PATH=
POETRY_NOT_IN_PATH=
MISSING_PYTHON_DEPS=

python_deps=("python3.10" "python3.10-venv" "python3.10-dev")
for dep in "${python_deps[@]}"; do
  if package_installed "$dep"; then
    echo "$dep is installed."
  else
    echo -e "$dep is \e[31mNOT\e[0m installed."
    MISSING_PYTHON_DEPS=true
  fi
done

if python3 --version &> /dev/null
then
  echo "python 3 exists"
  PYTHON3_EXISTS=true
else
  echo "python 3 missing"
  PYTHON3_EXISTS=false
fi

if pyenv --version &> /dev/null
then
  echo "pyenv exists"
  PYENV_EXISTS=true
else
  # check if pyenv is installed but not in path
  if test -f ~/.pyenv/bin/pyenv
  then
    echo "pyenv is installed but not in PATH"

  if [ -z $(grep "$PYENV_BASHRC" "$BASHRC")  ]; then
	  echo 'export PYENV_ROOT="$HOME/.pyenv"' >> $BASHRC
	  echo 'command -v pyenv >/dev/null || export PATH="$PYENV_ROOT/bin:$PATH"' >> $BASHRC
	  echo 'command -v pyenv >/dev/null || export PATH="$PYENV_ROOT/bin:$PATH"' >> $BASHRC
	  source $BASHRC
	  echo 'Pyenv has been automatically added to your ~/.bashrc file. Restart your terminal or use source ~/.bashrc'
  else echo "Pyenv exist in ~/.bashrc"
  fi
    PYENV_NOT_IN_PATH=true
  else
   echo "pyenv missing"
  fi
  PYENV_EXISTS=false
fi

if poetry --version &> /dev/null
then
  echo "poetry exists"
  POETRY_EXISTS=true
else
  # check if poetry is installed but not in PATH
  if test -f ~/.local/bin/poetry
  then
    echo "poetry is installed but not in PATH"
    POETRY_NOT_IN_PATH=true
  else
    echo "poetry missing"
  fi

  POETRY_EXISTS=false
fi


### GENERAL DEPENDENCIES
echo -e "\nchecking for general dependencies installation ..."
apt_dependencies=("genisoimage" "git" "gcc" "make" "libssl-dev" "build-essential" "curl" "openssh-server" "openssh-client")
MISSING_DEPS=false

for dep in "${apt_dependencies[@]}"; do
  if package_installed "$dep"; then
    echo "$dep is installed."
  else
    echo -e "$dep is \e[31mNOT\e[0m installed."
    MISSING_DEPS=true
  fi
done


echo -e "\nfinished checking for existing dependencies, evaluating what needs to be installed.\n"

### This section will combine any of the previous checks for each software to determine what should
### be installed. Some software has more than one check, some only have one but a new variable with
### the same naming convention is created to keep things more readable.

INSTALL_OVS_OVN=
INSTALL_DOCKER=
INSTALL_ANDROID_EMULATOR=
INSTALL_CARGO=
INSTALL_LIBVIRT=
INSTALL_GENERAL_DEPENDENCIES=
INSTALL_PYTHON=
INSTALL_PYENV=
INSTALL_POETRY=
# Environment variables to configure shell after installation.
BASHRC="$HOME/.bashrc"
POETRY_BASHRC="export PATH="$HOME/.local/bin:$PATH""
PYENV_BASHRC="export PYENV_ROOT="$HOME/.pyenv""

# For OVS and OVS, we want to make sure that the versions align, so ideally both are installed from source and not mix
# and matched from apt. So if any of the EXIST and VERSION variables are not true, stop the script and tell the user
# about this problem and let them decide how to fix.
if [ "$OVS_EXISTS" = true ] && [ "$OVN_EXISTS" = true ] && [ "$OVS_VERSION" = true ] && [ "$OVN_VERSION" = true ]; then
  echo -e "\e[32mOVS and OVN exist and are the correct versions, nothing to do.\e[0m"
  INSTALL_OVS_OVN=false
elif [ "$OVS_EXISTS" = true ] && [ "$OVN_EXISTS" = true ] && ([ "$OVS_VERSION" = false ] || [ "$OVN_VERSION" = false ]); then
  echo -e "\e[31mERROR\e[0m"
  echo "OVS and OVN exist but the versions are not correct, please check your installation to make sure that either is"
  echo "not installed via a package manager. OVS and OVN ideally should be installed from source so that the versions"
  echo "match. This script will not try to overwrite your install to avoid causing dependency issues with other tools."
  echo "If you are able to uninstall both OVS and OVN, then re-run this script to have the correct versions installed."
  echo -e "\nexpected OVS version $EXPECTED_OVS_VERSION and current OVS version:"
  ovs-vsctl --version
  echo -e "\nexpected OVN version $EXPECTED_OVN_VERSION and current OVN version:"
  ovn-nbctl --version
  echo -e "\npre-requisite script will now stop."
  exit 1
elif [ "$OVS_EXISTS" = true ] && [ "$OVN_EXISTS" = false ]; then
  echo -e "\e[31mERROR\e[0m"
  echo "OVS is installed but OVN is not installed. Please check if OVS is installed via your package manager as the"
  echo "the testbed needs both OVS and OVN to be installed via source to have compatible versions. This script will"
  echo "not try to overwrite your OVS install to avoid causing dependency issues with other tools."
  echo "If you are able to uninstall OVS, then re-run this script to have the correct versions installed."
  echo -e "\npre-requisite script will now stop."
  exit 1
elif [ "$OVS_EXISTS" = false ] && [ "$OVN_EXISTS" = false ]; then
  echo "Both OVS and OVN will be installed"
  INSTALL_OVS_OVN=true
fi

# check docker installation
if [ "$DOCKER_EXISTS" = true ]; then
  echo -e "\e[32mDocker is installed, nothing to do.\e[0m"
  INSTALL_DOCKER=false
else
  echo "Docker will be installed"
  INSTALL_DOCKER=true
fi

# check android installation
if [ "$ANDROID_SDK_MANAGER" = true ] && [ "$ANDROID_AVD_MANAGER" = true ]; then
  echo -e "\e[32mAndroid emulator is installed, nothing to do.\e[0m"
  INSTALL_ANDROID_EMULATOR=false
elif ([ "$ANDROID_SDK_MANAGER" = false ] && [ "$ANDROID_AVD_MANAGER" = true ]) || ([ "$ANDROID_SDK_MANAGER" = true ] && [ "$ANDROID_AVD_MANAGER" = false ]); then
  echo -e "\e[31mERROR\e[0m"
  echo "Either the Android SDK manager or Android AVD manager is not installed. Please check your installation as both"
  echo "are required. To avoid dependency issues, we will not overwrite your installation as it may not be in the"
  echo "expected location."
  echo -e "\npre-requisite script will now stop."
  exit 1
else
  echo "Android emulator will be installed."
  INSTALL_ANDROID_EMULATOR=true
fi

# check rust installation
if [ "$CARGO_EXISTS" = true ]; then
  echo -e "\e[32mCargo is installed, nothing to do.\e[0m"
  INSTALL_CARGO=false
else
  echo "Cargo will be installed"
  INSTALL_CARGO=true
fi

# check libvirt installation
if [ "$LIBVIRT_EXISTS" = true ] && [ "$MISSING_LIBVIRT_DEPS" = false ]; then
  echo -e "\e[32mLibvirt is installed, nothing to do.\e[0m"
  INSTALL_LIBVIRT=false
else
  echo "Libvirt and dependencies will be installed"
  INSTALL_LIBVIRT=true
fi

# check python installation
if [ "$PYTHON3_EXISTS" = true ] && [ "$MISSING_PYTHON_DEPS" = false ]; then
  echo -e "\e[32mPython 3 is installed, nothing to do.\e[0m"
  INSTALL_PYTHON=false
else
  echo "Python 3 and dependencies will be installed"
  INSTALL_PYTHON=true
fi
if [ "$PYENV_EXISTS" = true ]; then
  echo -e "\e[32mPyenv is installed, nothing to do.\e[0m"
  INSTALL_PYENV=false
else
  echo "Pyenv will be installed"
  INSTALL_PYENV=true
fi
if [ "$POETRY_EXISTS" = true ]; then
  echo -e "\e[32mPoetry is installed, nothing to do.\e[0m"
  INSTALL_POETRY=false
else
  echo "Poetry will be installed"
  INSTALL_POETRY=true
fi

# check general dependencies
if [ "$MISSING_DEPS" = false ]; then
  echo -e "\e[32mGeneral dependencies are installed, nothing to do.\e[0m"
  INSTALL_GENERAL_DEPENDENCIES=false
else
  echo "General dependencies will be installed"
  INSTALL_GENERAL_DEPENDENCIES=true
fi


# ask the user if they want to continue, if there is anything to install
if [ "$INSTALL_OVS_OVN" = true ] || [ "$INSTALL_DOCKER" = true ] || [ "$INSTALL_ANDROID_EMULATOR" = true ] || [ "$INSTALL_CARGO" = true ] || [ "$INSTALL_LIBVIRT" = true ] || [ "$INSTALL_PYTHON" = true ] || [ "$INSTALL_PYENV" = true ] || [ "$INSTALL_POETRY" = true ] || [ "$INSTALL_GENERAL_DEPENDENCIES" = true ]; then
  read -p "Do you want to continue with the installation outlined above? (y/n): " answer
  answer=${answer,,}
  if [[ "$answer" == "y" ]]; then
      echo
  elif [[ "$answer" == "n" ]]; then
      exit 0
  else
      echo "Please enter 'y' or 'n'. Exiting ..."
      exit 1
  fi
else
  echo "Everything is installed, nothing to do. Exiting."
  exit 0
fi


### Begin dependency installation section

echo -e "\ninstalling any missing dependencies ...\n"

sudo apt update &> /dev/null

if [ "$INSTALL_GENERAL_DEPENDENCIES" = true ]; then
  echo "installing general dependencies"
  for dep in "${apt_dependencies[@]}"; do
    if package_installed "$dep"; then
      # nothing to do
      true
    else
      sudo apt install "$dep" -y || exit 1
    fi
  done
fi

if [ "$INSTALL_OVS_OVN" = true ]; then
  echo "installing OVN and OVS"
  ./util/installation/ovs_ovn.sh || exit 1
fi

if [ "$INSTALL_DOCKER" = true ]; then
  echo "installing Docker"
  ./util/installation/docker.sh || exit 1
fi

if [ "$INSTALL_ANDROID_EMULATOR" = true ]; then
  echo "installing Android Emulator"
  ./util/installation/android.sh || exit 1
fi

if [ "$INSTALL_CARGO" = true ]; then
  echo "installing Cargo"
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y || exit 1
fi

if [ "$MISSING_LIBVIRT_DEPS" = true ]; then
  for dep in "${libvirt_deps[@]}"; do
    if package_installed "$dep"; then
      # nothing to do
      true
    else
      sudo apt install "$dep" -y || exit 1
    fi
  done
fi
if [ "$INSTALL_LIBVIRT" = true ]; then
  echo "installing Libvirt"
#  sudo apt install qemu-kvm libvirt-daemon-system virt-manager libvirt-dev -y || exit 1
  for dep in "${libvirt_deps[@]}"; do
    if package_installed "$dep"; then
      # nothing to do
      true
    else
      sudo apt install "$dep" -y || exit 1
    fi
  done
  sudo adduser $USER libvirt || exit 1
  sudo systemctl restart libvirtd || exit 1
fi

if [ "$MISSING_PYTHON_DEPS" = true ]; then
  for dep in "${python_deps[@]}"; do
    if package_installed "$dep"; then
      # nothing to do
      true
    else
      sudo apt install "$dep" -y || exit 1
    fi
  done
fi
if [ "$INSTALL_PYTHON" = true ]; then
  echo "installing Python 3"
#  sudo apt install python3.10 python3.10-venv python3.10-dev || exit 1
  for dep in "${python_deps[@]}"; do
    if package_installed "$dep"; then
      # nothing to do
      true
    else
      sudo apt install "$dep" -y || exit 1
    fi
  done
fi

if [ "$INSTALL_PYENV" = true ]; then
  echo -e "\ninstalling Pyenv"
  if [ -z $(grep "$PYENV_BASHRC" "$BASHRC")  ]; then
	  echo 'export PYENV_ROOT="$HOME/.pyenv"' >> $BASHRC
	  echo 'command -v pyenv >/dev/null || export PATH="$PYENV_ROOT/bin:$PATH"' >> $BASHRC
	  echo 'command -v pyenv >/dev/null || export PATH="$PYENV_ROOT/bin:$PATH"' >> $BASHRC
	  source $BASHRC
	  echo 'Pyenv has been automatically added to your ~/.bashrc file. Restart your terminal or use source ~/.bashrc'
  else 
	  echo "Pyenv exist in ~/.bashrc"
  fi

  if [ "$PYENV_NOT_IN_PATH" = true ]; then
    echo -e "\e[38;5;214mWARNING\e[0m Please ensure Pyenv is in your shell PATH, as it is already installed."
    echo "See documentation at https://github.com/pyenv/pyenv?tab=readme-ov-file#set-up-your-shell-environment-for-pyenv"
  else

    curl https://pyenv.run | bash
    sudo apt install zlib1g-dev libbz2-dev libreadline-dev libsqlite3-dev libncursesw5-dev xz-utils tk-dev libxml2-dev libxmlsec1-dev libffi-dev liblzma-dev -y || exit 1
    # TODO - ask user if they want to add pyenv to the shell PATH
    ~/.pyenv/bin/pyenv install 3.10.5 || exit 1

  fi
fi

if [ "$INSTALL_POETRY" = true ]; then
  echo -e "\ninstalling Poetry"

  if [ -z $(grep "$POETRY_BASHRC" "$BASHRC")  ]; then
	  echo "export PATH="$HOME/.local/bin:$PATH"" >> $BASHRC
	  source $BASHRC
  else 
	  echo "Poetry exist in ~/.bashrc"
  fi
  if [ "$POETRY_NOT_IN_PATH" = true ]; then
      echo -e "\e[38;5;214mWARNING\e[0m Please ensure poetry is in your shell PATH"
      echo -e "You can add the following line to your ~/.bashrc\n"
      echo 'export PATH=$PATH:/home/$USER/.local/bin'
  else
    curl -sSL https://install.python-poetry.org | python3 -

    # TODO - ask user if they want to add poetry to the shell PATH
  fi
fi

echo -e "\nInstallation complete, make sure Pyenv and Poetry are in your shell PATH. Then, restart your shell or run:"
echo "source ~/.bashrc"
echo "You will need to do this before running the setup.sh script."
