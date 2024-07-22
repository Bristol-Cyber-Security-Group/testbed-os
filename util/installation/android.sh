CMDLINETOOLS_URL=https://dl.google.com/android/repository/commandlinetools-linux-11076708_latest.zip
! [[ $(which java) ]] && sudo apt-get -y install openjdk-17-jdk
! [[ $(which adb) ]] && sudo apt-get -y install android-tools-adb
sudo mkdir -p /opt/android-sdk/cmdline-tools/
sudo chown $USER: -R /opt/android-sdk/
wget $CMDLINETOOLS_URL -O /opt/android-sdk/cmdline-tools/tools.zip
unzip /opt/android-sdk/cmdline-tools/tools.zip -d /opt/android-sdk/cmdline-tools/
mv /opt/android-sdk/cmdline-tools/cmdline-tools /opt/android-sdk/cmdline-tools/latest
rm /opt/android-sdk/cmdline-tools/tools.zip
#if ! [[ $(which sdkmanager) ]]; then
#  echo 'export PATH=$PATH:/opt/android-sdk/cmdline-tools/latest/bin' >> ~/.bashrc
#  echo 'export ANDROID_HOME=/opt/android-sdk/' >> ~/.bashrc
#  echo 'export ANDROID_SDK_ROOT=/opt/android-sdk/' >> ~/.bashrc
#  source ~/.bashrc
#fi
yes | /opt/android-sdk/cmdline-tools/latest/bin/sdkmanager --licenses

echo y | /opt/android-sdk/cmdline-tools/latest/bin/sdkmanager --install "emulator" "platform-tools"

# need to create the platform folder otherwise the sdk will claim there is no sdk root
mkdir -p /opt/android-sdk/platforms

