#!/bin/bash
# Compiles release version and copies into /usr/local/bin/

set -e
cargo build --release
sudo cp ../target/release/kvm-compose /usr/local/bin/kvm-compose
sudo chown root /usr/local/bin/kvm-compose

