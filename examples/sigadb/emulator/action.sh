#!/bin/bash

sudo apt update
sudo apt install -y git openjdk-17-jdk-headless opendjk-8-jdk
sudo apt install android-tools-adb -y
bash /etc/nocloud/context/emulator.sh -i

#adb kill-server && adb start-server
#bash /etc/nocloud/context/emulator.sh -r

# uncomment and run if using cloud-init and require a desktop
#sudo apt install tasksel -y
#sudo tasksel install ubuntu-desktop

# run reboot command to load desktop GUI
#reboot
