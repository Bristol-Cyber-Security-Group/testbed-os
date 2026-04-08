FROM ubuntu:24.04

ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update && apt-get install -y \
    libvirt-daemon-system \
    qemu-kvm \
    libvirt-clients \
    && rm -rf /var/lib/apt/lists/*

COPY start-libvirt.sh /start-libvirt.sh
RUN chmod +x /start-libvirt.sh

CMD ["/start-libvirt.sh"]
