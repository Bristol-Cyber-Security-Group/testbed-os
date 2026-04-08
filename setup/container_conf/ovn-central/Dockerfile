FROM ubuntu:24.04

ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update && apt-get install -y \
    ovn-central \
    && rm -rf /var/lib/apt/lists/*

COPY start-ovn-central.sh /start-ovn-central.sh
RUN chmod +x /start-ovn-central.sh

CMD ["/start-ovn-central.sh"]
