# Disclaimer and Limitations

TestbedOS's abstraction over the OVN networking API is somewhat opinionated in this version.
For example, we do not expect the user to need the high availability configuration in OVN.
Additionally, to avoid overly pushing the TestbedOS's networking paradigm on a network configuration, we have not provided configurable DNS as explained in [DNS](networking.md#dns).
For very complex networks with very specific requirements, we need to assess how this impacts the TestbedOS's API so that it remains general.
In future versions we look to open up the API to include more configurability of the network.
All these constraints that we have applied are validated in code, so if there is something we don't support but OVN does, then that will be rejected in the parsing phase of the `kvm-compose.yaml` file.