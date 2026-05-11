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

The following shows an example involving a Docker guest machine that can be run on TestbedOS similar to the [Minimal Working Example](welcome.md#minimal-working-example).

``` yaml
machines:
- name: nginx
  network:
    - switch: sw0
      gateway: 10.0.0.1
      mac: "00:00:00:00:00:01"
      ip: "10.0.0.10"
  docker:
    image: nginx:stable
    env_file: docker.env
    volumes:
      - source: ${PWD}/html  # the schema allows you to reference the project dir as ${PWD}
        target: /usr/share/nginx/html

- name: one-off
  network:
    - switch: sw0
      gateway: 10.0.0.1
      mac: "00:00:00:00:00:02"
      ip: "10.0.0.11"
  docker:
    image: ensignprojects/ubuntu-curl:20260426

network:
  ovn:
    switches:
      sw0:
        subnet: "10.0.0.0/24"
```

The `html` directory contains a single HTML page `index.html` with a single statement `This is an html page to be served by nginx docker container`. 
The `docker.env` file contains a single environment variable `THREE=3`.
All these necessary materials required to run the example can be found under the `examples/docker-single` in the [TestbedOS code repository](https://github.com/Bristol-Cyber-Security-Group/testbed-os).

After `kvm-compose up`, you can interact with the docker containers in the deployment by, for example, running 

``` shell
kvm-compose exec one-off curl 10.0.0.10
``` 

The results of running the command should return the content of the `index.html` page.

You can further interact with the deployment by using the normal Docker commands. For example, you can run

``` shell
sudo docker ps
```

You should be able to see both Docker container machine guests in the results with the name `docker-single-nginx` and `docker-single-one-off`.The container names follow the TestbedOS naming convention of `<project name>-<guest name>`.

Similarly, the following command can be run.
``` shell
sudo docker docker-single-nginx env
```
The output will include the environment variable `THREE=3` as specified in the above `kvm-compose.yaml` file and in the `docker.env` file.

## Scaling

As with libvirt guest machines, TestbedOS optionally provides a way to scale a Docker container guest from a single machine definition and create cloned instances of the containers during a deployment. For example, this can be used to spawn multiple instances of the container. The scaling is implemented via the `scaling` field in the `kvm-compose.yaml` file, and the following options are supported.

- **`count`**: the number of cloned instances to be created,
- **`interfaces`**: an optional field to define a list of networking-related configurations, shown in the following example.

### Working Deployment Example

The following deployment shows an example of the `scaling` parameter by defining `clones`. 
This deployment can be run similar to the [Minimal Working Example](welcome.md#minimal-working-example).

``` yaml
machines:

  - name: nginx
    network:
      - switch: sw0
        gateway: 10.0.0.1
        mac: "00:00:00:00:00:01"
        ip: "10.0.0.10"
    docker:
      image: nginx:stable
      env_file: docker.env
      environment:
        A: B
        TWO: "2"
      volumes:
        - source: ${PWD}/html
          target: /usr/share/nginx/html

  - name: nginx-clones
    docker:
      image: nginx:stable
      environment:
        B: A
        THREE: "3"
      volumes:
        - source: ${PWD}/clones_html
          target: /usr/share/nginx/html
      scaling:
        count: 2
        interfaces:
          sw0:
            clones: [0, 1]
            gateway: "10.0.0.1"
            ip_type: dynamic
            mac_range:
              from: "00:00:00:00:00:02"
              to: "00:00:00:00:00:03"

network:
  ovn:
    switches:

      sw0:
        subnet: "10.0.0.0/24"

      public:
        subnet: "172.16.1.0/24"
        ports:
          - name: ls-public
            localnet:
              network_name: public

    routers:

      lr0:
        ports:

          - name: lr0-sw0
            mac: "00:00:00:00:ff:01"
            gateway_ip: "10.0.0.1/24"
            switch: sw0

        static_routes:
          - prefix: "0.0.0.0/0"
            nexthop: "172.16.1.1"

        nat:
          - nat_type: snat
            external_ip: "172.16.1.200"
            logical_ip: "10.0.0.0/16"

        dhcp:
          - switch: sw0
            exclude_ips:
              from: "10.0.0.1"
              to: "10.0.0.20"

```

The `html` and the `clones_html` directories each contains a single HTML page `index.html` with a single statement of `This is an html page to be served by nginx docker container` and `This is an html page to be served by the clone nginx docker containers` respectively.
The `docker.env` file contains a single environment variable `TEST=123`.
All these necessary materials required to run the example can be found under the `examples/docker-scaling` directory in [the TestbedOS code repository](https://github.com/Bristol-Cyber-Security-Group/testbed-os).

After `kvm-compose up`, you can see the interaction between the clones of the main machine guest, for example, 

``` shell
kvm-compose exec nginx shell-command curl 10.0.0.21
```

or 

``` shell
kvm-compose exec nginx shell-command curl 10.0.0.22
```

These commands will output the content in the `clones_html/index.html` as the main machine `nginx` curls to the two counts of the cloned guests which have been assigned the IP addresses of `10.0.0.21` and `10.0.0.22`. In a similar manner, the following commands can be run.

``` shell
kvm-compose exec nginx-clones-0 shell-command 10.0.0.10 
```

or 

``` shell
kvm-compose exec nginx-clones-1 shell-command 10.0.0.10
```

These two commands show that the cloned machine guests with the name format `nginx-clones-<clone count>` curl to the main `nginx` machine guest. The output for these commands will yield the content in the `html/index.html` file.

You can also interact with the deployment using normal Docker commands. For example, you can run the following command to check the running containers after the `kvm-compose up` command.

``` shell
sudo docker ps 
```

This will show the container names `docker-scaling-nginx`, `docker-scaling-nginx-clones-0`, and `docker-scaling-nginx-clones-1` which follow the TestbedOS naming convention of `<project name>-<guest name>`.

Running the following Docker commands will show the different environment variables assigned to the respective guest and cloned guest according to the `kvm-compose.yaml` file. 

``` shell
sudo docker exec docker-scaling-nginx env
```

This command outputs the environment variables in the `nginx` machine guest of `TEST=123`, `A=B`, and `TWO=2`, as provided from the `docker.env` file and the `environment` parameter in the `kvm-compose.yaml` file. Similarly,

``` shell
sudo docker exec docker-scaling-nginx-clones-0 env
```

and 

``` shell
sudo docker exec docker-scaling-nginx-clones-1 env
```

These two commands show the environment variables `THREE=3` and `B=A` as provided by the `environment` field of the `kvm-compose.yaml` file to the `nginx-clones` machine guests.