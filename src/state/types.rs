// Define the high level datastructures.
// This is *purely* for representing the state.

// TODO: serde to store the representation

use crate::errors::Error;
use anyhow::Context;
use std::{
    collections::{BTreeMap, BTreeSet},
    net::SocketAddr,
    path::PathBuf,
    str::FromStr,
};
use uuid::Uuid;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct State {
    pub subsystems: BTreeMap<String, Subsystem>,
    pub ports: BTreeMap<u32, Port>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Subsystem {
    pub model: Option<String>,
    pub serial: Option<String>,
    pub allowed_hosts: BTreeSet<String>,
    pub namespaces: BTreeMap<u32, Namespace>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Namespace {
    pub enabled: bool,
    pub device_path: PathBuf,
    pub device_uuid: Option<Uuid>,
    pub device_nguid: Option<Uuid>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Port {
    pub port_type: PortType,
    pub subsystems: BTreeSet<String>,
}

impl Port {
    pub fn new(port_type: PortType, subsystems: BTreeSet<String>) -> Self {
        Self {
            port_type,
            subsystems,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum PortType {
    Loop,
    Tcp(SocketAddr),
    Rdma(SocketAddr),
    FibreChannel(FibreChannelAddr),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct FibreChannelAddr {
    pub wwnn: u64,
    pub wwpn: u64,
}

impl FibreChannelAddr {
    pub fn new(wwnn: u64, wwpn: u64) -> Self {
        Self { wwnn, wwpn }
    }

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
                wwnn: u64::from_str_radix(&s[3..20], 16)
                    .with_context(|| Error::InvalidFCWWNN(s[3..20].to_string()))?,
                wwpn: u64::from_str_radix(&s[21..28], 16)
                    .with_context(|| Error::InvalidFCWWNN(s[21..28].to_string()))?,
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
    fn test_fcaddr() {
        let traddr = "nn-0x1000000044001123:pn-0x2000000055001123";
        let addr = FibreChannelAddr::new(0x1000000044001123, 0x2000000055001123);
        assert_eq!(traddr.parse::<FibreChannelAddr>().unwrap(), addr);
        assert_eq!(addr.to_traddr(), traddr);
    }
}
