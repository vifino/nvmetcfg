use crate::errors::{Error, Result};
use crate::helpers::{
    assert_valid_model, assert_valid_nqn, assert_valid_serial, get_hashmap_differences, read_str,
    write_str,
};
use crate::state::{Namespace, PortType};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use uuid::Uuid;

static NVMET_ROOT: &str = "/sys/kernel/config/nvmet/";

pub(super) struct NvmetRoot {}

impl NvmetRoot {
    pub(super) fn check_exists() -> Result<()> {
        let exists = Path::new(NVMET_ROOT).try_exists()?;
        if !exists {
            Err(Error::NoNvmetSysfs)
        } else {
            Ok(())
        }
    }

    pub(super) fn list_hosts() -> Result<HashSet<String>> {
        let path = Path::new(NVMET_ROOT).join("hosts");
        let paths = std::fs::read_dir(path)?;

        let mut hosts = HashSet::new();
        for wpath in paths {
            let path = wpath?;
            hosts.insert(path.file_name().to_str().unwrap().to_owned());
        }
        Ok(hosts)
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
    pub(super) fn has_port(id: u32) -> Result<bool> {
        let path = Path::new(NVMET_ROOT).join("ports").join(format!("{}", id));
        Ok(path.try_exists()?)
    }
    pub(super) fn create_port(id: u32) -> Result<NvmetPort> {
        let path = Path::new(NVMET_ROOT).join("ports").join(format!("{}", id));
        std::fs::create_dir(path.clone())?;
        Ok(NvmetPort { id, path })
    }
    pub(super) fn delete_port(id: u32) -> Result<()> {
        let path = Path::new(NVMET_ROOT).join("ports").join(format!("{}", id));
        if !path.try_exists()? {
            return Err(Error::NoSuchPort(id));
        }

        let port = NvmetPort {
            id,
            path: path.clone(),
        };

        for sub in port.list_subsystems()? {
            port.delete_subsystem(&sub)?;
        }

        std::fs::remove_dir(path)?;
        Ok(())
    }

    pub(super) fn list_subsystems() -> Result<Vec<NvmetSubsystem>> {
        let path = Path::new(NVMET_ROOT).join("subsystems");
        let paths = std::fs::read_dir(path)?;

        let mut ports = Vec::new();
        for wpath in paths {
            let path = wpath?;
            let nqn = path.file_name().to_str().unwrap().to_string();
            ports.push(NvmetSubsystem {
                nqn,
                path: path.path(),
            });
        }
        Ok(ports)
    }
    pub(super) fn has_subsystem(nqn: &str) -> Result<bool> {
        let path = Path::new(NVMET_ROOT).join("subsystems").join(&nqn);
        Ok(path.try_exists()?)
    }
    pub(super) fn create_subsystem(nqn: &str) -> Result<NvmetSubsystem> {
        assert_valid_nqn(nqn)?;
        let path = Path::new(NVMET_ROOT).join("subsystems").join(&nqn);
        std::fs::create_dir(path.clone())?;
        Ok(NvmetSubsystem {
            nqn: nqn.to_string(),
            path,
        })
    }
    pub(super) fn delete_subsystem(nqn: &str) -> Result<()> {
        assert_valid_nqn(nqn)?;
        let path = Path::new(NVMET_ROOT).join("subsystems").join(&nqn);
        if !path.try_exists()? {
            return Err(Error::NoSuchSubsystem(nqn.to_string()));
        }

        let sub = NvmetSubsystem {
            nqn: nqn.to_string(),
            path: path.clone(),
        };

        for host in sub.list_hosts()? {
            sub.disable_host(&host)?;
        }

        for (nsid, _ns) in sub.list_namespaces()? {
            sub.delete_namespace(nsid)?;
        }

        std::fs::remove_dir(path)?;
        Ok(())
    }
}

pub(super) struct NvmetPort {
    pub id: u32,
    path: PathBuf,
}

impl NvmetPort {
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
                write_str(self.path.join("addr_trtype"), "loop")?;
            }
            PortType::Tcp(saddr) => {
                write_str(self.path.join("addr_trtype"), "tcp")?;
                if saddr.is_ipv6() {
                    write_str(self.path.join("addr_adrfam"), "ipv6")?;
                } else {
                    write_str(self.path.join("addr_adrfam"), "ipv4")?;
                }
                write_str(self.path.join("addr_trsvcid"), saddr.port())?;
            }
            PortType::Rdma(saddr) => {
                write_str(self.path.join("addr_trtype"), "rdma")?;
                if saddr.is_ipv6() {
                    write_str(self.path.join("addr_adrfam"), "ipv6")?;
                } else {
                    write_str(self.path.join("addr_adrfam"), "ipv4")?;
                }
                write_str(self.path.join("addr_trsvcid"), saddr.port())?;
            }
            PortType::FibreChannel(fcaddr) => {
                write_str(self.path.join("addr_trtype"), "fc")?;
                write_str(self.path.join("addr_adrfam"), "fc")?;
                write_str(self.path.join("addr_traddr"), fcaddr.to_traddr())?;
            }
        }
        Ok(())
    }

    pub(super) fn list_subsystems(&self) -> Result<HashSet<String>> {
        let path = self.path.join("subsystems");
        let paths = std::fs::read_dir(path)?;

        let mut subsystems = HashSet::new();
        for wpath in paths {
            let path = wpath?;
            subsystems.insert(path.file_name().to_str().unwrap().to_owned());
        }
        Ok(subsystems)
    }

    pub(super) fn has_subsystem(&self, nqn: &str) -> Result<bool> {
        let path = Path::new(NVMET_ROOT).join("subsystems").join(nqn);
        Ok(path.try_exists()?)
    }
    pub(super) fn delete_subsystem(&self, nqn: &str) -> Result<()> {
        let path = self.path.join("subsystems").join(nqn);
        std::fs::remove_dir(path)?;
        Ok(())
    }
    pub(super) fn create_subsystem(&self, nqn: &str) -> Result<()> {
        assert_valid_nqn(nqn)?;
        let path = self.path.join("subsystems").join(nqn);
        let sub = Path::new(NVMET_ROOT).join("subsystems").join(nqn);
        if !sub.try_exists()? {
            return Err(Error::NoSuchSubsystem(nqn.to_string()));
        }
        std::os::unix::fs::symlink(path, sub)?;
        Ok(())
    }

    pub(super) fn set_subsystems(&self, desired: HashSet<String>) -> Result<()> {
        let actual = HashSet::from_iter(self.list_subsystems()?);
        let added = desired.difference(&actual);
        let removed = desired.difference(&actual);

        for sub in removed {
            self.delete_subsystem(sub)?;
        }

        for sub in added {
            self.create_subsystem(sub)?;
        }
        Ok(())
    }
}

