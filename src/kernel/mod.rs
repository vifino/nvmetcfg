pub(super) mod sysfs;

use crate::errors::{Error, Result};
use crate::helpers::assert_valid_nqn;
use crate::state::{Namespace, Port, PortDelta, State, StateDelta, Subsystem, SubsystemDelta};
use anyhow::Context;
use std::collections::BTreeMap;
use sysfs::NvmetRoot;

pub struct KernelConfig {}

impl KernelConfig {
    pub fn gather_state() -> Result<State> {
        NvmetRoot::check_exists()?;

        let mut state = State::default();

        // Gather ports.
        for port in NvmetRoot::list_ports().context("Failed to gather port list")? {
            if let Ok(port_type) = port.get_type() {
                let subs = port.list_subsystems().with_context(|| {
                    format!("Failed to gather subsystem state for port {}", port.id)
                })?;
                state.ports.insert(port.id, Port::new(port_type, subs));
            }
        }

        // Gather subsystems.
        for subsystem in NvmetRoot::list_subsystems().context("Failed to gather subsystem list")? {
            // Gather namespaces of subsystem.
            let mut namespaces = BTreeMap::<u32, Namespace>::new();
            for (nsid, nvmetns) in subsystem.list_namespaces()? {
                let ns = nvmetns.get_namespace().with_context(|| {
                    format!(
                        "Failed to get namespace {} for subsystem {}",
                        nsid, subsystem.nqn
                    )
                })?;
                namespaces.insert(nsid, ns);
            }

            let sub = Subsystem {
                model: Some(subsystem.get_model().with_context(|| {
                    format!("Failed to gather model for subsystem {}", subsystem.nqn)
                })?),
                serial: Some(subsystem.get_serial().with_context(|| {
                    format!("Failed to gather serial for subsystem {}", subsystem.nqn)
                })?),
                allowed_hosts: subsystem.list_hosts().with_context(|| {
                    format!(
                        "Failed to gather allowed hosts for subsystem {}",
                        subsystem.nqn
                    )
                })?,
                namespaces,
            };
            state.subsystems.insert(subsystem.nqn, sub);
        }

        Ok(state)
    }

