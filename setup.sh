#!/bin/bash
export OS_PRETTY_NAME=$(grep '^PRETTY_NAME' /etc/os-release)
if [[ "$OS_PRETTY_NAME" ==  'PRETTY_NAME="Debian GNU/Linux 12 (bookworm)"' ]]; then
	sudo apt install -y ansible
	cd setup/singleton
	ansible-playbook --ask-become-pass setup.yml
else
	echo "The only supported version for TestbedOS is Debian GNU/Linux 12 (bookworm)"
	exit 1
fi
