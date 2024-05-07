# Signal ADB example

Sets up two kvms, each with the signal android client.

## Setup

Provide an image (qcow2 or raw) with ubuntu desktop installed and set in the following section in kvm-compose yaml file:
```yaml
    disk:
      existing_disk:
        path: /var/lib/libvirt/images/ubuntu20.04-1.qcow2
        driver_type: qcow2
```

Run test cases using following syntax `kvm-compose playbook playbook.csv`. 

## Notes

This is using ssh for running playbooks, the VMs must have openssh-server installed with UFW rules allowing ssh. Note the use of `password_ssh_enabled: true` in the kvm-compose yaml file.
