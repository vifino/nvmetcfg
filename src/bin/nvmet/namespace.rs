use anyhow::Result;
use clap::Subcommand;
use nvmetcfg::errors::Error;
use nvmetcfg::helpers::assert_valid_nqn;
use nvmetcfg::kernel::KernelConfig;
use nvmetcfg::state::{Namespace, StateDelta, SubsystemDelta};

use std::path::PathBuf;
use uuid::Uuid;

#[derive(Subcommand)]
pub enum CliNamespaceCommands {
    /// Show detailed information about the Namespaces of a Subsystem.
    Show {
        /// NVMe Qualified Name of the Subsystem.
        sub: String,
    },
    /// List Namespaces of a Subsystem.
    List {
        /// NVMe Qualified Name of the Subsystem.
        sub: String,
    },
    /// Add a Namespace to an existing Subsystem.
    Add {
        /// NVMe Qualified Name of the Subsystem.
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
    /// Update an existing Namespace of a Subsystem.
    Update {
        /// NVMe Qualified Name of the Subsystem.
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
        /// NVMe Qualified Name of the Subsystem.
        sub: String,

        /// Namespace ID of the namespace to be removed.
        nsid: u32,
    },
}

impl CliNamespaceCommands {
    pub(super) fn parse(command: Self) -> Result<()> {
        match command {
            Self::Show { sub } => {
                assert_valid_nqn(&sub)?;
                let state = KernelConfig::gather_state()?;
                if let Some(subsystem) = state.subsystems.get(&sub) {
                    println!("Number of Namespaces: {}", subsystem.namespaces.len());
                    for (nsid, ns) in &subsystem.namespaces {
                        println!("Namespace {nsid}:");
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
            Self::List { sub } => {
                assert_valid_nqn(&sub)?;
                let state = KernelConfig::gather_state()?;
                if let Some(subsystem) = state.subsystems.get(&sub) {
                    for nsid in subsystem.namespaces.keys() {
                        println!("{nsid}");
                    }
                } else {
                    return Err(Error::NoSuchSubsystem(sub).into());
                }
            }
            Self::Add {
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
            Self::Update {
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
                    vec![SubsystemDelta::UpdateNamespace(nsid, new_ns)],
                )])?;
            }
            Self::Remove { sub, nsid } => {
                assert_valid_nqn(&sub)?;
                KernelConfig::apply_delta(vec![StateDelta::UpdateSubsystem(
                    sub,
                    vec![SubsystemDelta::RemoveNamespace(nsid)],
                )])?;
            }
        }
        Ok(())
    }
}
