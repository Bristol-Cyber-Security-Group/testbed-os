# TestbedOS Server Architecture

Here we explain in more details how the TestbedOS server works under the hood and its architecture. 
The server is running as a normal HTTP server at the moment, we have aspirations to run this in a socket so that we can protect it with user permissions.
The architectural layout of a TestbedOS deployment with the TestbedOS server of each host is shown in the diagram below.

![TestbedOS Deployment Architecture Diagram](testbedos_architecture.png)
<!-- <p align="center">
<img src="./testbedos_architecture.png" />
</p> -->

## `systemd` Service

The TestbedOS server is set up as a `systemd` service called `testbedos-server.service`.

The service is not set to start by default so you must run `sudo systemctl start testbedos-server.service`. You can make the service to start automatically at boot with `sudo systemctl enable --now testbedos-server.service`.
During the TestbedOS [installation](welcome.md#quick-installation) and [uninstallation](welcome.md#uninstallation) process, the `testbedos-server` `systemd` service is enabled and disabled for you.

Note that this service will use the default port (port 3355) so if you are developing the server you must disable/stop the systemd service.

## Server-to-Server and Client-to-Server Communication

The unidirectional communication from the TestbedOS server of [the `Main` TestbedOS host](configurations.md#singleton-mode-or-the-main-host) to the TestbedOS server of each [`Client` TestbedOS host](configurations.md#client-host) in the clustering mode occurs through a SSH channel. The server-to-server communication happens during the setup and orchestration stages where the server of the `Main` TestbedOS host manages and keeps track the hosts in the cluster. 

Each TestbedOS host (regardless of `Main` or `Client` in the clustering mode) communicates with their corresponding server on their `localhost` through an API that each server provides. The list of available API endpoints is provided in [TestbedOS Server API](server_api.md) and it supports the different HTTP verbs such as `GET` and `POST`. The client-to-server communication initially first occurs through HTTP, which is then upgraded to WebSocket. 

## Database Provider

The TestbedOS server needs a database to store and keep track the state of deployments.
Currently, the server is using a file-based database that is read or written on demand by the server.
The storage location for this database is in `/var/lib/testbedos/deployments/` and the details for each deployment is in a JSON file.

Each JSON file will generally include the following fields:
- `name`: the name of the TestbedOS deployment
- `project_location`: the path on the TestbedOS host to the folder where the files of the deployment live
- `state`: the state of the deployment. Values of this field include `"up"`, `"down"`, `"running"`, or `"failed"`, and
- `last_action_uuid`: not currently in use but has a default value of `null`.

In the future, if necessary, we can implement a sqlite database.

## Database Update Post-Orchestration

The TestbedOS server runs the commands as required during the orchestration when setting up a deployment depending on the "deployment action" requested by the TestbedOS client.
Please see [TestbedOS Architecture](orchestration.md) for more details.
Once the orchestration has finished, this updates the database of the server on the status of a deployment with the outcome of the orchestration.

## Error Handling

Currently the server has basic error handling which is passed to the user through HTTP error codes.
We have a generic wrapper that returns JSON with the error code and message, usually an error code of `500` if some business logic does not work.
Some of the errors are not yet fully propagated to the user, this needs some testing of different fail states.
The states of the deployment are recorded in the JSON file of the deployment in the database in `/var/lib/testbedos/deployments/`, as another form of error recording.
The error codes for each action command is also recorded in the log data file for each log file.

## Server Logging

Currently the TestbedOS server logs are sent to a single file in `/var/lib/testbedos/logs/`, which is overwritten on each server restart.
Each log file is named with the format `server.log.<date>` where `<date>` is the date when the log file is created.
The log files contain all the client request information with timestamps.
There will be log cleaning where the log files will be deleted if they are more than a week old.

## Server Log Streaming

The TestbedOS server provides an endpoint for websockets, specifically to stream the output of [a `kvm-compose` command](user_interface_kvm_compose.md) interacting with the deployment as the command execution can be long-running. In the [CLI](user_interface.md#cli) and the [GUI](user_interface.md#gui) the output from the server is in the form of `INFO` or `ERROR` statements, which can be seen when running a specific `kvm-compose` command. 
The `kvm-compose` commands use [the API offered by the server](server_api.md) for execution and deployment control.
Currently the websocket is solely one directional, from the TestbedOS server to the client.

For example, a `kvm-compose up` command from the CLI will make a request to the server which will set the state of the deployment to `"running"`, dispatch a blocking thread that handles [orchestration](orchestration.md), and return a UUID.
This UUID is then used by the TestbedOS client to create a websocket session which the server will find the log file and stream line by line until the end of the file.
If the command is still ongoing but the stream has reached the end of the file, the websocket will continue to poll the log file for a new line until the state of the command has changed from `"running"`.
This way, since usually [the orchestration](orchestration.md) is slower than streaming the logs, the client will keep the websocket open and we avoid time outs on the usual REST API calls.
The CLI will then check the state of the deployment on the REST API to see if the command worked or not as there is a success and fail state for each command.

For snapshots, the process is the same as above but with one further check to the log API.
The log API also captures the error code from the command the server executed.
For example, a `kvm-compose snapshot` command failed with error code 1 - the log API for that UUID will have the error code that the CLI checks to inform the user on the status.

This second step is not currently used for the main commands on the server like `kvm-compose up` and `kvm-compose down` but it can be in the future and for other new commands.
This error code value in the log data is easily set by the thread if needed, but it will be left as `None` or `null` for now.

## Cluster Management

The TestbedOS server will automatically manage [the cluster](clustering_mode.md) for you.
When a TestbedOS server is run in `"client"` mode, and makes a successful connection to the `"main"` TestbedOS host, the `"main"` host will keep track of the clients that have "joined" the cluster.
The `"main"` host will populate the `kvm-compose-config.json` file in the `/var/lib/testbedos/config/` folder dynamically, with the respective `host.json` files of each `"client"` host.
On connection of the `"client"` host, the host will push its own `host.json` to the `"main"` host so the `"main"` host knows how to use this client in its TestbedOS deployments.

After the join request from the `"client"` host, the `"main"` host will periodically make a request to the `"client"` host to see if it is still available.
If it is not available, the `"main"` host will remove it from its cluster configuration `kvm-compose-config.json`.
Additionally, the `"client"` hosts will also periodically make a request to the `"main"` host to see if it is still available.

## Developer Notes

The TestbedOS server is implemented with the Axum web framework for Rust.
All code for the server is solely dealing with the API handlers and the database connection.
Anything else, such as the business logic that exists in the `kvm-compose` library, is imported.

The database provider is implemented with `trait`s, so introducing a new database just requires implementing the `trait` functions.

This database connection is shared between all handlers and is wrapped with atomic read/write locks to ensure thread safety and prevent race conditions on the database (especially for the file based provider).

The `setup.sh` script will enable the server as a daemon, so if you want to run the server for development you will have to stop the service before you run you development version, as it shares the same port and the CLI will be connecting to this port.
Ideally we should make the port editable through environment variables for development purposes.

The server has a development mode, where there are checks for cargo's debug mode.
This will change the logging level, and enable hot reloading of templates i.e. HTML for the GUI.
You can do this via (provide the server with root permissions):

```bash
# make sure youre in the root of the server crate i.e.
# testbed-os/kvm-compose/testbedos-server/
# then run the following
sudo -E bash -c  'cargo run -- main' $USER
```

