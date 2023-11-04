// Define the high level datastructures.
// This is *purely* for representing the state.

// TODO: serde to store the representation

use std::{
    collections::{HashMap, HashSet},
    net::SocketAddr,
    str::FromStr,
};
use uuid::Uuid;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct State {
    pub subsystems: HashMap<String, Subsystem>,
    pub ports: HashMap<u32, Port>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Subsystem {
    pub model: Option<String>,
    pub serial: Option<String>,
    pub allowed_hosts: HashSet<String>,
    pub namespaces: HashMap<u32, Namespace>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Namespace {
    pub enabled: bool,
    pub device_path: String,
    pub device_uuid: Option<Uuid>,
    pub device_nguid: Option<Uuid>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Port {
    pub port_type: PortType,
    pub subsystems: HashSet<String>,
}

impl Port {
    pub fn new(port_type: PortType, subsystems: HashSet<String>) -> Self {
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
    type Err = crate::errors::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // The traddr looks like this:
        // nn-0x1000000044001123:pn-0x2000000055001123
        // OR
        // nn-1000000044001123:pn-2000000055001123
        // TODO: ERROR HANDLING. YIKES.

        if s.len() == 7 + 4 + 32 {
            Ok(Self {
                wwnn: u64::from_str_radix(&s[5..21], 16)?,
                wwpn: u64::from_str_radix(&s[27..43], 16)?,
            })
        } else if s.len() == 7 + 32 {
            Ok(Self {
                wwnn: u64::from_str_radix(&s[3..20], 16)?,
                wwpn: u64::from_str_radix(&s[21..28], 16)?,
            })
        } else {
            Err(Self::Err::InvalidFCAddr(s.to_string()))
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
