use crate::errors::{Error, Result};
use crate::helpers::{
    assert_valid_model, assert_valid_nqn, assert_valid_nsid, assert_valid_serial,
    get_btreemap_differences, read_str, write_str,
};
use crate::state::{Namespace, PortType};
use anyhow::Context;
use std::collections::{BTreeMap, BTreeSet};
use std::os::unix::fs::FileTypeExt;
use std::path::{Path, PathBuf};
use uuid::Uuid;

static NVMET_ROOT: &str = "/sys/kernel/config/nvmet/";

pub(super) struct NvmetRoot {}

impl NvmetRoot {
    pub(super) fn check_exists() -> Result<()> {
        let exists = Path::new(NVMET_ROOT).try_exists()?;
        if exists {
            Ok(())
        } else {
            Err(Error::NoNvmetSysfs.into())
        }
    }

    pub(super) fn list_used_hosts() -> Result<BTreeSet<String>> {
        let mut hosts = BTreeSet::new();
        let subsystems = Self::list_subsystems()
            .with_context(|| "Failed listing subsystems to list used hosts".to_string())?;
        for sub in subsystems {
            hosts.append(&mut sub.list_hosts().with_context(|| {
                format!(
                    "Failed listing allowed hosts for subsystem {} to list used hosts",
                    sub.nqn
                )
            })?);
        }
        Ok(hosts)
    }

    pub(super) fn remove_host(nqn: &str) -> Result<()> {
        let path = Path::new(NVMET_ROOT).join("hosts").join(nqn);
        std::fs::remove_dir(path)
            .with_context(|| format!("Failed to remove directory of host {nqn}"))?;
        Ok(())
    }

