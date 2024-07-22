use std::net::IpAddr;
use anyhow::bail;
use serde::{Deserialize, Serialize};

pub mod chassis;
pub mod ovs;
pub mod logical_switch;
pub mod logical_router;
pub mod logical_router_port;
pub mod logical_switch_port;
pub mod acl;

/// Helper macro to convert Vec<&str> to Vec<String> to avoid having to keep writing `.to_string()`
#[macro_export]
macro_rules! vec_of_strings {
    ($($x:expr),*) => (vec![$($x.to_string()),*]);
}

/// Struct to represent mac addresses, to validate the string to make sure it is a valid mac address
#[derive(Debug, Clone, Deserialize, Serialize, Eq, PartialEq, Hash)]
pub struct MacAddress {
    pub address: String,
    pub as_bytes: Option<u64>,
}

impl MacAddress {
    pub fn new(
        address: String,
    ) -> anyhow::Result<Self> {
        let as_bytes = if address.eq("router") {
            None
        } else if address.eq("00:00:00:00:00:00") {
            bail!("cant have a mac address of 00:00:00:00:00:00")
        } else {
            validate_mac(&address)?;
            Some(Self::get_mac_as_bytes(&address)?)
        };
        Ok(Self { address, as_bytes })
    }

    fn get_mac_as_bytes(address: &String) -> anyhow::Result<u64> {
        let split: Vec<_> = address.split(":").collect();
        // make sure the right number of colons
        if split.len() != 6 {
            // bail!("mac address not 48 or 64 bit format {address}");
            bail!("mac address not 48 bit format {address}");
        }
        // get octets
        let octets = MacAddress::get_octets(split);
        let mut mac_bytes = 0;
        for byte in octets {
            mac_bytes = (mac_bytes << 8) | u64::from(byte);
        }

        Ok(mac_bytes)
    }

    /// We represent the mac address as a 64 bit integer, while usually 48 bits there can be 64 bit
    /// versions. Since we dont have a 48bit integer, we will pad with zeros
    fn get_octets(address: Vec<&str>) -> [u8; 6] {
        // force into 64 bit length, assume we already checked if it is correct len from validate
        // let mut addr = if address.len() == 6 {
        //     vec!["00","00",address[0],address[1],address[2],address[3],address[4],address[5]]
        // } else {
        //     address
        // };
        // convert to u8 vec
        let addr: Vec<u8> = address.iter()
            .map(|oct| u8::from_str_radix(oct, 16).expect("converting mac octet to hex"))
            .collect();
        // create fixed length array
        let mut byte_array = [0u8; 6];
        // copy u8 values from vec into the array
        byte_array.copy_from_slice(&addr);
        byte_array
    }

    pub fn from_u64(num: u64) -> anyhow::Result<Self> {
        if num == 0 {
            bail!("mac from u64 is 0, cant have a mac address of 00:00:00:00:00:00")
        }
        // u64 to [u8; 6]
        let mut result = [0u8; 6];
        for i in 0..6 {
            result[5 - i] = ((num >> (i * 8)) & 0xFF) as u8;
        }
        // [u8; 6] to string
        let u8_vec = result.to_vec();
        // TODO - byte to hex to string

        let mac = format!(
            "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
            u8_vec[0], u8_vec[1], u8_vec[2], u8_vec[3], u8_vec[4], u8_vec[5]
        );

        Self::new(mac)
    }

    pub fn get_string(&self) -> String {
        self.address.clone()
    }
}

fn validate_mac(mac: &String) -> anyhow::Result<()> {
    // make sure the format of the octets is OK
    let split: Vec<_> = mac.split(":").collect();
    for octet in split {
        if octet.len() != 2 {
            bail!("octet {octet} in mac {mac} is not correct");
        }
    }
    Ok(())
}

/// Enum to represent the ip address a port can have. Either an ip address that can be v4 or v6 or
/// specify dynamic if using DHCP. Used throughout this crate to make comparisons and validation
/// easier.
#[derive(Debug, Clone, Deserialize, Serialize, Eq, PartialEq, Hash)]
pub enum OvnIpAddr {
    Ip(IpAddr),
    Dynamic,
    Subnet {
        ip: IpAddr,
        mask: u16,
    },
}

