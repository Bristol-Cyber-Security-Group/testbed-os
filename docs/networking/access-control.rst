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