    pub(super) fn list_ports() -> Result<Vec<NvmetPort>> {
        let path = Path::new(NVMET_ROOT).join("ports");
        let paths = std::fs::read_dir(path).context("Failed to list ports")?;

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
    pub(super) fn has_port(id: u16) -> Result<bool> {
        let path = Path::new(NVMET_ROOT).join("ports").join(format!("{id}"));
        Ok(path.try_exists()?)
    }
    pub(super) fn open_port(id: u16) -> NvmetPort {
        let path = Path::new(NVMET_ROOT).join("ports").join(format!("{id}"));
        NvmetPort { id, path }
    }
    pub(super) fn create_port(id: u16) -> Result<NvmetPort> {
        let port = Self::open_port(id);
        std::fs::create_dir(port.path.clone())
            .with_context(|| format!("Failed to create directory of port {id}"))?;
        Ok(port)
    }
    pub(super) fn delete_port(id: u16) -> Result<()> {
        let path = Path::new(NVMET_ROOT).join("ports").join(format!("{id}"));
        if !path.try_exists()? {
            return Err(Error::NoSuchPort(id).into());
        }

        let port = NvmetPort {
            id,
            path: path.clone(),
        };

        for sub in port.list_subsystems()? {
            port.disable_subsystem(&sub).with_context(|| {
                format!("Failed to disable subsystems of port {id} for deletion")
            })?;
        }

        std::fs::remove_dir(path)
            .with_context(|| format!("Failed to remove directory of port {id}"))?;
        Ok(())
    }

    pub(super) fn list_subsystems() -> Result<Vec<NvmetSubsystem>> {
        let path = Path::new(NVMET_ROOT).join("subsystems");
        let paths = std::fs::read_dir(path).context("Failed to list subsystems")?;

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
        let path = Path::new(NVMET_ROOT).join("subsystems").join(nqn);
        Ok(path.try_exists()?)
    }
    pub(super) fn open_subsystem(nqn: &str) -> Result<NvmetSubsystem> {
        assert_valid_nqn(nqn)?;
        let path = Path::new(NVMET_ROOT).join("subsystems").join(nqn);
        Ok(NvmetSubsystem {
            nqn: nqn.to_string(),
            path,
        })
    }
    pub(super) fn create_subsystem(nqn: &str) -> Result<NvmetSubsystem> {
        let sub = Self::open_subsystem(nqn)?;
        std::fs::create_dir(sub.path.clone())
            .with_context(|| format!("Failed to create directory of subsystem {nqn}"))?;
        Ok(sub)
    }
    pub(super) fn delete_subsystem(nqn: &str) -> Result<()> {
        assert_valid_nqn(nqn)?;
        let path = Path::new(NVMET_ROOT).join("subsystems").join(nqn);
        if !path.try_exists()? {
            return Err(Error::NoSuchSubsystem(nqn.to_string()).into());
        }

        let sub = NvmetSubsystem {
            nqn: nqn.to_string(),
            path: path.clone(),
        };

        for host in sub.list_hosts()? {
            sub.disable_host(&host).with_context(|| {
                format!("Failed to disable hosts for subsystem {nqn} before deletion")
            })?;
        }

        for (nsid, _ns) in sub.list_namespaces()? {
            sub.delete_namespace(nsid).with_context(|| {
                format!("Failed to delete namespaces of subsystem {nqn} before deletion")
            })?;
        }

        std::fs::remove_dir(path)
            .with_context(|| format!("Failed to remove directory of subsystem {nqn}"))?;
        Ok(())
    }
}

pub(super) struct NvmetPort {
    pub id: u16,
    path: PathBuf,
}

impl NvmetPort {
    pub(super) fn get_type(&self) -> Result<PortType> {
        let trtype = read_str(self.path.join("addr_trtype"))?;
        let traddr = read_str(self.path.join("addr_traddr"))?;
        let trsvcid = read_str(self.path.join("addr_trsvcid"))?;
        match trtype.as_str() {
            "loop" => Ok(PortType::Loop),
            "tcp" => Ok(PortType::Tcp(format!("{traddr}:{trsvcid}").parse()?)),
            "rdma" => Ok(PortType::Rdma(format!("{traddr}:{trsvcid}").parse()?)),
            "fc" => Ok(PortType::FibreChannel(traddr.parse()?)),
            _ => Err(Error::UnsupportedTrType(trtype).into()),
        }
    }
    pub(super) fn set_type(&self, port_type: PortType) -> Result<()> {
        // Remove all subsystems in order to unlock.
        let subs = self.list_subsystems()?;
        self.set_subsystems(&BTreeSet::new())?;

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
                write_str(self.path.join("addr_traddr"), saddr.ip())?;
                write_str(self.path.join("addr_trsvcid"), saddr.port())?;
            }
            PortType::Rdma(saddr) => {
                write_str(self.path.join("addr_trtype"), "rdma")?;
                if saddr.is_ipv6() {
                    write_str(self.path.join("addr_adrfam"), "ipv6")?;
                } else {
                    write_str(self.path.join("addr_adrfam"), "ipv4")?;
                }
                write_str(self.path.join("addr_traddr"), saddr.ip())?;
                write_str(self.path.join("addr_trsvcid"), saddr.port())?;
            }
            PortType::FibreChannel(fcaddr) => {
                write_str(self.path.join("addr_trtype"), "fc")?;
                write_str(self.path.join("addr_adrfam"), "fc")?;
                write_str(self.path.join("addr_traddr"), fcaddr.to_traddr())?;
                write_str(self.path.join("addr_trsvcid"), "none")?;
            }
        }
        // Re-add all the previously enabled subsystems.
        self.set_subsystems(&subs)?;
        Ok(())
    }

    pub(super) fn list_subsystems(&self) -> Result<BTreeSet<String>> {
        let path = self.path.join("subsystems");
        let paths = std::fs::read_dir(path)
            .with_context(|| format!("Failed to list enabled subsystems for pot {}", self.id))?;

        let mut subsystems = BTreeSet::new();
        for wpath in paths {
            let path = wpath?;
            subsystems.insert(path.file_name().to_str().unwrap().to_owned());
        }
        Ok(subsystems)
    }

    pub(super) fn has_subsystem(&self, nqn: &str) -> Result<bool> {
        let path = self.path.join("subsystems").join(nqn);
        Ok(path.try_exists()?)
    }
    pub(super) fn disable_subsystem(&self, nqn: &str) -> Result<()> {
        let path = self.path.join("subsystems").join(nqn);
        std::fs::remove_file(path)
            .with_context(|| format!("Failed to disable subsystem {} for port {}", nqn, self.id))?;
        Ok(())
    }
    pub(super) fn enable_subsystem(&self, nqn: &str) -> Result<()> {
        assert_valid_nqn(nqn)?;
        let path = self.path.join("subsystems").join(nqn);
        let sub = Path::new(NVMET_ROOT).join("subsystems").join(nqn);
        if !sub.try_exists()? {
            return Err(Error::NoSuchSubsystem(nqn.to_string()).into());
        }
        std::os::unix::fs::symlink(sub, path)
            .with_context(|| format!("Failed to enable subsystem {} for port {}", nqn, self.id))?;
        Ok(())
    }

    pub(super) fn set_subsystems(&self, desired: &BTreeSet<String>) -> Result<()> {
        let actual = BTreeSet::from_iter(self.list_subsystems()?);
        let added = desired.difference(&actual);
        let removed = actual.difference(desired);

        for sub in removed {
            self.disable_subsystem(sub).with_context(|| {
                format!("Failed to disable removed subsystem for port {}", self.id)
            })?;
        }

        for sub in added {
            self.enable_subsystem(sub).with_context(|| {
                format!("Failed to enable added subsystem for port {}", self.id)
            })?;
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
            write_str(self.path.join("attr_allow_any_host"), "1")
        } else {
            write_str(self.path.join("attr_allow_any_host"), "0")
        }
        .with_context(|| {
            format!(
                "Failed to set attr_allow_any_host for subsystem {}",
                self.nqn
            )
        })
    }

    pub(super) fn list_hosts(&self) -> Result<BTreeSet<String>> {
        let path = self.path.join("allowed_hosts");
        let paths = std::fs::read_dir(path)
            .with_context(|| format!("Failed to list allowed_hosts for subsystem {}", self.nqn))?;

        let mut hosts = BTreeSet::new();
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
            std::fs::create_dir(host.clone())
                .with_context(|| format!("Failed to create new host {nqn}"))?;
        }
        std::os::unix::fs::symlink(host, path)
            .with_context(|| format!("Failed to enable host {} in subsystem {}", nqn, self.nqn))?;
        Ok(())
    }
    pub(super) fn disable_host(&self, nqn: &str) -> Result<()> {
        let path = self.path.join("allowed_hosts").join(nqn);
        std::fs::remove_file(path)
            .with_context(|| format!("Failed to disable host {} in subsystem {}", nqn, self.nqn))?;
        Ok(())
    }
    pub(super) fn set_hosts(&self, hosts: &BTreeSet<String>) -> Result<()> {
        let current_hosts = self.list_hosts()?;
        let added_hosts = hosts.difference(&current_hosts);
        let removed_hosts = current_hosts.difference(hosts);

        for removed in removed_hosts {
            self.disable_host(removed).with_context(|| {
                format!("Failed to disable removed host in subsystem {}", self.nqn)
            })?;
        }
        self.set_allow_any(hosts.is_empty())?;
        for added in added_hosts {
            self.enable_host(added).with_context(|| {
                format!("Failed to enable added host in subsystem {}", self.nqn)
            })?;
        }
        Ok(())
    }

    pub(super) fn list_namespaces(&self) -> Result<BTreeMap<u32, NvmetNamespace>> {
        let path = self.path.join("namespaces");
        let paths = std::fs::read_dir(path)
            .with_context(|| format!("Failed to list namespaces of subsystem {}", self.nqn))?;

        let mut nses = BTreeMap::new();
        for wpath in paths {
            let path = wpath?;
            let nsid = path.file_name().to_str().unwrap().parse()?;
            nses.insert(
                nsid,
                NvmetNamespace {
                    path: path.path(),
                    nsid,
                },
            );
        }
        Ok(nses)
    }
    pub(super) fn open_namespace(&self, nsid: u32) -> Result<NvmetNamespace> {
        assert_valid_nsid(nsid)?;
        let path = self.path.join("namespaces").join(format!("{nsid}"));
        Ok(NvmetNamespace { nsid, path })
    }
    pub(super) fn create_namespace(&self, nsid: u32) -> Result<NvmetNamespace> {
        let ns = self.open_namespace(nsid)?;
        if ns.path.try_exists()? {
            return Err(Error::ExistingNamespace(nsid, self.nqn.clone()).into());
        }
        std::fs::create_dir(ns.path.clone()).with_context(|| {
            format!(
                "Failed to create directory of namespace {} in subsystem {}",
                nsid, self.nqn
            )
        })?;
        Ok(ns)
    }
    pub(super) fn delete_namespace(&self, nsid: u32) -> Result<()> {
        let path = self.path.join("namespaces").join(format!("{nsid}"));
        if !path.try_exists()? {
            return Err(Error::NoSuchNamespace(nsid, self.nqn.clone()).into());
        }
        let ns = NvmetNamespace {
            path: path.clone(),
            nsid,
        };
        // Disable first
        ns.set_enabled(false).with_context(|| {
            format!(
                "Failed to deactivate namespace {} before deletion in subsystem {}",
                nsid, self.nqn
            )
        })?;
        // Delete directory.
        std::fs::remove_dir(path).with_context(|| {
            format!(
                "Failed to remove directory of namespace {} in subsystem {}",
                nsid, self.nqn
            )
        })?;
        Ok(())
    }
    pub(super) fn set_namespaces(&self, nses: &BTreeMap<u32, Namespace>) -> Result<()> {
        // TODO: slightly inefficient as it fetches data for to-be-removed namespaces too
        // Utterly irrelevant though.
        let mut current = BTreeMap::new();
        for (id, nvmetns) in self.list_namespaces()? {
            current.insert(id, nvmetns.get_namespace()?);
        }
        let delta = get_btreemap_differences(&current, nses);

        for nsid in delta.removed {
            self.delete_namespace(nsid).with_context(|| {
                format!(
                    "Failed to set removed namespaces for subsystem {}",
                    self.nqn
                )
            })?;
        }
        for nsid in delta.changed {
            let ns = self.open_namespace(nsid)?;
            ns.set_namespace(nses.get(&nsid).unwrap())
                .with_context(|| {
                    format!(
                        "Failed to update existing namespaces for subsystem {}",
                        self.nqn
                    )
                })?;
        }
        for nsid in delta.added {
            let ns = self.create_namespace(nsid).with_context(|| {
                format!(
                    "Failed to create added namespaces for subsystem {}",
                    self.nqn
                )
            })?;
            ns.set_namespace(nses.get(&nsid).unwrap())
                .with_context(|| {
                    format!("Failed to set added namespaces for subsystem {}", self.nqn)
                })?;
        }
        Ok(())
    }

    pub(super) fn get_model(&self) -> Result<String> {
        read_str(self.path.join("attr_model"))
            .with_context(|| format!("Failed to get attr_model for subsystem {}", self.nqn))
    }
    pub(super) fn set_model(&self, model: &str) -> Result<()> {
        assert_valid_model(model)?;
        write_str(self.path.join("attr_model"), model)
            .with_context(|| format!("Failed to set attr_model for subsystem {}", self.nqn))?;
        Ok(())
    }
    pub(super) fn get_serial(&self) -> Result<String> {
        read_str(self.path.join("attr_serial"))
            .with_context(|| format!("Failed to read attr_serial for subsystem {}", self.nqn))
    }
    pub(super) fn set_serial(&self, serial: &str) -> Result<()> {
        assert_valid_serial(serial)?;
        write_str(self.path.join("attr_serial"), serial)
            .with_context(|| format!("Failed to set attr_serial for subsystem {}", self.nqn))?;
        Ok(())
    }
}

