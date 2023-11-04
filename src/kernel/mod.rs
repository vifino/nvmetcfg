pub(super) mod sysfs;

use crate::errors::*;
use crate::helpers::assert_valid_nqn;
use crate::state::*;
use std::collections::HashMap;
use sysfs::*;

pub struct KernelConfig {}

impl KernelConfig {
    pub fn gather_state() -> Result<State> {
        NvmetRoot::check_exists()?;

        let mut state = State::default();

        // Gather ports.
        for port in NvmetRoot::list_ports()? {
            if let Ok(port_type) = port.get_type() {
                let subs = port.list_subsystems()?;
                state.ports.insert(port.id, Port::new(port_type, subs));
            }
        }

        // Gather subsystems.
        for subsystem in NvmetRoot::list_subsystems()? {
            // Gather namespaces of subsystem.
            let mut namespaces = HashMap::<u32, Namespace>::new();
            for (nsid, nvmetns) in subsystem.list_namespaces()? {
                let ns = nvmetns.get_namespace()?;
                namespaces.insert(nsid, ns);
            }

            let sub = Subsystem {
                model: Some(subsystem.get_model()?),
                serial: Some(subsystem.get_serial()?),
                allowed_hosts: subsystem.list_hosts()?,
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
                    let p = NvmetRoot::create_port(id)?;
                    p.set_type(port.port_type)?;
                    for sub in &port.subsystems {
                        assert_valid_nqn(sub)?;
                    }
                    p.set_subsystems(port.subsystems)?;
                }
                StateDelta::UpdatePort(id, deltas) => {
                    if !NvmetRoot::has_port(id)? {
                        return Err(Error::NoSuchPort(id));
                    }
                    let p = NvmetRoot::create_port(id)?;
                    for delta in deltas {
                        match delta {
                            PortDelta::UpdatePortType(pt) => p.set_type(pt)?,
                            PortDelta::AddSubsystem(nqn) => p.create_subsystem(&nqn)?,
                            PortDelta::RemoveSubsystem(nqn) => p.delete_subsystem(&nqn)?,
                        }
                    }
                }
                StateDelta::RemovePort(id) => {
                    NvmetRoot::delete_port(id)?;
                }

                StateDelta::AddSubsystem(nqn, sub) => {
                    if NvmetRoot::has_subsystem(&nqn)? {
                        return Err(Error::ExistingSubsystem(nqn.to_string()));
                    }
                    let nvmetsub = NvmetRoot::create_subsystem(&nqn)?;
                    if let Some(model) = sub.model {
                        nvmetsub.set_model(&model)?;
                    }
                    if let Some(serial) = sub.serial {
                        nvmetsub.set_serial(&serial)?;
                    }
                    nvmetsub.set_namespaces(sub.namespaces)?;
                    nvmetsub.set_hosts(sub.allowed_hosts)?;
                }
                StateDelta::UpdateSubsystem(nqn, deltas) => {
                    if !NvmetRoot::has_subsystem(&nqn)? {
                        return Err(Error::NoSuchSubsystem(nqn.to_string()));
                    }
                    let nvmetsub = NvmetRoot::create_subsystem(&nqn)?;
                    for delta in deltas {
                        match delta {
                            SubsystemDelta::UpdateModel(model) => nvmetsub.set_model(&model)?,
                            SubsystemDelta::UpdateSerial(serial) => nvmetsub.set_serial(&serial)?,
                            SubsystemDelta::AddHost(host) => nvmetsub.enable_host(&host)?,
                            SubsystemDelta::RemoveHost(host) => nvmetsub.disable_host(&host)?,
                            SubsystemDelta::AddNamespace(nsid, ns)
                            | SubsystemDelta::UpdateNamespace(nsid, ns) => {
                                let nvmetns = nvmetsub.create_namespace(nsid)?;
                                nvmetns.set_namespace(&ns)?;
                            }
                            SubsystemDelta::RemoveNamespace(nsid) => {
                                nvmetsub.delete_namespace(nsid)?
                            }
                        }
                    }
                }
                StateDelta::RemoveSubsystem(nqn) => {
                    if !NvmetRoot::has_subsystem(&nqn)? {
                        return Err(Error::NoSuchSubsystem(nqn.to_string()));
                    }

                    // Fetch global hosts just before we remove the subsystem.
                    let prev_hosts = NvmetRoot::list_hosts()?;
                    let our_hosts = NvmetRoot::create_subsystem(&nqn)?.list_hosts()?;

                    // Before removing the subsystem, we need to remove all references to it.
                    for port in NvmetRoot::list_ports()? {
                        if port.has_subsystem(&nqn)? {
                            port.delete_subsystem(&nqn)?;
                        }
                    }

                    NvmetRoot::delete_subsystem(&nqn)?;

                    // Iterate over all remaining subsystems and find what host we're missing now.
                    let current_hosts = NvmetRoot::list_hosts()?;
                    for unused_host in prev_hosts.difference(&current_hosts) {
                        if our_hosts.contains(unused_host) {
                            NvmetRoot::remove_host(&unused_host)?;
                        }
                    }
                }
            }
        }
        Ok(())
    }
}
