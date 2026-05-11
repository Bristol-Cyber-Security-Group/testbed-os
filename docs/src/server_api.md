---
title: SERVER-API
section: 1
---

# TestbedOS Server API

The root of the REST API for the server can be found at `/api/` on the server's URL, the default is `localhost:3355`.

Ideally we would have OpenAPI schema generation, this is a TODO so the following is a brief introduction to the API endpoints.
The `POST` and `PUT` variants accept a JSON payload to control specific actions on the server.

The list of API endpoints is divided into two categories depending if the API is provided by the `Main` TestbedOS host's server or a client TestbedOS host's server. Please see the [TestbedOS Clustering Mode](clustering_mode.md) for more information on the types of TestbedOS host.

## Main TestbedOS Host Server API

The following is the list of API endpoints provided by the `Main` TestbedOS host's server.

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

`/api/metrics/host`
: For fetching the performance metrics for a host, specifically CPU and memory usage. This only supports `GET`.

`/api/metrics/state`
: For checking if the Docker containers where the [resource monitoring stack](resource_monitoring.md) live are up and running. This only supports `GET`.

`/api/metrics/guest/<project>/<name>`
: For fetching the CPU and memory usage of a guest with the identifier `<name>` in a TestbedOS project with the identifier `<project>`. This only supports `GET`.

`/api/metrics/dashboard/<project>`
: For fetching the HTML page for the resource monitoring data dashboard for a TestbedOS project with the identifier `<project>`. This only supports `GET`.

`/api/metrics/prometheus/hosts`
: For fetching the performance metrics (CPU and memory usage) for all TestbedOS hosts. This only supports `GET`.

`/api/metrics/prometheus/libvirt`
: For fetching the performance metrics (CPU and memory usage) for all [libvirt guests](libvirt.md) for Prometheus. This only supports `GET`.

`/api/metrics/prometheus/android`
: For fetching the performance metrics (CPU and memory usage) for all [AVD guests](avd.md) for Prometheus. This only supports `GET`.

`/api/metrics/prometheus/docker`
: For fetching the performance metrics (CPU and memory usage) for all [Docker container guests](docker.md) for Prometheus. This only supports `GET`.

## Client Host Server API

The following is the list of API endpoints provided by a client TestbedOS host's server.

`/api/config/status`
: For fetching the status of the client TestbedOS host and this only supports `GET`. At the moment this only returns 200 to show that this host is running.

`/api/metrics/host`
: For fetching the performance metrics for a host, specifically CPU and memory usage, to a client TestbedOS host. This only supports `GET`.

`/api/metrics/guest/<project>/<name>`
: For fetching the CPU and memory usage of a guest with the identifier `<name>` in a TestbedOS project with the identifier `<project>` to a client TestbedOS host. This only supports `GET`.