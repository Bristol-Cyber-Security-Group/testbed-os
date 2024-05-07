=====
Usage
=====
kvm-compose [--input] [--project-name] [-v|--verbosity] [--no-ask] [-h|--help] [-V|--version] <SUBCOMMANDS>

Description
-----------

    **kvm-compose** must be run in the desired project directory, where the |kvm-compose.yaml| (or whichever file name used with **--input=**) file exists.

Options
-------

      --input <INPUT>
          Configuration file [default: kvm-compose.yaml]

      --project-name <PROJECT_NAME>
          Defaults to the current folder name

  -v, --verbosity <VERBOSITY>

      --no-ask
          Suppress (accept) continue prompts

  -l, --local-command
          Choose to use commands with or without the testbed server, not all work without the server

      --server-connection <SERVER_CONNECTION>
          Specify the URL to the testbed server [default: http://localhost:3355/]

  -h, --help
          Print help

  -V, --version
          Print version


Subcommands
-----------

  generate-artefacts
        Create all artefacts for virtual devices in the current configuration
  clear-artefacts
        Destroy all artefacts for the current configuration
  cloud-images
        List supported cloud images
  setup-config
        Setup kvm compose config
  deployment
        Control deployments on the testbed server
  up
        Deploy the test case
  down
        Undeploy the test case
  snapshot
        Snapshot guests
  analysis-tools
        Analysis tools
  snapshot-testbed
        Prepare all artefacts in deployment to be shared and used in another testbed
  help
        Print this message or the help of the given subcommand(s)

Subcommand - up
---------------

Deploy the test case

Usage: kvm-compose up [OPTIONS]

Options:
  -p, --provision      Force regenerate guest images
  -r, --rerun-scripts  Force rerunning use specified guest setup scripts
  -h, --help           Print help

Subcommand - down
-----------------

Undeploy the test case

Usage: kvm-compose down

Options:
  -h, --help  Print help


Subcommand - deployment
-----------------------

Control deployments on the testbed server

Usage: kvm-compose deployment <COMMAND>

Commands:
  create
        Create a deployment
  destroy
        Destroy a deployment
  list
        List all deployments
  info
        This is the name of the deployment that is passed to the deployment commands
  help
        Print this message or the help of the given subcommand(s)

Subcommand - snapshot
---------------------

Snapshot guests

Usage: kvm-compose snapshot <COMMAND>

Commands:
  create
        Create a snapshot for a guest
  delete
        Destroy a snapshot
  info
        Get information about a guest and it's snapshots
  list
        List guest snapshots
  restore
        Restore guest from snapshot or all guests from latest snapshot, if any
  help
        Print this message or the help of the given subcommand(s)



.. |kvm-compose.yaml| replace:: :ref:`kvm-compose-yaml/index:kvm-compose Yaml`
