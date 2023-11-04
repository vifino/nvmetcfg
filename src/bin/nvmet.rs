use clap::{Parser, Subcommand, ValueEnum};

use nvmetcfg::helpers::assert_valid_nqn;
use nvmetcfg::kernel::KernelConfig;
use nvmetcfg::state::{PortDelta, StateDelta};

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
    /// List only the Port names.
    List,
    /// Show Port information.
    Show,
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
        address: Option<String>,
    },
    /// Remove a Port.
    Remove {
        // Port ID to remove.
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

fn main() -> nvmetcfg::errors::Result<()> {
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
            CliPortCommands::Add { .. } => {
                todo!()
            }
            CliPortCommands::Remove { id } => {
                KernelConfig::apply_delta(vec![StateDelta::RemovePort(id)])?;
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
