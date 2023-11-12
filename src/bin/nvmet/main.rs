mod namespace;
mod port;
mod state;
mod subsystem;

use anyhow::Result;
use clap::{Parser, Subcommand};

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
        port_command: port::CliPortCommands,
    },
    /// NVMe-oF Target Subsystem Commands
    Subsystem {
        #[command(subcommand)]
        subsystem_command: subsystem::CliSubsystemCommands,
    },
    /// NVMe-oF Target Subsystem Namespace Commands
    Namespace {
        #[command(subcommand)]
        namespace_command: namespace::CliNamespaceCommands,
    },
    /// NVMe-oF Target Subsystem State Management Commands
    State {
        #[command(subcommand)]
        state_command: state::CliStateCommands,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        CliCommands::Port { port_command } => port::CliPortCommands::parse(port_command),
        CliCommands::Subsystem { subsystem_command } => {
            subsystem::CliSubsystemCommands::parse(subsystem_command)
        }
        CliCommands::Namespace { namespace_command } => {
            namespace::CliNamespaceCommands::parse(namespace_command)
        }
        CliCommands::State { state_command } => state::CliStateCommands::parse(state_command),
    }
}
