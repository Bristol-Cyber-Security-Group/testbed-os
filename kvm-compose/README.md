# Info

This folder contains the rust code for the project.
It is split up into three crates:

- kvm-compose
- kvm-compose-cli
- kvm-compose-schemas
- testbedos-server

The general data schemas used in all parts of the testbed are kept in `kvm-compose-schemas`.

The crate `kvm-compose` contains the core testbed logic as a library.

The crate `kvm-compose-cli` contains the CLI for the project.

The crate `testbedos-server` contains the server binary, which can be run in server or client mode.

