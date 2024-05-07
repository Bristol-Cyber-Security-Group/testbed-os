# Service Clients

This client contains implementations of clients to connect to the various services the testbed uses.
These services are local unix sockets, so rather than using sub processes to run commands, we can communicate via sockets.

# Tests

The tests in this crate do require the services to be running otherwise they will fail.
So these are somewhat integration tests as well.

# Docker

Writing the socket against the api version v1.43 api reference https://docs.docker.com/engine/api/v1.43/


