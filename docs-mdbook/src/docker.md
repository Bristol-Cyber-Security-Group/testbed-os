# TestbedOS Docker Guest Machines

TestbedOS also provides Docker containers as guest machines under the `docker` field in the `kvm-compose.yaml` file. The following lists the available options in the `docker` section:

- **`image`**: the container image to be used,
- **`command`**: a command to override the container image's `CMD`, i.e., the default command arguments or the default command which is executed when the container is run,
- **`entrypoint`**: an entry point to override the container image's `ENTRYPOINT`, i.e., the executable that runs when the container is run,
- **`environment`**: a key-value mapping of environment variables to give to the container,
- **`env_file`**: a file containing the environment variables, similar to the **`environment`** option,
- **`volumes`**: a list of mounting points from the TestbedOS to the container
- **`privileged`**: an option to allow the container to be run as a privileged guest, and
- **`device`**: a list of mounting points from a TestbedOS host device, typically in `/dev`, to the container, similar to `volumes`.

Internally, these options are passed from TestbedOS to `docker run`, for more information on these options please refer to [docker container run](https://docs.docker.com/reference/cli/docker/container/run/).

## Minimal Working Example

For example, the following shows an example that can be run on TestbedOS similar to the [Minimal Working Example](welcome.md#minimal-working-example).

``` yaml

```

## Scaling

As with libvirt guest machines, TestbedOS optionally provides a way to scale a Docker container guest from a single machine definition and create cloned instances of the containers during a deployment. For example, this can be used to spawn multiple instances of the container. The scaling is implemented via the `scaling` filed in the `kvm-compose.yaml` file, and the following options are supported.

- **`count`**: the number of cloned instances to be created,
- **`interfaces`**: an optional field to define a list of networking-related configurations, shown in the following example.

``` yaml

```