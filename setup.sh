sudo apt install -y ansible
cd setup/singleton
ansible-playbook --ask-become-pass setup.yml
