pub(super) mod helpers;
pub(super) mod sysfs;

use crate::errors::*;
use crate::state::*;
use sysfs::*;

pub struct KernelConfig {}

impl KernelConfig {
    pub fn gather_state() -> Result<State> {
        let mut state = State::default();
        for port in NvmetRoot::list_ports()? {
            if let Ok(port_type) = port.get_type() {
                state
                    .ports
                    .insert(port.id, Port::new(port_type, port.list_subsystems()?));
            }
        }
        Ok(state)
    }

    pub fn apply_delta(changes: Vec<StateDelta>) -> Result<()> {
        todo!()
    }
}
