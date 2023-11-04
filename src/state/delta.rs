use super::types::*;
use crate::helpers::get_hashmap_differences;

// Define the representation of differences to the state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StateDelta {
    AddPort(u32, Port),
    UpdatePort(u32, Vec<PortDelta>),
    RemovePort(u32),

    AddSubsystem(String, Subsystem),
    UpdateSubsystem(String, Vec<SubsystemDelta>),
    RemoveSubsystem(String),
}

impl State {
    pub fn get_deltas(&self, other: &Self) -> Vec<StateDelta> {
        let mut deltas = Vec::new();

        let port_changes = get_hashmap_differences(&self.ports, &other.ports);
        let subsystem_changes = get_hashmap_differences(&self.subsystems, &other.subsystems);

        // Delete Ports not in new.
        for removed in port_changes.removed.iter() {
            deltas.push(StateDelta::RemovePort(*removed));
        }

        // Delete Subsystems not in new.
        for removed in subsystem_changes.removed.iter() {
            deltas.push(StateDelta::RemoveSubsystem(removed.to_string()));
        }

        // Update Subsystems
        for updated in subsystem_changes.changed.iter() {
            deltas.push(StateDelta::UpdateSubsystem(
                updated.to_string(),
                self.subsystems
                    .get(updated)
                    .unwrap()
                    .get_deltas(other.subsystems.get(updated).unwrap()),
            ));
        }

        // Add Subsystems not in base.
        for added in subsystem_changes.added.iter() {
            deltas.push(StateDelta::AddSubsystem(
                added.to_string(),
                other.subsystems.get(added).unwrap().clone(),
            ));
        }

        // Update Ports.
        for updated in port_changes.changed.iter() {
            deltas.push(StateDelta::UpdatePort(
                *updated,
                self.ports
                    .get(updated)
                    .unwrap()
                    .get_deltas(other.ports.get(&updated).unwrap()),
            ));
        }

        // Add Ports not in base.
        for added in port_changes.added.iter() {
            deltas.push(StateDelta::AddPort(
                *added,
                other.ports.get(&added).unwrap().clone(),
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
    pub fn get_deltas(&self, other: &Self) -> Vec<PortDelta> {
        let mut deltas = Vec::new();

        // Remove subsystems not in self.
        for new_sub in other.subsystems.difference(&self.subsystems) {
            deltas.push(PortDelta::RemoveSubsystem(new_sub.clone()));
        }

        // Updated Port Type.
        if self.port_type != other.port_type {
            deltas.push(PortDelta::UpdatePortType(other.port_type.clone()));
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
    pub fn get_deltas(&self, other: &Self) -> Vec<SubsystemDelta> {
        let mut deltas = Vec::new();

        let namespace_changes = get_hashmap_differences(&self.namespaces, &other.namespaces);

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
        for removed in namespace_changes.removed.iter() {
            deltas.push(SubsystemDelta::RemoveNamespace(*removed));
        }

        // Update namespaces.
        for updated in namespace_changes.changed.iter() {
            deltas.push(SubsystemDelta::UpdateNamespace(
                *updated,
                other.namespaces.get(updated).unwrap().clone(),
            ));
        }

        // Add new namespaces.
        for added in namespace_changes.added.iter() {
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
    use std::collections::{HashSet};

    #[test]
    fn test_state_get_deltas_port() {
        let mut deltas: Vec<StateDelta>;
        let mut base_state = State::default();
        let mut new_state = State::default();

        deltas = base_state.get_deltas(&new_state);
        assert_eq!(deltas.len(), 0);

        new_state
            .ports
            .insert(1, Port::new(PortType::Loop, HashSet::new()));
        deltas = base_state.get_deltas(&new_state);
        assert_eq!(deltas.len(), 1);
        assert_eq!(
            deltas[0],
            StateDelta::AddPort(1, Port::new(PortType::Loop, HashSet::new()))
        );

        base_state = new_state.clone();
        deltas = base_state.get_deltas(&new_state);
        assert_eq!(deltas.len(), 0);

        new_state.ports.insert(
            1,
            Port::new(
                PortType::Tcp("127.0.0.1:4420".parse().unwrap()),
                HashSet::new(),
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
            .insert("testnqn".to_string(), Subsystem::default());
        deltas = base_state.get_deltas(&new_state);
        assert_eq!(deltas.len(), 1);
        assert_eq!(
            deltas[0],
            StateDelta::AddSubsystem("testnqn".to_string(), Subsystem::default()),
        );

        base_state = new_state.clone();
        deltas = base_state.get_deltas(&new_state);
        assert_eq!(deltas.len(), 0);

        let mut testsub = Subsystem::default();
        testsub.allowed_hosts.insert("initiatornqn".to_string());
        new_state
            .subsystems
            .insert("testnqn".to_string(), testsub.clone());
        deltas = base_state.get_deltas(&new_state);
        assert_eq!(deltas.len(), 1);
        assert_eq!(
            deltas[0],
            StateDelta::UpdateSubsystem(
                "testnqn".to_string(),
                vec![SubsystemDelta::AddHost("initiatornqn".to_string())]
            )
        );

        base_state = new_state.clone();
        new_state.subsystems.remove("testnqn");
        deltas = base_state.get_deltas(&new_state);
        assert_eq!(deltas.len(), 1);
        assert_eq!(
            deltas[0],
            StateDelta::RemoveSubsystem("testnqn".to_string())
        );
    }

    #[test]
    fn test_subsystem_get_deltas_hosts() {
        let mut deltas: Vec<SubsystemDelta>;
        let mut base_state = Subsystem::default();
        let mut new_state = Subsystem::default();

        deltas = base_state.get_deltas(&new_state);
        assert_eq!(deltas.len(), 0);

        new_state.allowed_hosts.insert("testnqn1".to_string());
        deltas = base_state.get_deltas(&new_state);
        assert_eq!(deltas.len(), 1);
        assert_eq!(deltas[0], SubsystemDelta::AddHost("testnqn1".to_string()));

        base_state = new_state.clone();
        deltas = base_state.get_deltas(&new_state);
        assert_eq!(deltas.len(), 0);

        new_state.allowed_hosts.remove("testnqn1");
        deltas = base_state.get_deltas(&new_state);
        assert_eq!(deltas.len(), 1);
        assert_eq!(
            deltas[0],
            SubsystemDelta::RemoveHost("testnqn1".to_string())
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
