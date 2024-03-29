use anyhow::{Context, Result};
use clap::Subcommand;
use nvmetcfg::{errors::Error, kernel::KernelConfig, state::State};
use serde::{Deserialize, Serialize};
use std::{fs::File, path::PathBuf};

#[derive(Subcommand)]
pub enum CliStateCommands {
    /// Save the NVMe-oF Target configuration to file.
    Save {
        /// File to save the state to.
        file: PathBuf,
    },
    /// Restore the NVMe-oF Target configuration from previously saved configuration.
    Restore {
        /// File from which to load the state.
        file: PathBuf,
    },
    /// Remove all configuration of the NVMe-oF Target.
    Clear,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigFile {
    // TODO: Make this proper?
    #[serde(default)]
    pub version: u32,
    #[serde(flatten)]
    pub state: State,
}

impl CliStateCommands {
    pub(super) fn parse(command: Self) -> Result<()> {
        match command {
            CliStateCommands::Save { file } => {
                let f = File::create(file).context("Failed to open state file for writing")?;
                let state =
                    KernelConfig::gather_state().context("Failed to gather state for writing")?;
                let config = ConfigFile { version: 0, state };
                serde_yaml::to_writer(f, &config)
                    .context("Failed to write current state to file")?;
                println!("Sucessfully written current state to file.");
                Ok(())
            }
            CliStateCommands::Restore { file } => {
                let f = File::open(file).context("Failed to open state file for reading")?;
                let config: ConfigFile =
                    serde_yaml::from_reader(f).context("Failed to read from state file")?;
                if config.version != 0 {
                    return Err(Error::UnsupportedConfigVersion(config.version).into());
                }
                let desired = config.state;
                let current =
                    KernelConfig::gather_state().context("Failed to gather state for writing")?;
                let delta = current.get_deltas(&desired);
                let delta_len = delta.len();
                if delta_len == 0 {
                    println!(
                        "No changes made: System state has no changes compared to saved state."
                    );
                } else {
                    KernelConfig::apply_delta(delta)
                        .context("Failed to apply state delta between current and saved state")?;
                    println!("Sucessfully applied saved state: {delta_len} state changes.");
                }
                Ok(())
            }
            CliStateCommands::Clear => {
                let current =
                    KernelConfig::gather_state().context("Failed to gather state for writing")?;
                let delta = current.get_deltas(&State::default());
                let delta_len = delta.len();
                if delta_len == 0 {
                    println!("No changes made: System state has no configuration.");
                } else {
                    KernelConfig::apply_delta(delta)
                        .context("Failed to apply state delta between current and saved state")?;
                    println!("Sucessfully cleared configuration: {delta_len} state changes.");
                }
                Ok(())
            }
        }
    }
}
