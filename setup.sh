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
fi

if ! pyenv --version &> /dev/null && test -f ~/.pyenv/bin/pyenv
then
  echo -e "\e[31mERROR\e[0m Please place Pyenv in your shell PATH, as it is already installed."
  echo "See documentation at https://github.com/pyenv/pyenv?tab=readme-ov-file#set-up-your-shell-environment-for-pyenv"
  echo "This script requires Pyenv to be in the PATH to run ... Exiting."
  exit 1
elif ! pyenv --version &> /dev/null && ! test -f ~/.pyenv/bin/pyenv
then
  echo "Pyenv not installed, cannot continue."
fi


echo "installing testbed os poetry environment"
# create poetry environment
# TODO need to run poetry update on an existing environment
poetry env use 3.10.5 || exit
poetry install || exit
#poetry update || exit

# removing old doc build and make new structure
rm -rf build/*
mkdir build/html/
mkdir build/man/

echo "building man pages"
# building the man pages for each md file
find ./docs/src/ -type f -name "*.md" | while IFS= read -r file; do
    relative_path=$(realpath --relative-to="./docs/src/" "$file")   # relative path from testbed-os including filename and extension
    relative_dir=build/man/$(dirname $relative_path)    # path difference excluding filename and extension in /build/man
    mkdir -p $relative_dir                              # create the same path difference in /build/man
    relative_path=${relative_path%.md}.1                # changing the extension
    pandoc $file -s -t man -M author="BCSG" -o ./build/man/$relative_path   # generating the man page
    
    cp $file ${file%.md}.bak            # making a backup for the file as this will be overwritten when removing the man pages metadata yaml declaration 
    sed -i '/^---$/,/^---$/d' "$file"   # removing the metadata yaml declaration for the man pages metadata for pandoc
done

echo "building the html pages"
cp ./kvm-compose/testbedos-server/assets/icons/*.png docs/src/      # copying the icons from the server's assets to the source of the documentation
cp ./kvm-compose/testbedos-server/assets/diagrams/*.png docs/src/   # copying the diagrams from the server's assets to the source of the documentation
mdbook build docs/ -d build/html/       # generating the html files from the md files

# restoring the original md files with the pandoc man pages metadata yaml declaration and deleting the backup
find ./docs/src/ -type f -name "*.bak" | while IFS= read -r file; do
    cp $file ${file%.bak}.md
    rm $file
done

# place documentation in server assets
sudo rm -rf /var/lib/testbedos/assets/documentation/
sudo mkdir /var/lib/testbedos/assets/documentation/
sudo cp -r build/html/. /var/lib/testbedos/assets/documentation/

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