    pub fn apply_delta(changes: Vec<StateDelta>) -> Result<()> {
        for change in changes {
            match change {
                StateDelta::AddPort(id, port) => {
                    let p = NvmetRoot::create_port(id)
                        .with_context(|| format!("Failed to add new port {id}"))?;
                    p.set_type(port.port_type)
                        .with_context(|| format!("Failed to set new port type for port {id}"))?;
                    for sub in &port.subsystems {
                        assert_valid_nqn(sub).with_context(|| {
                            format!("Failed to validate new port subsystems for port {id}")
                        })?;
                    }
                    p.set_subsystems(&port.subsystems).with_context(|| {
                        format!("Failed to set new port subsystems for port {id}")
                    })?;
                }
                StateDelta::UpdatePort(id, deltas) => {
                    if !NvmetRoot::has_port(id)? {
                        return Err(Into::<anyhow::Error>::into(Error::NoSuchPort(id)))
                            .with_context(|| format!("Failed to update port {id}"));
                    }
                    let p = NvmetRoot::open_port(id);
                    for delta in deltas {
                        match delta {
                            PortDelta::UpdatePortType(pt) => p.set_type(pt).with_context(|| {
                                format!("Failed to update port type of port {id}")
                            })?,
                            PortDelta::AddSubsystem(nqn) => {
                                p.enable_subsystem(&nqn).with_context(|| {
                                    format!("Failed to add subsystem {nqn} to port {id}")
                                })?
                            }
                            PortDelta::RemoveSubsystem(nqn) => {
                                p.disable_subsystem(&nqn).with_context(|| {
                                    format!("Failed to remove subsytem {nqn} from port {id}")
                                })?
                            }
                        }
                    }
                }
                StateDelta::RemovePort(id) => {
                    NvmetRoot::delete_port(id)
                        .with_context(|| format!("Failed to remove port {id}"))?;
                }

                StateDelta::AddSubsystem(nqn, sub) => {
                    if NvmetRoot::has_subsystem(&nqn)? {
                        return Err(Into::<anyhow::Error>::into(Error::ExistingSubsystem(
                            nqn.to_owned(),
                        )))
                        .with_context(|| format!("Failed to add new subsystem {nqn}"));
                    }
                    let nvmetsub = NvmetRoot::create_subsystem(&nqn)
                        .with_context(|| format!("Failed to add new subsystem {nqn}"))?;
                    if let Some(model) = sub.model {
                        nvmetsub.set_model(&model).with_context(|| {
                            format!("Failed to set model for new subsystem {nqn}")
                        })?;
                    }
                    if let Some(serial) = sub.serial {
                        nvmetsub.set_serial(&serial).with_context(|| {
                            format!("Failed to set serial for new subsystem {nqn}")
                        })?;
                    }
                    nvmetsub.set_namespaces(&sub.namespaces).with_context(|| {
                        format!("Failed to add namespaces for new subsystem {nqn}")
                    })?;
                    nvmetsub.set_hosts(&sub.allowed_hosts).with_context(|| {
                        format!("Failed to set allowed hosts for new subsystem {nqn}")
                    })?;
                }
                StateDelta::UpdateSubsystem(nqn, deltas) => {
                    if !NvmetRoot::has_subsystem(&nqn)? {
                        return Err(Into::<anyhow::Error>::into(Error::NoSuchSubsystem(
                            nqn.to_owned(),
                        )))
                        .with_context(|| format!("Failed to update existing subsystem {nqn}"));
                    }
                    let nvmetsub = NvmetRoot::open_subsystem(&nqn)
                        .with_context(|| format!("Failed to update subsystem {nqn}"))?;
                    for delta in deltas {
                        match delta {
                            SubsystemDelta::UpdateModel(model) => {
                                nvmetsub.set_model(&model).with_context(|| {
                                    format!("Failed to update model for subsystem {nqn}")
                                })?
                            }
                            SubsystemDelta::UpdateSerial(serial) => {
                                nvmetsub.set_serial(&serial).with_context(|| {
                                    format!("Failed to update serial for subsystem {nqn}")
                                })?
                            }
                            SubsystemDelta::AddHost(host) => {
                                nvmetsub.enable_host(&host).with_context(|| {
                                    format!("Failed to add allowed host to subsystem {nqn}")
                                })?
                            }
                            SubsystemDelta::RemoveHost(host) => {
                                nvmetsub.disable_host(&host).with_context(|| {
                                    format!("Failed to remove allowed host from subsystem {nqn}")
                                })?
                            }
                            SubsystemDelta::AddNamespace(nsid, ns) => {
                                let nvmetns =
                                    nvmetsub.create_namespace(nsid).with_context(|| {
                                        format!("Failed to add namespace for subsystem {nqn}")
                                    })?;
                                nvmetns.set_namespace(&ns).with_context(|| {
                                    format!("Failed to set new namespace for subsystem {nqn}")
                                })?;
                            }
                            SubsystemDelta::UpdateNamespace(nsid, ns) => {
                                let nvmetns = nvmetsub.open_namespace(nsid).with_context(|| {
                                    format!("Failed to update namespace for subsystem {nqn}")
                                })?;
                                nvmetns.set_namespace(&ns).with_context(|| {
                                    format!("Failed to update namespace for subsystem {nqn}")
                                })?;
                            }
                            SubsystemDelta::RemoveNamespace(nsid) => {
                                nvmetsub.delete_namespace(nsid).with_context(|| {
                                    format!("Failed to remove namespace for subsystem {nqn}")
                                })?;
                            }
                        }
                    }
                }
                StateDelta::RemoveSubsystem(nqn) => {
                    if !NvmetRoot::has_subsystem(&nqn)? {
                        return Err(Into::<anyhow::Error>::into(Error::NoSuchSubsystem(
                            nqn.to_owned(),
                        )))
                        .with_context(|| format!("Failed to remove existing subsystem {nqn}"));
                    }

                    // Fetch global hosts just before we remove the subsystem.
                    let prev_hosts = NvmetRoot::list_hosts()
                        .with_context(|| format!("Failed to list all allowed hosts before removing existing subsystem {nqn}"))?;
                    let our_hosts = NvmetRoot::open_subsystem(&nqn)?
                        .list_hosts()
                        .with_context(|| format!("Failed to list subsystem hosts before removing existing subsystem {nqn}"))?;

                    // Before removing the subsystem, we need to remove all references to it.
                    for port in NvmetRoot::list_ports().with_context(|| {
                        format!("Failed to list ports before removing existing subsystem {nqn}")
                    })? {
                        if port.has_subsystem(&nqn).with_context(|| {
                            format!(
                                "Failed to check if port has subsystem {nqn} before removing it"
                            )
                        })? {
                            port.disable_subsystem(&nqn).with_context(|| format!("Failed to disable subsystem {nqn} from all ports before removing it"))?;
                        }
                    }

                    NvmetRoot::delete_subsystem(&nqn)
                        .with_context(|| format!("Failed to remove subsystem {nqn}"))?;

                    // Iterate over all remaining subsystems and find what host we're missing now.
                    let current_hosts = NvmetRoot::list_hosts().with_context(|| format!("Failed to list all allowed hosts before removing existing subsystem {nqn}"))?;
                    for unused_host in prev_hosts.difference(&current_hosts) {
                        if our_hosts.contains(unused_host) {
                            NvmetRoot::remove_host(unused_host).with_context(|| {
                                format!(
                                    "Failed to remove unused hosts after deletion of subsystem {nqn}"
                                )
                            })?;
                        }
                    }
                }
            }
        }
        Ok(())
    }
}
