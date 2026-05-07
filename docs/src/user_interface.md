---
title: INTERFACES
section: 1
---

# TestbedOS User Interface

TestbedOS provides two kinds of user interface for users after installation: the [Command Line Interface (CLI)](#cli) and the [Graphical User Interface(GUI)](#gui). Here, we give a more detailed description of both user interfaces. 

## CLI

We have previously seen the CLI in action through the [Minimal Working Example](welcome.md#minimal-working-example) where we run the `kvm-compose` commands to interact with the MWE deployment.
`kvm-compose` is a binary CLI tool written in Rust.
It uses the `serde` library to deserialise and parse the `kvm-compose.yaml` file to start building a state representation in memory.
The state representation, once enumerated with details from the `kvm-compose.yaml` and `kvm-compose-config.json`, will be also serialised with serde into the state JSON file to be used with [orchestration](orchestration.md).
Along with the state JSON, the artefacts are also generated.
Note: this CLI tool uses the APIs on [the TestbedOS server](orchestration.md) to execute the orchestration tasks for a deployment.

## GUI

The GUI is a web application that is hosted by the TestbedOS server at http://localhost:3355/gui. When accessing this URL from a browser, the homepage of the TestbedOS GUI is shown.

On the homepage of the GUI, a new deployment can be created by first clicking `List` under the `Deployments` tab.
Here, the list of the previously created deployments is shown.

To create a new deployment, simply click the `Create New Deployment` button at the top of the list of deployments, if there is any.
There will be a form that asks for the name of the project (deployment) and a YAML for the deployment, which is the specification file that we have previously seen in the [Minimal Working Example](welcome.md#minimal-working-example).
Please fill those in on the form and once that is done, simply click `Create Deployment` to complete the creation of the deployment.
The deployment name as previously provided to the form will appear in the list of existing deployment.


### Running the `kvm-compose` Commands and Tooling

The GUI also supports running commands such as orchestration and analysis tools, directly from the GUI to replicate the CLI tool `kvm-compose`.

If a deployment from the list of existing deployments under the `Deployments` tab and then `List`, then you will be given a page with a list of buttons to interact with the deployment, including `View Yaml`, `View State`, `View Topology`, `Run command`, `Resource Monitoring`, and `Delete`.
 
Clicking the `Run command` button will provide the GUI tools that you can use to execute the `kvm-compose` commands that we can run on the deployment on the left side of the page, similar to the commands that we have run in [Minimal Working Example](welcome.md#minimal-working-example) on the CLI, including the following commands.
- `kvm-compose generate-artefacts`
- `kvm-compose up`
- `kvm-compose down`
- `kvm-compose clear-artefacts`
- [`kvm-compose exec ...`](user_interface_exec.md) 

On the left side of the page, the GUI also provides a pseudo terminal to emulate the CLI behaviour to display the logs of commands as they appear from the TestbedOS server.
Note that the logging is exactly the same logging with what you would see if the command is executed on the CLI, the GUI logging is mostly a general indication of what is happening so that there is some activity to be seen by the user.
For more detailed logging, especially if there is a problem, please refer to the server logs for more information.

### Authentication

There are currently no users and login for the GUI.
While the testbed is un-authenticated, we have not added this but in a future release when the endpoints are secured we will also add users to the GUI.

### GUI Internals

The TestbedOS GUI uses serverside HTML rendering using [teradocs](https://keats.github.io/tera/docs/), and a combination of bootstrap and jQuery.
This has been kept to a minimum to keep the site simple, functional, and easy to maintain.
The dynamic portions of the GUI utilise the TestbedOS's API to get information about the deployments.


Command running is supported via a websocket connection from the browser to the testbed server api.
As commands are executed on the TestbedOS server, the server pushes logs to the GUI back over the websocket.
To keep things simple, the command running from the GUI is essentially a wrapper around the CLI code.
This means that the same code the CLI would use to trigger a command to the server is used in a special endpoint just for the GUI.
In essence, this special endpoint on the server will open up another websocket to the server for the original command running code path.
This was the path chosen in favour of partially re-implementing the command running in javascript, using endpoints to control the filesystem rather than the browser.
Another possibility would have been to run the CLI as a subprocess, but we are moving away from subprocess use.