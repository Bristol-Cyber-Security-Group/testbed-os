set -x

echo "hi from $(hostname -I) setup script"

# spawn a python server just for the communication tests
nohup python3 -m http.server > /dev/null 2>&1 &
