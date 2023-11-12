use super::types::{Namespace, Port, PortType, State, Subsystem};
use crate::helpers::get_btreemap_differences;

// Define the representation of differences to the state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StateDelta {
    AddPort(u16, Port),
    UpdatePort(u16, Vec<PortDelta>),
    RemovePort(u16),

    AddSubsystem(String, Subsystem),
    UpdateSubsystem(String, Vec<SubsystemDelta>),
    RemoveSubsystem(String),
}

impl State {
    #[must_use]
    pub fn get_deltas(&self, other: &Self) -> Vec<StateDelta> {
        let mut deltas = Vec::new();

        let port_changes = get_btreemap_differences(&self.ports, &other.ports);
        let subsystem_changes = get_btreemap_differences(&self.subsystems, &other.subsystems);

        // Delete Ports not in new.
        for removed in &port_changes.removed {
            deltas.push(StateDelta::RemovePort(*removed));
        }

        // Delete Subsystems not in new.
        for removed in &subsystem_changes.removed {
            deltas.push(StateDelta::RemoveSubsystem(removed.to_string()));
        }

        // Update Subsystems
        for updated in &subsystem_changes.changed {
            deltas.push(StateDelta::UpdateSubsystem(
                updated.to_string(),
                self.subsystems
                    .get(updated)
                    .unwrap()
                    .get_deltas(other.subsystems.get(updated).unwrap()),
            ));
        }

        // Add Subsystems not in base.
        for added in &subsystem_changes.added {
            deltas.push(StateDelta::AddSubsystem(
                added.to_string(),
                other.subsystems.get(added).unwrap().clone(),
            ));
        }

        // Update Ports.
        for updated in &port_changes.changed {
            deltas.push(StateDelta::UpdatePort(
                *updated,
                self.ports
                    .get(updated)
                    .unwrap()
                    .get_deltas(other.ports.get(updated).unwrap()),
            ));
        }

        // Add Ports not in base.
        for added in &port_changes.added {
            deltas.push(StateDelta::AddPort(
                *added,
                other.ports.get(added).unwrap().clone(),
            ));
        }

        deltas
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PortDelta {
    UpdatePortType(PortType),

    AddSubsystem(String),
    RemoveSubsystem(String),
}

impl Port {
    #[must_use]
    pub fn get_deltas(&self, other: &Self) -> Vec<PortDelta> {
        let mut deltas = Vec::new();

        // Remove subsystems not in self.
        for removed_sub in self.subsystems.difference(&other.subsystems) {
            deltas.push(PortDelta::RemoveSubsystem(removed_sub.clone()));
        }

        // Updated Port Type.
        if self.port_type != other.port_type {
            deltas.push(PortDelta::UpdatePortType(other.port_type));
        }

        // Add subsystems not in self.
        for new_sub in other.subsystems.difference(&self.subsystems) {
            deltas.push(PortDelta::AddSubsystem(new_sub.clone()));
        }

        deltas
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubsystemDelta {
    UpdateModel(String),
    UpdateSerial(String),

    AddHost(String),
    RemoveHost(String),

    AddNamespace(u32, Namespace),
    UpdateNamespace(u32, Namespace),
    RemoveNamespace(u32),
}

impl Subsystem {
    #[must_use]
    pub fn get_deltas(&self, other: &Self) -> Vec<SubsystemDelta> {
        let mut deltas = Vec::new();

        let namespace_changes = get_btreemap_differences(&self.namespaces, &other.namespaces);

        // Updated model
        if self.model != other.model {
            if let Some(model) = &other.model {
                deltas.push(SubsystemDelta::UpdateModel(model.clone()));
            }
        }

        // Updated serial
        if self.serial != other.serial {
            if let Some(serial) = &other.serial {
                deltas.push(SubsystemDelta::UpdateSerial(serial.clone()));
            }
        }

        // Add hosts not in self.
        for new_host in other.allowed_hosts.difference(&self.allowed_hosts) {
            deltas.push(SubsystemDelta::AddHost(new_host.clone()));
        }

        // Delete namespaces not in other.
        for removed in &namespace_changes.removed {
            deltas.push(SubsystemDelta::RemoveNamespace(*removed));
        }

        // Update namespaces.
        for updated in &namespace_changes.changed {
            deltas.push(SubsystemDelta::UpdateNamespace(
                *updated,
                other.namespaces.get(updated).unwrap().clone(),
            ));
        }

        // Add new namespaces.
        for added in &namespace_changes.added {
            deltas.push(SubsystemDelta::AddNamespace(
                *added,
                other.namespaces.get(added).unwrap().clone(),
            ));
        }

        // Delete hosts not in other.
        for removed_host in self.allowed_hosts.difference(&other.allowed_hosts) {
            deltas.push(SubsystemDelta::RemoveHost(removed_host.clone()));
        }

        deltas
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn test_state_get_deltas_port() {
        let mut deltas: Vec<StateDelta>;
        let mut base_state = State::default();
        let mut new_state = State::default();

        deltas = base_state.get_deltas(&new_state);
        assert_eq!(deltas.len(), 0);

        new_state
            .ports
            .insert(1, Port::new(PortType::Loop, BTreeSet::new()));
        deltas = base_state.get_deltas(&new_state);
        assert_eq!(deltas.len(), 1);
        assert_eq!(
            deltas[0],
            StateDelta::AddPort(1, Port::new(PortType::Loop, BTreeSet::new()))
        );

        base_state = new_state.clone();
        deltas = base_state.get_deltas(&new_state);
        assert_eq!(deltas.len(), 0);

        new_state.ports.insert(
            1,
            Port::new(
                PortType::Tcp("127.0.0.1:4420".parse().unwrap()),
                BTreeSet::new(),
            ),
        );
        deltas = base_state.get_deltas(&new_state);
        assert_eq!(deltas.len(), 1);
        assert_eq!(
            deltas[0],
            StateDelta::UpdatePort(
                1,
                vec![PortDelta::UpdatePortType(PortType::Tcp(
                    "127.0.0.1:4420".parse().unwrap()
                ))]
            )
        );

        base_state = new_state.clone();
        deltas = base_state.get_deltas(&new_state);
        assert_eq!(deltas.len(), 0);

        new_state.ports.insert(
            1,
            Port::new(
                PortType::Tcp("127.0.0.1:4420".parse().unwrap()),
                BTreeSet::from_iter(vec!["nqn.subsystem".to_string()]),
            ),
        );
        deltas = base_state.get_deltas(&new_state);
        assert_eq!(deltas.len(), 1);
        assert_eq!(
            deltas[0],
            StateDelta::UpdatePort(
                1,
                vec![PortDelta::AddSubsystem("nqn.subsystem".to_string())]
            )
        );

        base_state = new_state.clone();
        deltas = base_state.get_deltas(&new_state);
        assert_eq!(deltas.len(), 0);

        new_state.ports.insert(
            1,
            Port::new(
                PortType::Tcp("127.0.0.1:4420".parse().unwrap()),
                BTreeSet::new(),
            ),
        );
        deltas = base_state.get_deltas(&new_state);
        assert_eq!(deltas.len(), 1);
        assert_eq!(
            deltas[0],
            StateDelta::UpdatePort(
                1,
                vec![PortDelta::RemoveSubsystem("nqn.subsystem".to_string())]
            )
        );

        base_state = new_state.clone();
        new_state.ports.remove(&1);
        deltas = base_state.get_deltas(&new_state);
        assert_eq!(deltas.len(), 1);
        assert_eq!(deltas[0], StateDelta::RemovePort(1));
    }

    #[test]
    fn test_state_get_deltas_subsystem() {
        let mut deltas: Vec<StateDelta>;
        let mut base_state = State::default();
        let mut new_state = State::default();

        deltas = base_state.get_deltas(&new_state);
        assert_eq!(deltas.len(), 0);

        new_state
            .subsystems
            .insert("nqn.test".to_string(), Subsystem::default());
        deltas = base_state.get_deltas(&new_state);
        assert_eq!(deltas.len(), 1);
        assert_eq!(
            deltas[0],
            StateDelta::AddSubsystem("nqn.test".to_string(), Subsystem::default()),
        );

        base_state = new_state.clone();
        deltas = base_state.get_deltas(&new_state);
        assert_eq!(deltas.len(), 0);

        let mut testsub = Subsystem::default();
        testsub.allowed_hosts.insert("nqn.initiator".to_string());
        new_state
            .subsystems
            .insert("nqn.test".to_string(), testsub.clone());
        deltas = base_state.get_deltas(&new_state);
        assert_eq!(deltas.len(), 1);
        assert_eq!(
            deltas[0],
            StateDelta::UpdateSubsystem(
                "nqn.test".to_string(),
                vec![SubsystemDelta::AddHost("nqn.initiator".to_string())]
            )
        );

        base_state = new_state.clone();
        let testsub = Subsystem::default();
        new_state
            .subsystems
            .insert("nqn.test".to_string(), testsub.clone());
        deltas = base_state.get_deltas(&new_state);
        assert_eq!(deltas.len(), 1);
        assert_eq!(
            deltas[0],
            StateDelta::UpdateSubsystem(
                "nqn.test".to_string(),
                vec![SubsystemDelta::RemoveHost("nqn.initiator".to_string())]
            )
        );

        base_state = new_state.clone();
        new_state.subsystems.remove("nqn.test");
        deltas = base_state.get_deltas(&new_state);
        assert_eq!(deltas.len(), 1);
        assert_eq!(
            deltas[0],
            StateDelta::RemoveSubsystem("nqn.test".to_string())
        );
    }

    #[test]
    fn test_subsystem_get_deltas_hosts() {
        let mut deltas: Vec<SubsystemDelta>;
        let mut base_state = Subsystem::default();
        let mut new_state = Subsystem::default();

        deltas = base_state.get_deltas(&new_state);
        assert_eq!(deltas.len(), 0);

        new_state.allowed_hosts.insert("nqn.test1".to_string());
        deltas = base_state.get_deltas(&new_state);
        assert_eq!(deltas.len(), 1);
        assert_eq!(deltas[0], SubsystemDelta::AddHost("nqn.test1".to_string()));

        base_state = new_state.clone();
        deltas = base_state.get_deltas(&new_state);
        assert_eq!(deltas.len(), 0);

        new_state.allowed_hosts.remove("nqn.test1");
        deltas = base_state.get_deltas(&new_state);
        assert_eq!(deltas.len(), 1);
        assert_eq!(
            deltas[0],
            SubsystemDelta::RemoveHost("nqn.test1".to_string())
        );

        base_state = new_state.clone();
        deltas = base_state.get_deltas(&new_state);
        assert_eq!(deltas.len(), 0);
    }

    #[test]
    fn test_subsystem_get_deltas_model_serial() {
        let mut deltas: Vec<SubsystemDelta>;
        let mut base_state = Subsystem::default();
        let mut new_state = Subsystem::default();

        deltas = base_state.get_deltas(&new_state);
        assert_eq!(deltas.len(), 0);

        new_state.model = Some("inSANe".to_string());
        new_state.serial = Some("1001".to_string());
        deltas = base_state.get_deltas(&new_state);
        assert_eq!(deltas.len(), 2);
        assert_eq!(deltas[0], SubsystemDelta::UpdateModel("inSANe".to_string()));
        assert_eq!(deltas[1], SubsystemDelta::UpdateSerial("1001".to_string()));

        base_state = new_state.clone();
        deltas = base_state.get_deltas(&new_state);
        assert_eq!(deltas.len(), 0);
    }
}
