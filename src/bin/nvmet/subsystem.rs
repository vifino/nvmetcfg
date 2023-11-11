use anyhow::Result;
use clap::Subcommand;
use nvmetcfg::errors::Error;
use nvmetcfg::helpers::{assert_compliant_nqn, assert_valid_nqn};
use nvmetcfg::kernel::KernelConfig;
use nvmetcfg::state::{StateDelta, Subsystem, SubsystemDelta};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Subcommand)]
pub(super) enum CliSubsystemCommands {
    /// Show detailed Subsystem information.
    Show,
    /// List only the Subsystem names.
    List,
    /// Create a new Subsystem.
    Add {
        /// NVMe Qualified Name of the Subsystem.
        /// This should follow the supported formats in the NVMe specification.
        ///
        /// Examples:
        ///
        /// - nqn.2014-08.com.example:nvme.host.sys.xyz
        ///
        /// - nqn.2014-08.org.nvmexpress:uuid:f81d4fae-7dec-11d0-a765-00a0c91e6bf6
        sub: String,

        // Set the model.
        model: Option<String>,

        // Set the serial.
        serial: Option<String>,
    },
    /// Remove an existing Subsystem.
    Remove {
        /// NVMe Qualified Name of the Subsystem.
        sub: String,
    },
    /// List the Hosts allowed to use a Subsystem.
    ListHosts {
        /// NVMe Qualified Name of the Subsystem.
        sub: String,
    },
    /// Add a Host/Initiator to the whitelist of a Subsystem.
    AddHost {
        /// NVMe Qualified Name of the Subsystem.
        sub: String,
        /// NVMe Qualified Name of the Host/Initiator.
        host: String,
    },
    /// Remove a Host/Initiator from the whitelist of a Subsystem.
    RemoveHost {
        /// NVMe Qualified Name of the Subsystem.
        sub: String,
        /// NVMe Qualified Name of the Host/Initiator.
        host: String,
    },
}

impl CliSubsystemCommands {
    pub(super) fn parse(command: Self) -> Result<()> {
        match command {
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
                assert_compliant_nqn(&sub)?;
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
        }
        Ok(())
    }
}
