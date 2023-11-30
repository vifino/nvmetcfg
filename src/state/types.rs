// Define the high level datastructures.
// This is *purely* for representing the state.

// TODO: serde to store the representation

use crate::errors::Error;
use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    net::SocketAddr,
    path::PathBuf,
    str::FromStr,
};
use uuid::Uuid;

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct State {
    pub subsystems: BTreeMap<String, Subsystem>,
    pub ports: BTreeMap<u16, Port>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Subsystem {
    pub model: Option<String>,
    pub serial: Option<String>,
    pub allowed_hosts: BTreeSet<String>,
    pub namespaces: BTreeMap<u32, Namespace>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Namespace {
    pub enabled: bool,
    pub device_path: PathBuf,
    pub device_uuid: Option<Uuid>,
    pub device_nguid: Option<Uuid>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Port {
    #[serde(flatten)]
    pub port_type: PortType,
    pub subsystems: BTreeSet<String>,
}

impl Port {
    #[must_use]
    pub const fn new(port_type: PortType, subsystems: BTreeSet<String>) -> Self {
        Self {
            port_type,
            subsystems,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "port_type", content = "port_addr")]
pub enum PortType {
    Loop,
    Tcp(SocketAddr),
    Rdma(SocketAddr),
    FibreChannel(FibreChannelAddr),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FibreChannelAddr {
    pub wwnn: u64,
    pub wwpn: u64,
}

impl FibreChannelAddr {
    #[must_use]
    pub const fn new(wwnn: u64, wwpn: u64) -> Self {
        Self { wwnn, wwpn }
    }

    #[must_use]
    pub fn to_traddr(&self) -> String {
        format!("nn-{:#018x}:pn-{:#018x}", self.wwnn, self.wwpn)
    }
}

impl FromStr for FibreChannelAddr {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // The traddr looks like this:
        // nn-0x1000000044001123:pn-0x2000000055001123
        // OR
        // nn-1000000044001123:pn-2000000055001123

        if s.len() == 7 + 4 + 32 {
            Ok(Self {
                wwnn: u64::from_str_radix(&s[5..21], 16)
                    .with_context(|| Error::InvalidFCWWNN(s[5..21].to_string()))?,
                wwpn: u64::from_str_radix(&s[27..43], 16)
                    .with_context(|| Error::InvalidFCWWPN(s[27..43].to_string()))?,
            })
        } else if s.len() == 7 + 32 {
            Ok(Self {
                wwnn: u64::from_str_radix(&s[3..19], 16)
                    .with_context(|| Error::InvalidFCWWNN(s[3..19].to_string()))?,
                wwpn: u64::from_str_radix(&s[23..39], 16)
                    .with_context(|| Error::InvalidFCWWPN(s[23..39].to_string()))?,
            })
        } else {
            Err(Error::InvalidFCAddr(s.to_string()).into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fcaddr_valid() {
        let addr = FibreChannelAddr::new(0x1000_0000_4400_1123, 0x2000_0000_5500_1123);
        let traddr_long = "nn-0x1000000044001123:pn-0x2000000055001123";
        let traddr_short = "nn-1000000044001123:pn-2000000055001123";
        assert_eq!(traddr_long.parse::<FibreChannelAddr>().unwrap(), addr);
        assert_eq!(traddr_short.parse::<FibreChannelAddr>().unwrap(), addr);

        // The kernel returns it long, so we do as well.
        assert_eq!(addr.to_traddr(), traddr_long);
    }

    #[test]
    fn test_fcaddr_invalid() {
        let traddr_too_short = "nn-10000000440011:pn-20000000550011";
        assert!(traddr_too_short.parse::<FibreChannelAddr>().is_err());
        let traddr_invalid_hex = "nn-10MEH00044001123:pn-2000000055001123";
        assert!(traddr_invalid_hex.parse::<FibreChannelAddr>().is_err());
    }
}
