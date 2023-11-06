use anyhow::Result;
use clap::{Subcommand, ValueEnum};
use nvmetcfg::errors::Error;
use nvmetcfg::helpers::assert_valid_nqn;
use nvmetcfg::kernel::KernelConfig;
use nvmetcfg::state::{Port, PortDelta, PortType, StateDelta};
use std::collections::BTreeSet;

#[derive(Subcommand)]
pub(super) enum CliPortCommands {
    /// Show detailed Port information.
    Show,
    /// List only the Port names.
    List,
    /// Create a new Port.
    Add {
        /// Allow reconfiguring existing Port.
        #[arg(long)]
        existing: bool,

        /// Port ID to use.
        pid: u32,

        /// Type of Port.
        port_type: CliPortType,

        // Port Address to use.
        #[arg(
            required_if_eq("port_type", "tcp"),
            required_if_eq("port_type", "rdma"),
            required_if_eq("port_type", "fc")
        )]
        address: Option<String>,
    },
    /// Remove a Port.
    Remove {
        // Port ID to remove.
        pid: u32,
    },
    /// List the subsystems provided by a Port.
    ListSubsystems {
        /// Port ID.
        pid: u32,
    },
    /// Add a Subsystem to a Port.
    AddSubsystem {
        /// Port ID.
        pid: u32,
        /// NQN of the Subsystem to add.
        sub: String,
    },
    /// Remove a Subsystem from a Port.
    RemoveSubsystem {
        /// Port ID.
        pid: u32,
        /// NQN of the Subsystem to remove.
        sub: String,
    },
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub(super) enum CliPortType {
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
            CliPortCommands::List => {
                let state = KernelConfig::gather_state()?;
                for (id, _) in state.ports {
                    println!("{}", id);
                }
            }
            CliPortCommands::Show => {
                let state = KernelConfig::gather_state()?;
                println!("Configured ports: {}", state.ports.len());
                for (id, port) in state.ports {
                    println!("Port {}:", id);
                    println!("\tType: {:?}", port.port_type);
                    println!("\tSubsystems: {}", port.subsystems.len());
                    for sub in port.subsystems {
                        println!("\t\t{}", sub);
                    }
                }
            }
            CliPortCommands::Add {
                existing,
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

                let state_delta = if existing {
                    vec![StateDelta::UpdatePort(
                        pid,
                        vec![PortDelta::UpdatePortType(pt)],
                    )]
                } else {
                    vec![StateDelta::AddPort(pid, Port::new(pt, BTreeSet::new()))]
                };
                KernelConfig::apply_delta(state_delta)?;
            }
            CliPortCommands::Remove { pid } => {
                KernelConfig::apply_delta(vec![StateDelta::RemovePort(pid)])?;
            }
            CliPortCommands::ListSubsystems { pid } => {
                let state = KernelConfig::gather_state()?;
                if let Some(port) = state.ports.get(&pid) {
                    for sub in &port.subsystems {
                        println!("{}", sub);
                    }
                } else {
                    return Err(Error::NoSuchPort(pid))?;
                }
            }
            CliPortCommands::AddSubsystem { pid, sub } => {
                assert_valid_nqn(&sub)?;
                KernelConfig::apply_delta(vec![StateDelta::UpdatePort(
                    pid,
                    vec![PortDelta::AddSubsystem(sub)],
                )])?;
            }
            CliPortCommands::RemoveSubsystem { pid, sub } => {
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