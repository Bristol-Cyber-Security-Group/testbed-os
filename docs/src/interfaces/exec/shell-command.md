# shell-command

This allows you to run ad-hoc commands in the guest.
The command are executed via a pseudo terminal on the guest, which means there is a requirement for credentials.
For cloud-init libvirt guests, we have default credentials already configured but if you have your own virtual machine then you must supply them.
They can be supplied as follows (yaml trimmed with elipses for brevity):

``` yaml
- name: client1
  ...
  libvirt:
    cpus: 2
    memory_mb: 2048
    username: ubuntu
    password: password
    libvirt_type:
      ...
```

The username and password fields exist under the libvirt indentation level but above the libvirt_type.
If the guest has passwordless login, then just leave an empty string but the element must be present.

The command running does not currently have a mechanism to elevate privileges i.e. defer to the user for a password.
For example, if you attempted to run a command and it requires the root password that is not the same as the password provided.

Note: if you want to send some command that uses special characters i.e. $ or ~, then make sure to wrap these or the whole command in quotes.
Otherwise your current shell will expand them before the command is captured by the testbed code.

