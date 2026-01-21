# TestbedOS Server API

The root of the REST API for the server can be found at `/api/` on the server's URL, the default is `localhost:3355`.

Ideally we would have OpenAPI schema generation, this is a TODO so the following is a brief introduction to the API endpoints.
The `POST` and `PUT` variants accept a JSON payload to control specific actions on the server.

`/api/config/cluster`
: For fetching or editing [the `kvm-compose-config` configuration](configurations.md#overall-configurations). This supports `GET` and `POST`, to get and update respectively the configuration.

`/api/config/host`
: For fetching or editing [the `host.json` configuration](configurations.md#testbedos-host-configuration). This supports `GET` and `POST`, to get and update respectively the configuration.

`/api/config/status`
: For fetching the status of the host and this only supports `GET`. At the moment this only returns 200 to show that this host is running.

`/api/config/default`
: For fetching the default [`host.json` configuration](configurations.md#testbedos-host-configurations) and this only supports `GET`.

`/api/config/usergroupqemu`
: For fetching the qemu user group of the host and this only supports `GET`.

`/api/cluster`
: For joining a cluster and this only supports `POST`.

`/api/cluster/<name>`
: For checking if it is part of a cluster with the identified `<name>` and this only supports `GET`.

`/api/validate/yaml`
: For checking if a given YAML is syntactically valid according to the [TestbedOS schema](schema.md) and this only supports `POST`.

`/api/validate/projectname/`
: For checking if a project name is valid, i.e., if the project name is not an empty string as well as if the project name is available and not taken. This only supports `POST`.

`/api/deployments`
: For fetching the list of deployments or creating a new deployment, supported respectively through `GET` and `POST`, to get and update the deployments respectively.

`/api/active-deployments`
: For fetching the list of active deployments, i.e., deployments with an 'Up' state. This only supports `GET`.

`/api/deployments/<name>`
: For performing a specific action on a deployment with the identifier `<name>` based on the following methods.
: `GET`: For fetching the deployment's project path on the TestbedOS host file system.
: `DELETE`: For removing (deleting) the JSON configuration files in the deployment's project folder and from the list of deployments.
: `POST`: For updating the deployment JSON configuration file in the deployment's project folder.

`/api/deployments/<name>/state`
: For fetching and updating the state of a deployment with the identifier `name` through the supported `GET` and `POST` methods respectively.
<!-- `/api/deployments/:name/action`
: which is for applying commands to testbed deployments.
    This supports POST, to apply a specific testbed action such as generate-artefacts. -->

`/api/metrics/dashboard/<project>`
:

`/api/metrics/host`
: For fetching the performance metrics for a host, specifically CPU and memory usage. This only supports `GET`.

`/api/metrics/state`
:

`/api/metrics/guest/<project>/<name>`
:

`/api/metrics/prometheus/hosts`
:

`/api/metrics/prometheus/libvirt`
:

`/api/metrics/prometheus/android`
:

`/api/metrics/prometheus/docker`
:

`/api/orchestration`
:


## Client Server API

`/api/config/status`
:

`/api/metrics/host`
:

`/api/metrics/guest/<project>/<name>`
: