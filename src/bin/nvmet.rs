use clap::{Parser, Subcommand, ValueEnum};

use nvmetcfg::errors::{Error, Result};
use nvmetcfg::helpers::assert_valid_nqn;
use nvmetcfg::kernel::KernelConfig;
use nvmetcfg::state::{Port, PortDelta, PortType, StateDelta};
use std::collections::BTreeSet;

#[derive(Parser)]
#[command(name = "nvmet")]
#[command(author = "Adrian 'vifino' Pistol <vifino@posteo.net>")]
#[command(about = "NVMe-oF Target Configuration CLI", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: CliCommands,
}

#[derive(Subcommand)]
enum CliCommands {
    /// NVMe-oF Target Port Commands
    Port {
        #[command(subcommand)]
        port_command: CliPortCommands,
    },
}

#[derive(Subcommand)]
enum CliPortCommands {
    /// Show Port information.
    Show,
    /// List only the Port names.
    List,
    /// Create a new Port.
    Add {
        /// Allow reconfiguring existing Port.
        #[arg(long)]
        existing: bool,

        /// Port ID to use.
        id: u32,

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
        id: u32,
    },
    /// List the subsystems provided by a Port.
    ListSubsystems {
        /// Port ID.
        id: u32,
    },
    /// Add a Subsystem to a Port.
    AddSubsystem {
        /// Port ID.
        id: u32,
        /// NQN of the Subsystem to add.
        nqn: String,
    },
    /// Remove a Subsystem from a Port.
    RemoveSubsystem {
        /// Port ID.
        id: u32,
        /// NQN of the Subsystem to remove.
        nqn: String,
    },
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum CliPortType {
    /// Loopback NVMe Device (for testing)
    Loop,
    /// NVMe over TCP
    Tcp,
    /// NVMe over RDMA/RoCE
    Rdma,
    /// NVMe over Fibre Channel
    Fc,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        CliCommands::Port { port_command } => match port_command {
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
                id,
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
                        id,
                        vec![PortDelta::UpdatePortType(pt)],
                    )]
                } else {
                    vec![StateDelta::AddPort(id, Port::new(pt, BTreeSet::new()))]
                };
                KernelConfig::apply_delta(state_delta)?;
            }
            CliPortCommands::Remove { id } => {
                KernelConfig::apply_delta(vec![StateDelta::RemovePort(id)])?;
            }
            CliPortCommands::ListSubsystems { id } => {
                let state = KernelConfig::gather_state()?;
                if let Some(port) = state.ports.get(&id) {
                    for sub in &port.subsystems {
                        println!("{}", sub);
                    }
                } else {
                    return Err(Error::NoSuchPort(id))?;
                }
            }
            CliPortCommands::AddSubsystem { id, nqn } => {
                assert_valid_nqn(&nqn)?;
                KernelConfig::apply_delta(vec![StateDelta::UpdatePort(
                    id,
                    vec![PortDelta::AddSubsystem(nqn.to_string())],
                )])?;
            }
            CliPortCommands::RemoveSubsystem { id, nqn } => {
                assert_valid_nqn(&nqn)?;
                KernelConfig::apply_delta(vec![StateDelta::UpdatePort(
                    id,
                    vec![PortDelta::RemoveSubsystem(nqn.to_string())],
                )])?;
            }
        },
    };
    Ok(())
}
