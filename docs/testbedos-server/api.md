====================
TestbedOS Server API
====================

The root of the REST API for the server can be found at `/api/` on the servers url, the default is `localhost:3355`.

Ideally we would have OpenAPI schema generation, this is a TODO so the following is a brief introduction to the API endpoints.
The POST/PUT variants accept a JSON payload to control specific actions on the server.

:`/api/config`: which is for editing the `kvm-compose-config`.
    This supports GET and POST, to get and update respectively.

:`/api/deployments`: which is for controlling the testbed deployments.
    This supports GET and POST, to get and update the deployments respectively.

:`/api/deployments/:name`: which is for controlling specific testbed deployments.
    The `:name` will be the name of the deployment.
    This supports GET, DELETE and PUT, to get, delete and update the specific deployment respectively.

:`/api/deployments/:name/action`: which is for applying commands to testbed deployments.
    This supports POST, to apply a specific testbed action such as generate-artefacts.

:`/api/deployments/:name/state`: which is for interacting with an existing state for the deployment.
    This supports GET, to get the state json file for the deployment.
