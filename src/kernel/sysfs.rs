use super::helpers::{read_str, write_str};
use crate::errors::{Error, Result};
use crate::state::PortType;
use std::path::{Path, PathBuf};

static NVMET_ROOT: &str = "/sys/kernel/config/nvmet/";

pub(super) struct NvmetRoot {}

impl NvmetRoot {
    pub(super) fn list_hosts() -> Result<Vec<String>> {
        let path = Path::new(NVMET_ROOT).join("hosts");
        let paths = std::fs::read_dir(path)?;

        let mut hosts = Vec::new();
        for wpath in paths {
            let path = wpath?;
            hosts.push(path.file_name().to_str().unwrap().to_owned());
        }
        Ok(hosts)
    }

    pub(super) fn create_host(nqn: &str) -> Result<()> {
        let path = Path::new(NVMET_ROOT).join("hosts").join(nqn);
        std::fs::create_dir(path)?;
        Ok(())
    }
    pub(super) fn remove_host(nqn: &str) -> Result<()> {
        let path = Path::new(NVMET_ROOT).join("hosts").join(nqn);
        std::fs::remove_dir(path)?;
        Ok(())
    }

    pub(super) fn list_ports() -> Result<Vec<NvmetPort>> {
        let path = Path::new(NVMET_ROOT).join("ports");
        let paths = std::fs::read_dir(path)?;

        let mut ports = Vec::new();
        for wpath in paths {
            let path = wpath?;
            if let Ok(id) = path.file_name().to_str().unwrap().parse() {
                ports.push(NvmetPort {
                    id,
                    path: path.path(),
                });
            }
        }
        Ok(ports)
    }

    pub(super) fn create_port(id: u32) -> Result<NvmetPort> {
        let path = Path::new(NVMET_ROOT).join("ports").join(format!("{}", id));
        std::fs::create_dir(path.clone())?;
        Ok(NvmetPort { id, path })
    }
    pub(super) fn delete_port(id: u32) -> Result<()> {
        // TODO: remove enabled subsystems first
        let path = Path::new(NVMET_ROOT).join("ports").join(format!("{}", id));
        std::fs::remove_dir(path)?;
        Ok(())
    }
}

pub(super) struct NvmetPort {
    pub id: u32,
    path: PathBuf,
}

impl NvmetPort {
    pub(super) fn delete(self) -> Result<()> {
        std::fs::remove_dir(self.path)?;
        Ok(())
    }

    pub(super) fn get_type(&self) -> Result<PortType> {
        let trtype = read_str(self.path.join("addr_trtype"))?;
        let traddr = read_str(self.path.join("addr_traddr"))?;
        let trsvcid = read_str(self.path.join("addr_trsvcid"))?;
        match trtype.as_str() {
            "loop" => Ok(PortType::Loop),
            "tcp" => Ok(PortType::Tcp(format!("{}:{}", traddr, trsvcid).parse()?)),
            "rdma" => Ok(PortType::Rdma(format!("{}:{}", traddr, trsvcid).parse()?)),
            "fc" => Ok(PortType::FibreChannel(traddr.parse()?)),
            _ => Err(Error::UnsupportedTrType(trtype)),
        }
    }
    pub(super) fn set_type(&self, port_type: PortType) -> Result<()> {
        match port_type {
            PortType::Loop => {
                write_str(self.path.join("addr_trtype"), "loop".to_string())?;
            }
            PortType::Tcp(saddr) => {
                write_str(self.path.join("addr_trtype"), "tcp".to_string())?;
                if saddr.is_ipv6() {
                    write_str(self.path.join("addr_adrfam"), "ipv6".to_string())?;
                } else {
                    write_str(self.path.join("addr_adrfam"), "ipv4".to_string())?;
                }
                write_str(self.path.join("addr_trsvcid"), format!("{}", saddr.port()))?;
            }
            PortType::Rdma(saddr) => {
                write_str(self.path.join("addr_trtype"), "rdma".to_string())?;
                if saddr.is_ipv6() {
                    write_str(self.path.join("addr_adrfam"), "ipv6".to_string())?;
                } else {
                    write_str(self.path.join("addr_adrfam"), "ipv4".to_string())?;
                }
                write_str(self.path.join("addr_trsvcid"), format!("{}", saddr.port()))?;
            }
            PortType::FibreChannel(fcaddr) => {
                write_str(self.path.join("addr_trtype"), "fc".to_string())?;
                write_str(self.path.join("addr_adrfam"), "fc".to_string())?;
                write_str(self.path.join("addr_traddr"), fcaddr.to_traddr())?;
            }
        }
        Ok(())
    }

    pub(super) fn list_subsystems(&self) -> Result<Vec<String>> {
        let path = self.path.join("subsystems");
        let paths = std::fs::read_dir(path)?;

        let mut subsystems = Vec::new();
        for wpath in paths {
            let path = wpath?;
            subsystems.push(path.file_name().to_str().unwrap().to_owned());
        }
        Ok(subsystems)
    }
}
