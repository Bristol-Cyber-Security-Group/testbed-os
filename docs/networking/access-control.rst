Access Control
==============

OVN provides an extensive ACL implementation to apply security policies to the network.
By default, the network has no security policy so you are only limited by routing and NAT.

Please see the ovn-nbctl man pages for more information on how the ACL works.
Additionally, please see the ovn-sbctl man pages for the `Logical_Flow` table documentation for the match expressions to set up filters in your ACL.

In the `kvm-compose.yaml` file, there is an optional section called `acl` under the `ovn` element, which exposes the OVN ACL api.
We supply a shortcut to apply a deny all security policy with a low priority to reduce boilerplate code.

The yaml schema is as follows:

.. code-block:: yaml

    acl:
      apply_deny_all: false
      switches:
        sw0:
          - direction: to-lport
            priority: 10
            match: "ip4.src == '10.0.0.10'"
            action: allow

The `apply_deny_all` element defaults to false if not specified.

The `switches` element is optional, if specified you must specify valid logical switch names that are found in the switches section of the ovn network definition.
You can specify a list of ACL per switch.
Each ACL requires:

- `direction` : either
    - `to-lport`
    - `from-lport`
- `priority` : 0 to 32767 inclusive
- `match` : the filter to match to this rule
- `action` : one of
    - `allow-related`
    - `allow-stateless`
    - `allow`
    - `drop`
    - `pass`
    - `reject`

Creating and Designing Rules
----------------------------

The OVN ACL rules are very expressive, and some care is necessary to craft the right rule without any unintended side edge cases.
We have to consider the contents of the filter in the `match` section, the `direction` and the `action`.

Before setting up any security rules, you must consider how you open up your network traffic.
You will likely want to place a drop rule with a low priority as a base, so that you can then open up specifically the traffic flows you want.
To do this, you want to specify a generic drop rule with a match such as `"ip"`.
Alternatively, you can specifically block certain traffic flows when you generally want to allow all traffic.

Direction
*********

There are two directions, `to-lport` and `from-lport`.

to-lport specifies that the filtering will happen on traffic forwarded to a logical port.

from-lport specifies that the filtering will happen on traffic arriving from a logical port.

There is an important distinction when using these, when you are using the ACL just on a logical switch or when using a port group.
If you are just applying the ACL to a logical switch, for to-lport this rule will be applied as it arrives to the logical switch.
For from-lport this rule will be applied as it leaves the logical switch.
Similarly, for the port group, when the traffic is leaving or arriving at one of the logical ports in the port group.

So in the case of a drop rule, you can think of this as placing the ACL at the start of the packet's journey (from-lport) or at the end of it's journey (to-lport).
This means if you use to-lport, the packet will still travel all the way up to the logical switch (say there was many hops to reach the destination).
Whereas if you use from-lport, the packet will be immediately filtered as it leaves the logical port.

So if you want to create a drop all traffic rule for a logical switch to stop traffic coming in, you will want to use a `to-lport` with a drop.

Match
*****

The match section is the filter that OVN will apply on every packet.
There is an extensive syntax for this, so it is recommended to see the ovn-nbctl documentation for the full list and descriptions.
For this documentation, we will discuss how to create a basic filter.
Note that these filters may not be the most efficient, but the should be clear in their purpose.

If you want to drop all traffic travelling to logical ports in a specific logical switch (sw0):

.. code-block:: yaml

    sw0:
      - direction: to-lport
        priority: 1
        match: "ip4"
        action: drop

The use of `to-lport` means that the filtering will be triggered when the traffic is destined to the port.
We match on just `ip4` to catch any ipv4 traffic (you may want to also block ip6 if that is being used).
If the filter is matched, then we apply the action which is drop in this case.

If you want to allow traffic with a specific source and destination address:

.. code-block:: yaml

    sw0:
      - direction: to-lport
        priority: 2
        match: "ip4 && ip4.src == 10.0.0.11 && ip4.dst == 10.0.0.12"
        action: allow

Similar to the drop rule, we also specify the source and destination explicitly.
You can also specify a subnet rather than a specific ip such as `10.0.0.0/24` which is valid syntax.

By combining rules like these, you can shape your traffic in complex ways.
These examples would ultimately be time consuming for a large network with complex security requirements.
It is recommended to both look at the OVN ACL documentation for more sophisticated syntax, but also to experiment with rules.
You can quickly iterate with rules on an existing testbed deployment with the following command:

.. code-block:: shell

    kvm-compose up -a

Which will remove the existing rules and re-apply the rules in the yaml file.
This command will not attempt to rebuild guests or the network.

Additionally, you can also test your rules from guests by either using the ping command to the specific ip address of a guest.
Or, you can use netcat for tcp connections since ping would use icmp traffic.
You can do this with:

.. code-block:: shell

    # server with ip 10.0.0.10
    nc -l -p 8000

    # client
    nc 10.0.0.10 8000

Then you can type in any message, press enter and you will see this message appear on the server, if the ACL allowed the traffic.

Action
******

TODO


