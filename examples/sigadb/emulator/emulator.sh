#!/bin/bash

# IMPORTANT: do not use sudo with this script unless the installation directory is a root user directory
# installation directory
DIR=~/sdk
# command line tools package from Google
CMDTOOLS_URL=https://dl.google.com/android/repository/commandlinetools-linux-7583922_latest.zip
# signal app url
SIGNAL_APP=https://updates.signal.org/android/Signal-Android-website-prod-universal-release-5.32.7.apk

# install the android emulator
install(){
	echo "Installing the android emulator";
	# check if java installed
	! [[ $(which java) ]] && sudo apt-get -y install openjdk-8-jdk
	# check if adb installed
	! [[ $(which adb) ]] && sudo apt-get -y install android-tools-adb
	# download cmdtools
	mkdir -p $DIR/cmdline-tools
	wget $CMDTOOLS_URL -O $DIR/cmdline-tools/tools.zip
	unzip $DIR/cmdline-tools/tools.zip -d $DIR/cmdline-tools/
	mv $DIR/cmdline-tools/cmdline-tools $DIR/cmdline-tools/latest
	rm $DIR/cmdline-tools/tools.zip
	# install emulator
	yes | $DIR/cmdline-tools/latest/bin/sdkmanager --licenses
	echo y | $DIR/cmdline-tools/latest/bin/sdkmanager --install "emulator" "system-images;android-28;google_apis_playstore;x86" "platforms;android-28" "platform-tools"
	# create android device
	echo no | $DIR/cmdline-tools/latest/bin/avdmanager create avd -n "my_avd_30" -k "system-images;android-28;google_apis_playstore;x86"
	# download signal app
	wget $SIGNAL_APP -O $DIR/signal.apk
	# start adb server
	adb kill-server && adb start-server
	echo "Done";
}

# uninstall emulator and all related downloaded data
uninstall(){
	echo "Unstalling the android emulator ....";
	rm -r $DIR
	rm -r ~/.android
	echo "Done";
}

run(){
	echo "Running the android emulator";
	$DIR/emulator/emulator -avd my_avd_30 &
}

usage(){
	echo "To install the emulator  -i, --install"
	echo "To uninstall the emulator -u, --uninstall"
	echo "To run the emulator after installing -r, --run"
}

process_args(){
	[ "$#" -lt 1 ] && usage && exit 1;
	while [ "$#" -gt 0 ]; do
		case $1 in
			-i|--install) install && exit 0;;
			-u|--uninstall) uninstall && exit 0;;
			-r|--run) run && exit 0;;
			-h|--help) usage && exit 0;;
			*) usage && exit 1;;
		esac;
	done
}

process_args $@