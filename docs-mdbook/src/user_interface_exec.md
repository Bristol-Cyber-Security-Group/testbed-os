# Exec Subcommands

The `kvm-compose exec ...` commands offers a way to run tasks against the guests on both the CLI and the GUI.
This facilitates accessing the guest for the user without dealing with the intricacies of the TestbedOS coding. 
We also prepare built-in tools against the guests.
We have previously seen one example of an exec command in the [Minimal Working Example](welcome.md#minimal-working-example).

The following is a recap from [the `kvm-compose` command structure](user_interface_kvm_compose.md#command-structure) with the `exec` command. The `exec` command is then followed by the arguments for the variations of the `exec` command accordingly. Please refer to the following sections for each variation and the `--help` flag is also available for each of subcommand for more information.

``` bash
kvm-compose exec <guest name> <SUBCOMMANDS>
```
