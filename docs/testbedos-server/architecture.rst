=============================
TestbedOS Server Architecture
=============================

REST API
--------
See API document.

Systemd Service
---------------
The testbedos-server is setup as a service with systemd, its called `testbedos-server.service`.
It is not set to start by default so you must run `sudo systemctl start testbedos-server.service`, but you can make this autostart with `sudo systemctl enable testbedos-server.service` manually.
The setup script and teardown script will manage the install and uninstall for you.

The server is running as a normal HTTP server at the moment, we have aspirations to run this in a socket so that we can protect it with user permissions.

Note that this service will use the default port so if you are developing the server you must disable/stop the systemd service.

Database Provider
-----------------

The server needs a database to store state.
Currently, the server is using a file based architecture that is read/write on demand by the server.
The storage location for this database is in `/var/lib/testbedos/deployments/`.
In the future, if necessary, we can implement a sqlite database.

Orchestration
-------------

The server will run the orchestration commands depending on the "deployment action" requested by the client.
This deployment action will be passed on to the orchestration tool used internally, see orchestration architecture document for more details.
The execution of this orchestration tool happens inside a long running thread.
Once the orchestration tool has finished, this long running thread will update the database of the server with the outcome of the orchestration.

Error Handling
--------------

Currently the server has basic error handling which is passed to the user through HTTP error codes.
We have a generic wrapper that returns JSON with the error code and message, usually 500 errors if some business logic does not work.
Some of the errors are not yet fully propagated to the user, this needs some testing of different fail states.

The states of the deployment is recorded in the database, as another form of error recording.
The error codes for each action command is also recorded in the log data file for each log file.

Logging
-------

Currently the logging is sent to a single file in `/var/lib/testbedos/logs/`, which is overwritten by each server restart.
The log contains all the request information with timestamps.
There is an aspiration to introduce log rotation and cleanup as we expect the server to be running for long periods of time on dedicated hardware.

The server also stores logs for each command actioned on a deployment.
This is stored in `/var/lib/testbedos/logs/orchestration/<iso-date-time>-<project-name>.log`.
There is also a json file with a reference to all the logs for the specific deployment in `/var/lib/testbedos/deployments/<project-name>-logs.json`, that contains the path all the logs in the previous example path.
This is a hashmap/keyvalue store of `uuid:{log_path, error_code}` where the uuid key has a hashmap/keyvalue of the path to the log file and the error code associated with that log.


Log Streaming
-------------

The server provides an endpoint for websockets, specifically to stream the logs of a specific deployment action.
For example, an up command from the CLI will make a request to the server which will set the state of the deployment to "running", dispatch a blocking thread that handles orchestration and return a uuid.
This uuid is then used by the client to create a websocket session which the server will find the log file and stream line by line until the end of the file.
If the command is still ongoing but the stream has reached the end of the file, the websocket will continue to poll the log file for a new line until the state of the command has changed from "running".
This way, since usually the orchestration is slower than streaming the logs, the client will keep the websocket open and we avoid time outs on the usual REST calls.
The CLI will then check the state of the deployment on the REST api to see if the command worked or not as there is a success and fail state for each command.

For snapshots, the process is the same as above but with one further check to the log API.
The log API also captures the error code from the command the server executed.
For example, a snapshot command failed with error code 1 - the log api for that uuid will have the error code that the CLI checks to inform the user on the status.

This second step is not currently used for the main commands on the server like UP/DOWN but it can be in the future and for other new commands.
This error code value in the log data easily be set by the thread if needed, it will be left as None/null for now.

Cluster Management
------------------

The testbed server will automatically manage the testbed cluster for you.
When a testbed server is run in client mode, and makes a successful connection to the master testbed, the master testbed will keep track of the clients that have "joined" the cluster.
The master will populate the `kvm-compose-config.json` file in the testbed config folder dynamically, with the respective `host.json` files of each client.
On connection of the client, the client will push it's own `host.json` to the master so the master knows how to use this client in it's testbed deployments.

After the join request from the client, the master will periodically make a request to the client to see if it is still available.
If it is not available, the master will remove it from it's cluster configuration `kvm-compose-config.json`.
Additionally, the clients will also periodically make a request to the master to see if it is still available.

Developer Notes
---------------

The server is implemented with the Axum web framework.
All code for the server is solely dealing with the API handlers and database connection.
Anything else, such as the business logic exists in the `kvm-compose` library which is imported.

The database provider is implemented with traits, so introducing a new database just requires implementing the trait functions.

This database connection is shared between all handlers and is wrapped with atomic read/write locks to ensure thread safety and prevent race conditions on the database (especially for the file based provider).

The `setup.sh` script will enable the server as a daemon, so if you want to run the server for development you will have to stop the service before you run you development version, as it shares the same port and the CLI will be connecting to this port.
Ideally we should make the port editable through environment variables for development purposes.

The server has a development mode, where there are checks for cargo's debug mode.
This will change the logging level, and enable hot reloading of templates i.e. HTML for the GUI.
You can do this via (provide the server with root permissions):

``bash
# make sure youre in the root of the server crate i.e.
# testbed-os/kvm-compose/testbedos-server/
# then run the following
sudo -E bash -c  'cargo run -- master' $USER
``