impl OvnIpAddr {
    pub fn to_string(
        &self,
    ) -> String {
        match &self {
            OvnIpAddr::Ip(ip) => {
                match &ip {
                    IpAddr::V4(v4) => v4.to_string(),
                    IpAddr::V6(v6) => v6.to_string(),
                }
            }
            OvnIpAddr::Dynamic => "dynamic".to_string(),
            OvnIpAddr::Subnet { ip, mask } => format!("{ip}/{mask}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_mac() {
        let mac = MacAddress::new("00:00:00:00:00:03".into());
        assert!(mac.is_ok());
        assert_eq!(mac.as_ref().unwrap().address, "00:00:00:00:00:03".to_string());
        assert!(mac.as_ref().unwrap().as_bytes.is_some());
        assert_eq!(mac.unwrap().as_bytes.unwrap(), 3u64);

        let mac = MacAddress::new("router".into());
        assert!(mac.is_ok());
        assert_eq!(mac.as_ref().unwrap().address, "router".to_string());
        assert!(mac.as_ref().unwrap().as_bytes.is_none());

        let mac = MacAddress::new("asdf".into());
        assert!(mac.is_err());
    }

    #[test]
    fn test_get_octets_48bit_mac() {
        let mac = vec!["00", "00", "00", "00", "00", "00"];
        let mac_bytes = MacAddress::get_octets(mac);
        let expected_mac = [0, 0, 0, 0, 0, 0];
        assert_eq!(mac_bytes, expected_mac);

        let mac = vec!["ff", "ff", "ff", "ff", "ff", "ff"];
        let mac_bytes = MacAddress::get_octets(mac);
        let expected_mac = [255, 255, 255, 255, 255 ,255];
        assert_eq!(mac_bytes, expected_mac);
    }

    // #[test]
    // fn test_get_octets_64bit_mac() {
    //     let mac = vec!["00", "00", "00", "00", "00", "00", "00", "00"];
    //     let mac_bytes = MacAddress::get_octets(mac);
    //     let expected_mac = [0, 0, 0, 0, 0, 0, 0 ,0];
    //     assert_eq!(mac_bytes, expected_mac);
    //
    //     let mac = vec!["ff", "ff", "ff", "ff", "ff", "ff", "ff", "ff"];
    //     let mac_bytes = MacAddress::get_octets(mac);
    //     let expected_mac = [255, 255, 255, 255, 255, 255, 255 ,255];
    //     assert_eq!(mac_bytes, expected_mac);
    // }

    #[test]
    fn test_mac_as_bytes_48bit() {
        let mac_bytes = MacAddress::get_mac_as_bytes(&"00:00:00:00:00:00".into());
        assert!(mac_bytes.is_ok());
        assert_eq!(mac_bytes.unwrap(), 0u64);

        let mac_bytes = MacAddress::get_mac_as_bytes(&"00:00:00:00:00:ff".into());
        assert!(mac_bytes.is_ok());
        assert_eq!(mac_bytes.unwrap(), 255u64);

        let mac_bytes = MacAddress::get_mac_as_bytes(&"ff:00:00:00:00:00".into());
        assert!(mac_bytes.is_ok());
        assert_eq!(mac_bytes.unwrap(), 280375465082880u64);
    }

    // #[test]
    // fn test_mac_as_bytes_64bit() {
    //     let mac_bytes = MacAddress::get_mac_as_bytes(&"00:00:00:00:00:00:00:00".into());
    //     assert!(mac_bytes.is_ok());
    //     assert_eq!(mac_bytes.unwrap(), 0u64);
    //
    //     let mac_bytes = MacAddress::get_mac_as_bytes(&"00:00:ff:00:00:00:00:00".into());
    //     assert!(mac_bytes.is_ok());
    //     assert_eq!(mac_bytes.unwrap(), 280375465082880u64);
    //
    //     let mac_bytes = MacAddress::get_mac_as_bytes(&"ff:00:00:00:00:00:00:00".into());
    //     assert!(mac_bytes.is_ok());
    //     assert_eq!(mac_bytes.unwrap(), 18374686479671623680u64);
    // }

    #[test]
    fn test_bad_mac_as_bytes() {
        let mac_bytes = MacAddress::get_mac_as_bytes(&"00:00:00:00:00".into());
        assert!(mac_bytes.is_err());
        let mac_bytes = MacAddress::get_mac_as_bytes(&"00:00:00:00:00:00:00:00:00".into());
        assert!(mac_bytes.is_err());
        let mac_bytes = MacAddress::get_mac_as_bytes(&"00;00;00;00;00;00;00".into());
        assert!(mac_bytes.is_err());
    }

    #[test]
    fn test_from_u64() {
        let mac = MacAddress::from_u64(1u64);
        assert!(mac.is_ok());
        assert_eq!(mac.as_ref().unwrap().address, "00:00:00:00:00:01".to_string());
        assert!(mac.as_ref().unwrap().as_bytes.is_some());
        assert_eq!(mac.unwrap().as_bytes.unwrap(), 1u64);

        let mac = MacAddress::from_u64(280375465082880u64);
        assert!(mac.is_ok());
        assert_eq!(mac.as_ref().unwrap().address, "ff:00:00:00:00:00".to_string());
        assert!(mac.as_ref().unwrap().as_bytes.is_some());
        assert_eq!(mac.unwrap().as_bytes.unwrap(), 280375465082880u64);
    }

    #[test]
    fn test_mac_cant_be_0() {
        let mac = MacAddress::new("00:00:00:00:00:00".into());
        assert!(mac.is_err());
        let mac = MacAddress::from_u64(0u64);
        assert!(mac.is_err());
    }

}
