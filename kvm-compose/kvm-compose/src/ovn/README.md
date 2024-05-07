# Testbed OVN

This abstraction over OVN is somewhat opinionated in the way we want to create things.
The create commands will behave in a certain way rather than provide a one to one abstraction on the set of commands in OVN.

In the future, we can look to create a one to one mapping of the OVN API and then build opinionated helper functions on top.
This means we can use our opinionated functions and go down into the one to one API when necessary.
