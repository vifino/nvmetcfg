use clap::Command;

use nvmetcfg::kernel::KernelConfig;
//use nvmetcfg::state::*;

fn main() -> nvmetcfg::errors::Result<()> {
    let matches = Command::new("nvmet")
        .about("NVMe Target Configuration CLI")
        .subcommand_required(true)
        .subcommand(Command::new("list-ports").about("List configured ports"))
        .get_matches();

    match matches.subcommand() {
        Some(("list-ports", _matches)) => {
            let state = KernelConfig::gather_state()?;
            println!("Configured ports: {}", state.ports.len());
            for (id, port) in state.ports {
                println!("Port {}:", id);
                println!("\tType: {:?}", port.port_type);
            }
        }
        _ => unreachable!("calp should never let this happen"),
    };
    Ok(())
}
