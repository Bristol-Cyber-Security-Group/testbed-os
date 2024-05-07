set -x

echo "dummy shared setup script"

sudo apt install net-tools

echo $(hostname -i)

sleep 10

echo "finished sleep"