pub(super) struct NvmetNamespace {
    nsid: u32,
    path: PathBuf,
}

impl NvmetNamespace {
    pub(super) fn is_enabled(&self) -> Result<bool> {
        Ok(
            match read_str(self.path.join("enable"))
                .with_context(|| {
                    format!("Failed to get enabled state for namespace {}", self.nsid)
                })?
                .as_str()
            {
                "1" => true,
                "0" => false,
                _ => unreachable!(
                    "nvmet subsystem namespace enabled state can never be anything but 1 or 0"
                ),
            },
        )
    }
    pub(super) fn set_enabled(&self, enabled: bool) -> Result<()> {
        if enabled {
            write_str(self.path.join("enable"), "1")
        } else {
            write_str(self.path.join("enable"), "0")
        }
        .with_context(|| format!("Failed to set enabled state for namespace {}", self.nsid))
    }

    pub(super) fn get_device_path(&self) -> Result<PathBuf> {
        Ok(read_str(self.path.join("device_path"))?.into())
    }
    pub(super) fn set_device_path(&self, dev: &PathBuf) -> Result<()> {
        let path = Path::new(dev);
        // TODO: is it possible to mount a file instead? there is a mysterious "buffered_io" file..
        let metadata = std::fs::metadata(path)
            .with_context(|| {
                format!(
                    "Failed to get metadata for device {} in namespace {}",
                    dev.display(),
                    self.nsid
                )
            })?
            .file_type();
        if !metadata.is_block_device() {
            return Err(Error::InvalidDevice(dev.display().to_string()).into());
        }
        write_str(
            self.path.join("device_path"),
            path.canonicalize()?.to_str().unwrap(),
        )
        .with_context(|| format!("Failed to set device_path for namespace {}", self.nsid))
    }

