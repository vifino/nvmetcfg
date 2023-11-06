use clap::{Parser, Subcommand, ValueEnum};

use anyhow::Result;
use nvmetcfg::errors::Error;
use nvmetcfg::helpers::assert_valid_nqn;
use nvmetcfg::kernel::KernelConfig;
use nvmetcfg::state::{
    Namespace, Port, PortDelta, PortType, StateDelta, Subsystem, SubsystemDelta,
};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use uuid::Uuid;

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
    /// NVMe-oF Target Subsystem Commands
    Subsystem {
        #[command(subcommand)]
        subsystem_command: CliSubsystemCommands,
    },
    /// NVMe-oF Target Subsystem Namespace Commands
    Namespace {
        #[command(subcommand)]
        namespace_command: CliNamespaceCommands,
    },
}

#[derive(Subcommand)]
enum CliPortCommands {
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

#[derive(Subcommand)]
enum CliSubsystemCommands {
    /// Show detailed Subsystem information.
    Show,
    /// List only the Subsystem names.
    List,
    /// Create a new Subsystem.
    Add {
        /// NQN of the Subsystem.
        sub: String,

        // Set the model.
        model: Option<String>,

        // Set the serial.
        serial: Option<String>,
    },
    /// Remove an existing Subsystem.
    Remove {
        /// NQN of the Subsystem.
        sub: String,
    },
    /// List the Hosts allowed to use a Subsystem.
    ListHosts {
        /// NQN of the Subsystem.
        sub: String,
    },
    /// Add a Host/Initiator to the whitelist of a Subsystem.
    AddHost {
        /// NQN of the Subsystem.
        sub: String,
        /// NQN of the Host/Initiator.
        host: String,
    },
    /// Remove a Host/Initiator from the whitelist of a Subsystem.
    RemoveHost {
        /// NQN of the Subsystem.
        sub: String,
        /// NQN of the Host/Initiator.
        host: String,
    },
}

#[derive(Subcommand)]
enum CliNamespaceCommands {
    /// Show detailed information about the Namespaces of a Subsystem.
    Show {
        /// NQN of the Subsystem.
        sub: String,
    },
    /// List Namespaces of a Subsystem.
    List {
        /// NQN of the Subsystem.
        sub: String,
    },
    /// Add a namespace to an existing Subsystem.
    Add {
        /// NQN of the Subsystem.
        sub: String,

        /// Namespace ID of the new namespace.
        nsid: u32,

        /// Path to the block device.
        path: PathBuf,

        /// Do not enable it after creation.
        #[arg(long)]
        disabled: bool,

        /// Optionally set the UUID.
        #[arg(long)]
        uuid: Option<Uuid>,

        /// Optionally set the NGUID.
        #[arg(long)]
        nguid: Option<Uuid>,
    },
    /// Remove a Namespace from a Subsystem.
    Remove {
        /// NQN of the Subsystem.
        sub: String,

        /// Namespace ID of the namespace to be removed.
        nsid: u32,
    },
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
        },
        CliCommands::Subsystem { subsystem_command } => match subsystem_command {
            CliSubsystemCommands::Show => {
                let state = KernelConfig::gather_state()?;
                println!("Configured subsystems: {}", state.subsystems.len());
                for (nqn, sub) in state.subsystems {
                    println!("Subsystem: {}", nqn);
                    // TODO: this is not exactly true. :(
                    // We don't represent attr_allow_any_host in our abstraction.
                    // Perhaps we should make allowed_hosts Option<...>?
                    // That'd require some rework for sure..
                    println!("\tAllow Any Host: {}", sub.allowed_hosts.len() == 0);
                    if sub.allowed_hosts.len() != 0 {
                        println!("\tNumber of allowed Hosts: {}", sub.allowed_hosts.len());
                        println!("\tAllowed Hosts:");
                        for host in sub.allowed_hosts {
                            println!("\t\t{}", host);
                        }
                    }
                    println!("\tNumber of Namespaces: {}", sub.namespaces.len());
                    print!("\tNamespaces:");
                    for (nsid, _ns) in sub.namespaces {
                        print!(" {}", nsid)
                    }
                    println!();
                }
            }
            CliSubsystemCommands::List => {
                let state = KernelConfig::gather_state()?;
                for (nqn, _) in state.subsystems {
                    println!("{}", nqn);
                }
            }
            CliSubsystemCommands::Add { sub, model, serial } => {
                assert_valid_nqn(&sub)?;
                KernelConfig::apply_delta(vec![StateDelta::AddSubsystem(
                    sub,
                    Subsystem {
                        model,
                        serial,
                        allowed_hosts: BTreeSet::new(),
                        namespaces: BTreeMap::new(),
                    },
                )])?;
            }
            CliSubsystemCommands::Remove { sub } => {
                assert_valid_nqn(&sub)?;
                KernelConfig::apply_delta(vec![StateDelta::RemoveSubsystem(sub)])?;
            }
            CliSubsystemCommands::ListHosts { sub } => {
                assert_valid_nqn(&sub)?;
                let state = KernelConfig::gather_state()?;
                if let Some(subsystem) = state.subsystems.get(&sub) {
                    for host in &subsystem.allowed_hosts {
                        println!("{}", host);
                    }
                } else {
                    return Err(Error::NoSuchSubsystem(sub).into());
                }
            }
            CliSubsystemCommands::AddHost { sub, host } => {
                assert_valid_nqn(&sub)?;
                assert_valid_nqn(&host)?;
                KernelConfig::apply_delta(vec![StateDelta::UpdateSubsystem(
                    sub,
                    vec![SubsystemDelta::AddHost(host)],
                )])?;
            }
            CliSubsystemCommands::RemoveHost { sub, host } => {
                assert_valid_nqn(&sub)?;
                assert_valid_nqn(&host)?;
                KernelConfig::apply_delta(vec![StateDelta::UpdateSubsystem(
                    sub,
                    vec![SubsystemDelta::RemoveHost(host)],
                )])?;
            }
        },
        CliCommands::Namespace { namespace_command } => match namespace_command {
            CliNamespaceCommands::Show { sub } => {
                assert_valid_nqn(&sub)?;
                let state = KernelConfig::gather_state()?;
                if let Some(subsystem) = state.subsystems.get(&sub) {
                    println!("Number of Namespaces: {}", subsystem.namespaces.len());
                    for (nsid, ns) in &subsystem.namespaces {
                        println!("Namespace {}:", nsid);
                        println!("\tEnabled: {}", ns.enabled);
                        println!("\tDevice Path: {}", ns.device_path.display());
                        println!(
                            "\tDevice UUID: {}",
                            ns.device_uuid.expect("device_uuid should always be set")
                        );
                        println!(
                            "\tDevice NGUID: {}",
                            ns.device_nguid.expect("device_nguid should always be set")
                        );
                    }
                } else {
                    return Err(Error::NoSuchSubsystem(sub).into());
                }
            }
            CliNamespaceCommands::List { sub } => {
                assert_valid_nqn(&sub)?;
                let state = KernelConfig::gather_state()?;
                if let Some(subsystem) = state.subsystems.get(&sub) {
                    for (nsid, _ns) in &subsystem.namespaces {
                        println!("{}", nsid);
                    }
                } else {
                    return Err(Error::NoSuchSubsystem(sub).into());
                }
            }
            CliNamespaceCommands::Add {
                sub,
                nsid,
                path,
                disabled,
                uuid,
                nguid,
            } => {
                assert_valid_nqn(&sub)?;
                let new_ns = Namespace {
                    enabled: !disabled,
                    device_path: path,
                    device_uuid: uuid,
                    device_nguid: nguid,
                };
                KernelConfig::apply_delta(vec![StateDelta::UpdateSubsystem(
                    sub,
                    vec![SubsystemDelta::AddNamespace(nsid, new_ns)],
                )])?;
            }
            CliNamespaceCommands::Remove { sub, nsid } => {
                assert_valid_nqn(&sub)?;
                KernelConfig::apply_delta(vec![StateDelta::UpdateSubsystem(
                    sub,
                    vec![SubsystemDelta::RemoveNamespace(nsid)],
                )])?;
            }
        },
    };
    Ok(())
}