pub(super) struct NvmetSubsystem {
    pub(super) nqn: String,
    path: PathBuf,
}

impl NvmetSubsystem {
    pub(super) fn set_allow_any(&self, enabled: bool) -> Result<()> {
        if enabled {
            write_str(self.path.join("attr_allow_any_host"), "1")?;
        } else {
            write_str(self.path.join("attr_allow_any_host"), "0")?;
        }
        Ok(())
    }

    pub(super) fn list_hosts(&self) -> Result<HashSet<String>> {
        let path = self.path.join("allowed_hosts");
        let paths = std::fs::read_dir(path)?;

        let mut hosts = HashSet::new();
        for wpath in paths {
            let path = wpath?;
            hosts.insert(path.file_name().to_str().unwrap().to_owned());
        }
        Ok(hosts)
    }
    pub(super) fn enable_host(&self, nqn: &str) -> Result<()> {
        assert_valid_nqn(nqn)?;
        let path = self.path.join("allowed_hosts").join(nqn);
        let host = Path::new(NVMET_ROOT).join("hosts").join(nqn);
        if !host.try_exists()? {
            std::fs::create_dir(host.clone())?;
        }
        std::os::unix::fs::symlink(path, host)?;
        Ok(())
    }
    pub(super) fn disable_host(&self, nqn: &str) -> Result<()> {
        let path = self.path.join("allowed_hosts").join(nqn);
        std::fs::remove_dir(path)?;
        Ok(())
    }
    pub(super) fn set_hosts(&self, hosts: HashSet<String>) -> Result<()> {
        let current_hosts = self.list_hosts()?;
        let added_hosts = hosts.difference(&current_hosts);
        let removed_hosts = current_hosts.difference(&hosts);

        for removed in removed_hosts {
            self.disable_host(&removed)?;
        }
        for added in added_hosts {
            self.enable_host(&added)?;
        }

        self.set_allow_any(hosts.len() == 0)?;
        Ok(())
    }

