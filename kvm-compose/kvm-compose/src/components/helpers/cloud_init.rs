use std::collections::{BTreeMap};
use crate::assets::Assets;
use serde::Serialize;
use std::fmt;
use std::fmt::Formatter;
use kvm_compose_schemas::kvm_compose_yaml::{MachineNetwork};
use crate::components::helpers::xml::TEMPLATES;

// The functions in this file simply create a string representation of the cloud-init metadata files
// to be passed to the "virt-install" command

#[derive(Serialize, Clone, Debug)]
struct MetaData {
    #[serde(rename = "instance-id")]
    instance_id: String,
    #[serde(rename = "local-hostname")]
    local_hostname: String,
    public_ssh_key: String,
    environment: BTreeMap<String, String>,
    // tb_set_ip: String,
}

impl fmt::Display for MetaData {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(&serde_json::to_string_pretty(&self).unwrap())
            .expect("Cloud init MetaData to string failed");
        Ok(())
    }
}

pub fn create_meta_data(
    hostname: String,
    public_ssh_key: String,
    environment: BTreeMap<String,String>,
    // tb_set_ip: String,
) -> String {
    format!(
        "{}",
        MetaData {
            instance_id: hostname.clone(),
            local_hostname: hostname,
            public_ssh_key,
            environment,
            // tb_set_ip,
        }
    )
}

pub fn create_user_data() -> Vec<u8> {
    Assets::get("cloud_init.yaml").unwrap().data.into_owned()
}

/// This struct represents one ethernet definition in the cloud-init network config yaml
#[derive(Serialize)]
struct NetworkConfigEthernet {
    name: String,
    mac_address: String,
    dhcp4: bool,
    addresses: Option<String>,
    routes: Option<String>,
    nameservers: String,
}

pub fn create_network_config(
    network_definition: &Option<Vec<MachineNetwork>>,
) -> anyhow::Result<Vec<u8>> {
    let mut tera_context = tera::Context::new();
    let mut interfaces = Vec::new();
    if let Some(network_definition) = network_definition {
        for (idx, interface) in network_definition.iter().enumerate() {
            let interface_name = format!("ens{idx}");
            let dhcp = if interface.ip.eq(&"dynamic".to_string()) {
                true
            } else {
                false
            };
            let ip = if interface.ip.eq(&"dynamic".to_string()) {
                None
            } else {
                Some(interface.ip.clone())
            };

            // we can only define the default route once, so only apply on first interface
            // TODO - when interfaces are on different logical switches, the gateway may need to be
            //  different - maybe don't use default?
            let routes = if idx > 0 {
                None
            } else {
                interface.gateway.clone()
            };
            
            let yaml_def = NetworkConfigEthernet {
                name: interface_name,
                mac_address: interface.mac.clone(),
                dhcp4: dhcp,
                addresses: ip,
                routes,
                nameservers: "1.1.1.1".to_string(),
            };
            interfaces.push(yaml_def);
        }
    }

    tera_context.insert("interfaces", &interfaces);
    let render = TEMPLATES.render("cloud_init_network", &tera_context)?;

    tracing::info!("@@@@@@@@@@\n\n{render}");

    Ok(render.into_bytes())
}
