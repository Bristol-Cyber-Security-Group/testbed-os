---
title: KVM-COMPOSE-YAML
section: 1
---

# kvm-compose Yaml

This YAML document contains the high level description of the intended testbed configuration.
The document must exist before executing any [kvm-compose](../index.md#kvm-compose) commands.

This document does not express any low level details about how the testbed will be deployed or load balancing of guests and network topology.
The low level details are derived from a combination of the kvm-compose yaml file and the [state json](../../testbed-config/index.md#testbed-config) file.
