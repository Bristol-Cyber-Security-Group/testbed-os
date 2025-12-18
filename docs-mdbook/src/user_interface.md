# TestbedOS User Interface

## CLI

`kvm-compose` is a binary CLI tool written in rust.
It uses the serde library to deserialise and parse the |kvm-compose.yaml| file to start building a state representation in memory.
The state representation, once enumerated with details from the |kvm-compose.yaml| and |kvm-compose-config.json|, will be also serialised with serde into the state JSON file to be used with |orchestration|.
Along with the state JSON, the artefacts are also generated.
Note: this CLI tool uses the APIs on the |testbed server| to execute the testbed processes.