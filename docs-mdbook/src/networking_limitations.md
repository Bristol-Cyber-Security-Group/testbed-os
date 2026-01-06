# Limitations

However, the testbed abstraction over the OVN api is somewhat opinionated in this version of the testbed.
For example, we do not expect the user to need high availability configuration in OVN.
Additionally, to avoid overly pushing the testbed opinion on a network configuration, we have not provided configurable DNS.
This is explained more further down.
For very complex networks with very specific requirements, we need to assess how this impacts the testbed's API so that it remains general.
In future versions we look to open up the API to include more configurability of the network.
All these constraints that we have applied are validated in code, so if there is something we don't support but OVN does, then that will be rejected in the yaml parsing phase.