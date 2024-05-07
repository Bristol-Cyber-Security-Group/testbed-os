use std::collections::BTreeMap;
use crate::assets::Assets;
use serde::Serialize;
use std::fmt;
use std::fmt::Formatter;

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
    tb_set_ip: String,
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
    tb_set_ip: String,
) -> String {
    format!(
        "{}",
        MetaData {
            instance_id: hostname.clone(),
            local_hostname: hostname,
            public_ssh_key,
            environment,
            tb_set_ip,
        }
    )
}

pub fn create_user_data() -> Vec<u8> {
    Assets::get("cloud_init.yaml").unwrap().data.into_owned()
}

pub fn create_network_config() -> Vec<u8> {
    Assets::get("network-config.yaml").unwrap().data.into_owned()
}
