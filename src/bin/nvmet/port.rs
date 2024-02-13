use anyhow::Result;
use clap::{Subcommand, ValueEnum};
use nvmetcfg::errors::Error;
use nvmetcfg::helpers::assert_valid_nqn;
use nvmetcfg::kernel::KernelConfig;
use nvmetcfg::state::{Port, PortDelta, PortType, StateDelta};
use std::collections::BTreeSet;

#[derive(Subcommand)]
pub enum CliPortCommands {
    /// Show detailed Port information.
    Show,
    /// List only the Port names.
    List,
    /// Create a new Port.
    Add {
        /// Port ID to use.
        pid: u16,

        /// Type of Port.
        port_type: CliPortType,

        /// Port Address to use.
        ///
        /// For Tcp and Rdma port types, this should be an IP address and Port:
        /// IPv4: 1.2.3.4:4420
        /// IPv6: [::1]:4420
        ///
        /// For Fibre Channel transport, this should be the WWNN/WWPN in the following format:
        /// Long:  nn-0x1000000044001123:pn-0x2000000055001123
        /// Short: nn-1000000044001123:pn-2000000055001123
        #[arg(
            verbatim_doc_comment,
            required_if_eq("port_type", "tcp"),
            required_if_eq("port_type", "rdma"),
            required_if_eq("port_type", "fc")
        )]
        address: Option<String>,
    },
    /// Update an existing Port.
    Update {
        /// Port ID to use.
        pid: u16,

        /// Type of Port.
        port_type: CliPortType,

        /// Port Address to use.
        ///
        /// For Tcp and Rdma port types, this should be an IP address and Port:
        /// IPv4: 1.2.3.4:4420
        /// IPv6: [::1]:4420
        ///
        /// For Fibre Channel transport, this should be the WWNN/WWPN in the following format:
        /// Long:  nn-0x1000000044001123:pn-0x2000000055001123
        /// Short: nn-1000000044001123:pn-2000000055001123
        #[arg(
            verbatim_doc_comment,
            required_if_eq("port_type", "tcp"),
            required_if_eq("port_type", "rdma"),
            required_if_eq("port_type", "fc")
        )]
        address: Option<String>,
    },
    /// Remove a Port.
    Remove {
        /// Port ID to remove.
        pid: u16,
    },
    /// List the subsystems provided by a Port.
    ListSubsystems {
        /// Port ID.
        pid: u16,
    },
    /// Add a Subsystem to a Port.
    AddSubsystem {
        /// Port ID.
        pid: u16,
        /// NVMe Qualified Name of the Subsystem to add.
        sub: String,
    },
    /// Remove a Subsystem from a Port.
    RemoveSubsystem {
        /// Port ID.
        pid: u16,
        /// NVMe Qualified Name of the Subsystem to remove.
        sub: String,
    },
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub enum CliPortType {
    /// Loopback NVMe Device (for testing)
    Loop,
    /// NVMe over TCP
    Tcp,
    /// NVMe over RDMA/RoCE
    Rdma,
    /// NVMe over Fibre Channel
    Fc,
}

impl CliPortCommands {
    pub(super) fn parse(command: Self) -> Result<()> {
        match command {
            Self::List => {
                let state = KernelConfig::gather_state()?;
                for (id, _) in state.ports {
                    println!("{id}");
                }
            }
            Self::Show => {
                let state = KernelConfig::gather_state()?;
                println!("Configured ports: {}", state.ports.len());
                for (id, port) in state.ports {
                    println!("Port {id}:");
                    println!("\tType: {:?}", port.port_type);
                    println!("\tSubsystems: {}", port.subsystems.len());
                    for sub in port.subsystems {
                        println!("\t\t{sub}");
                    }
                }
            }
            Self::Add {
                pid,
                port_type,
                address,
            } => {
                let pt = match port_type {
                    CliPortType::Loop => PortType::Loop,
                    CliPortType::Tcp => PortType::Tcp(address.unwrap().parse()?),
                    CliPortType::Rdma => PortType::Rdma(address.unwrap().parse()?),
                    CliPortType::Fc => PortType::FibreChannel(address.unwrap().parse()?),
                };

                let state_delta = vec![StateDelta::AddPort(pid, Port::new(pt, BTreeSet::new()))];
                KernelConfig::apply_delta(state_delta)?;
            }
            Self::Update {
                pid,
                port_type,
                address,
            } => {
                let pt = match port_type {
                    CliPortType::Loop => PortType::Loop,
                    CliPortType::Tcp => PortType::Tcp(address.unwrap().parse()?),
                    CliPortType::Rdma => PortType::Rdma(address.unwrap().parse()?),
                    CliPortType::Fc => PortType::FibreChannel(address.unwrap().parse()?),
                };

                let state_delta = vec![StateDelta::UpdatePort(
                    pid,
                    vec![PortDelta::UpdatePortType(pt)],
                )];
                KernelConfig::apply_delta(state_delta)?;
            }
            Self::Remove { pid } => {
                KernelConfig::apply_delta(vec![StateDelta::RemovePort(pid)])?;
            }
            Self::ListSubsystems { pid } => {
                let state = KernelConfig::gather_state()?;
                if let Some(port) = state.ports.get(&pid) {
                    for sub in &port.subsystems {
                        println!("{sub}");
                    }
                } else {
                    return Err(Error::NoSuchPort(pid))?;
                }
            }
            Self::AddSubsystem { pid, sub } => {
                assert_valid_nqn(&sub)?;
                KernelConfig::apply_delta(vec![StateDelta::UpdatePort(
                    pid,
                    vec![PortDelta::AddSubsystem(sub)],
                )])?;
            }
            Self::RemoveSubsystem { pid, sub } => {
                assert_valid_nqn(&sub)?;
                KernelConfig::apply_delta(vec![StateDelta::UpdatePort(
                    pid,
                    vec![PortDelta::RemoveSubsystem(sub)],
                )])?;
            }
        }
        Ok(())
    }
}