    pub(super) fn get_device_uuid(&self) -> Result<Uuid> {
        Ok(Uuid::parse_str(
            read_str(self.path.join("device_uuid"))
                .with_context(|| format!("Failed to read device_uuid for namespace {}", self.nsid))?
                .as_str(),
        )?)
    }
    pub(super) fn set_device_uuid(&self, uuid: &Uuid) -> Result<()> {
        write_str(self.path.join("device_uuid"), uuid.hyphenated()).with_context(|| {
            format!(
                "Failed to set device_uuid {} for namespace {}",
                uuid, self.nsid
            )
        })?;
        Ok(())
    }

    pub(super) fn get_device_nguid(&self) -> Result<Uuid> {
        Ok(Uuid::parse_str(
            read_str(self.path.join("device_nguid"))
                .with_context(|| {
                    format!("Failed to read device_nguid for namespace {}", self.nsid)
                })?
                .as_str(),
        )?)
    }
    pub(super) fn set_device_nguid(&self, uuid: &Uuid) -> Result<()> {
        write_str(self.path.join("device_nguid"), uuid.hyphenated()).with_context(|| {
            format!(
                "Failed to set device_nguid {} for namespace {}",
                uuid, self.nsid
            )
        })?;
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
        self.set_enabled(false).with_context(|| {
            format!(
                "Failed to disable namespace {} before applying changes",
                self.nsid
            )
        })?;

        self.set_device_path(&ns.device_path)?;
        if let Some(uuid) = ns.device_uuid {
            self.set_device_uuid(&uuid)?;
        }
        if let Some(nguid) = ns.device_nguid {
            self.set_device_nguid(&nguid)?;
        }

        self.set_enabled(ns.enabled).with_context(|| {
            format!(
                "Failed to enable namespace {} after applying changes",
                self.nsid
            )
        })?;

        Ok(())
    }
}