    pub(super) fn list_namespaces(&self) -> Result<HashMap<u32, NvmetNamespace>> {
        let path = self.path.join("namespaces");
        let paths = std::fs::read_dir(path)?;

        let mut nses = HashMap::new();
        for wpath in paths {
            let path = wpath?;
            let nsid = path.file_name().to_str().unwrap().parse()?;
            nses.insert(nsid, NvmetNamespace { path: path.path() });
        }
        Ok(nses)
    }
    pub(super) fn create_namespace(&self, nsid: u32) -> Result<NvmetNamespace> {
        let path = self.path.join("namespaces").join(format!("{}", nsid));
        std::fs::create_dir(path.clone())?;
        Ok(NvmetNamespace { path: path.clone() })
    }
    pub(super) fn delete_namespace(&self, nsid: u32) -> Result<()> {
        let path = self.path.join("namespaces").join(format!("{}", nsid));
        if !path.try_exists()? {
            return Err(Error::NoSuchNamespace(nsid, self.nqn.clone()));
        }
        let ns = NvmetNamespace { path: path.clone() };
        // Disable first
        ns.set_enabled(false)?;
        // Delete directory.
        std::fs::remove_dir(path)?;
        Ok(())
    }
    pub(super) fn set_namespaces(&self, nses: HashMap<u32, Namespace>) -> Result<()> {
        // TODO: slightly inefficient as it fetches data for to-be-removed namespaces too
        // Utterly irrelevant though.
        let mut current = HashMap::new();
        for (id, nvmetns) in self.list_namespaces()? {
            current.insert(id, nvmetns.get_namespace()?);
        }
        let delta = get_hashmap_differences(&current, &nses);

        for nsid in delta.removed {
            self.delete_namespace(nsid)?;
        }
        for nsid in delta.changed {
            let ns = self.create_namespace(nsid)?;
            ns.set_namespace(nses.get(&nsid).unwrap())?;
        }
        for nsid in delta.added {
            let ns = self.create_namespace(nsid)?;
            ns.set_namespace(nses.get(&nsid).unwrap())?;
        }
        Ok(())
    }

    pub(super) fn get_model(&self) -> Result<String> {
        Ok(read_str(self.path.join("attr_model"))?)
    }
    pub(super) fn set_model(&self, model: &str) -> Result<()> {
        assert_valid_model(model)?;
        write_str(self.path.join("attr_model"), model)?;
        Ok(())
    }
    pub(super) fn get_serial(&self) -> Result<String> {
        Ok(read_str(self.path.join("attr_serial"))?)
    }
    pub(super) fn set_serial(&self, serial: &str) -> Result<()> {
        assert_valid_serial(serial)?;
        write_str(self.path.join("attr_serial"), serial)?;
        Ok(())
    }
}

pub(super) struct NvmetNamespace {
    path: PathBuf,
}

impl NvmetNamespace {
    pub(super) fn is_enabled(&self) -> Result<bool> {
        Ok(match read_str(self.path.join("enable"))?.as_str() {
            "1" => true,
            "0" => false,
            _ => unreachable!(
                "nvmet subsystem namespace enabled state can never be anything but 1 or 0"
            ),
        })
    }
    pub(super) fn set_enabled(&self, enabled: bool) -> Result<()> {
        if enabled {
            write_str(self.path.join("enable"), "1")?;
        } else {
            write_str(self.path.join("enable"), "0")?;
        }
        Ok(())
    }

    pub(super) fn get_device_path(&self) -> Result<String> {
        read_str(self.path.join("device_path"))
    }
    pub(super) fn set_device_path(&self, dev: &str) -> Result<()> {
        let path = Path::new(dev);
        // TODO: check if it is a *device*, not a file
        // TODO: is it possible to mount a file instead? there is a mysterious "buffered_io" file..
        if !path.is_file() {
            return Err(Error::InvalidDevice(dev.to_string()));
        }
        write_str(
            self.path.join("device_path"),
            path.canonicalize()?.to_str().unwrap(),
        )
    }

    pub(super) fn get_device_uuid(&self) -> Result<Uuid> {
        Ok(Uuid::parse_str(
            read_str(self.path.join("device_uuid"))?.as_str(),
        )?)
    }
    pub(super) fn set_device_uuid(&self, uuid: &Uuid) -> Result<()> {
        write_str(self.path.join("device_uuid"), uuid.urn())?;
        Ok(())
    }

    pub(super) fn get_device_nguid(&self) -> Result<Uuid> {
        Ok(Uuid::parse_str(
            read_str(self.path.join("device_nguid"))?.as_str(),
        )?)
    }
    pub(super) fn set_device_nguid(&self, uuid: &Uuid) -> Result<()> {
        write_str(self.path.join("device_nguid"), uuid.urn())?;
        Ok(())
    }

    pub(super) fn get_namespace(&self) -> Result<Namespace> {
        Ok(Namespace {
            enabled: self.is_enabled()?,
            device_path: self.get_device_path()?,
            device_uuid: Some(self.get_device_uuid()?),
            device_nguid: Some(self.get_device_nguid()?),
        })
    }
    pub(super) fn set_namespace(&self, ns: &Namespace) -> Result<()> {
        // Always need to disable before applying changes.
        self.set_enabled(false)?;

        self.set_device_path(&ns.device_path)?;
        if let Some(uuid) = ns.device_uuid {
            self.set_device_uuid(&uuid)?;
        }
        if let Some(nguid) = ns.device_nguid {
            self.set_device_nguid(&nguid)?;
        }

        self.set_enabled(ns.enabled)?;

        Ok(())
    }
}
