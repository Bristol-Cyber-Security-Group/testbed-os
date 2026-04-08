FROM ubuntu:24.04

ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update && apt-get install -y \
    openvswitch-switch \
    openvswitch-common \
    ovn-host \
    iproute2 \
    kmod \
    && rm -rf /var/lib/apt/lists/*

COPY start-ovn-chassis.sh /start-ovn-chassis.sh
RUN chmod +x /start-ovn-chassis.sh

CMD ["/start-ovn-chassis.sh"]
