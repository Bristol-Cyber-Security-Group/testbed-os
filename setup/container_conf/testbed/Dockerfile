# TODO - get rustup image instead?
FROM ubuntu:24.04 as builder

ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update && apt-get install -y \
    curl build-essential pkg-config \
    libvirt-dev \
    libssl-dev \
    libpcap-dev \
    && rm -rf /var/lib/apt/lists/*

# download rustup rather than use the rust image, since we will be running the server binary in an ubuntu image we need
# to make sure that the dependencies are exactly the same
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

WORKDIR /workspace

COPY kvm-compose/ /workspace/

WORKDIR /workspace

# TODO - need to build the binaries but then need to run the server config folder setup etc that goes in
# TODO - optimise the build step? i.e. mounting/caching rust build folder?
RUN cargo build --release
#RUN cargo build --release --bin cli


FROM ubuntu:24.04 AS runtime

ENV DEBIAN_FRONTEND=noninteractive

# TODO - ovn cli common (ovn-central?)? avd?
RUN apt-get update && apt-get install -y \
    libvirt-dev \
    libssl-dev \
    libpcap-dev \
    libvirt-clients \
    docker.io \
    openvswitch-common \
    openvswitch-switch \
    ovn-common \
    genisoimage \
    qemu-utils \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# TODO - setup the testbed system configuration

# TODO - display for AVD?

# TODO - actual location of server
COPY --from=builder /workspace/target/release/testbedos-server /app/testbedos-server
COPY --from=builder /workspace/testbedos-server/mode.json /app/mode.json
COPY --from=builder /workspace/testbedos-server/assets/ /app/assets/
COPY --from=builder /workspace/kvm-compose/assets/id_ed25519_testbed_insecure_key /app/id_ed25519_testbed_insecure_key
COPY --from=builder /workspace/kvm-compose/assets/id_ed25519_testbed_insecure_key.pub /app/id_ed25519_testbed_insecure_key.pub

COPY --from=builder /workspace/target/release/kvm-compose /app/kvm-compose

COPY setup/container_conf/testbed/entrypoint.sh /app/entrypoint.sh
RUN chmod +x /app/entrypoint.sh

# TODO -  entrypoint that runs the install.sh script for server to make sure that the config files exist
CMD ["/app/entrypoint.sh"]

## TODO - re-add the CLI build step
#FROM builder AS cli-export
## INTERMEDIATE EMPTY CONTAINER JUST TO DUMP CLI
## TODO - can we extract this during the build step? or only at runtime?
## DOCKER_BUILDKIT=1 docker build --target cli-export --output type=local,dest=./bin .
#COPY --from=builder /workspace/target/release/kvm-compose /dist/kvm-compose
# TODO - write docker compose build script to then take this cli and install it
