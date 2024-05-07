set -e

sudo apt update
sudo apt install -y git openjdk-16-jdk-headless
mkdir -p /opt/signal
cd /opt/signal
git clone https://github.com/AsamK/signal-cli.git
cd signal-cli
git checkout 30690785fd51f9ba841f6f57d5f403bd970f26e3
git apply --ignore-space-change --ignore-whitespace /etc/nocloud/context/use_custom_server.patch
./gradlew build
./gradlew installDist

# Usage
# cd /opt/signal/signal-cli/build/install/signal-cli/bin
# USER=$(/etc/nocloud/env.sh number)
# ./signal-cli -u $USER register
# ./signal-cli -u $USER verify $(/etc/nocloud/env.sh code)
# ./signal-cli -u $USER send -m "message..." DESTINATION
# ./signal-cli --verbose -u $USER receive
