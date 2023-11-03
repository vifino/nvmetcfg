use super::types::*;
use std::collections::{HashMap, HashSet};

#[derive(Default)]
pub struct HashMapDelta<K> {
    same: HashSet<K>,
    removed: HashSet<K>,
    changed: HashSet<K>,
    added: HashSet<K>,
}
pub fn get_hashmap_differences<K, V>(base: &HashMap<K, V>, new: &HashMap<K, V>) -> HashMapDelta<K>
where
    V: Eq,
    K: Eq + std::hash::Hash + Clone + Default,
{
    let mut delta = HashMapDelta::default();
    for base_key in base.keys() {
        if !new.contains_key(base_key) {
            delta.removed.insert(base_key.clone());
        } else if base.get(base_key) == new.get(base_key) {
            delta.same.insert(base_key.clone());
        } else {
            delta.changed.insert(base_key.clone());
        }
    }

    for new_key in new.keys() {
        if !base.contains_key(new_key) {
            delta.added.insert(new_key.clone());
        }
    }
    delta
}

// Define the representation of differences to the state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StateDelta {
    AddPort(u32, Port),
    UpdatePort(u32, Port),
    DelPort(u32),

    AddSubsystem(String, Subsystem),
    UpdateSubsystem(String, Subsystem),
    DelSubsystem(String),
}

impl State {
    pub fn get_deltas(&self, other: &Self) -> Vec<StateDelta> {
        let mut deltas = Vec::new();

        let port_changes = get_hashmap_differences(&self.ports, &other.ports);
        let subsystem_changes = get_hashmap_differences(&self.subsystems, &other.subsystems);

        // Delete Ports not in new.
        for removed in port_changes.removed.iter() {
            deltas.push(StateDelta::DelPort(*removed));
        }

        // Delete Subsystems not in new.
        for removed in subsystem_changes.removed.iter() {
            deltas.push(StateDelta::DelSubsystem(removed.to_string()));
        }

        // Update Subsystems
        for updated in subsystem_changes.changed.iter() {
            deltas.push(StateDelta::UpdateSubsystem(
                updated.to_string(),
                other.subsystems.get(updated).unwrap().clone(),
            ));
        }

        // Update Ports.
        for updated in port_changes.changed.iter() {
            deltas.push(StateDelta::UpdatePort(
                *updated,
                other.ports.get(&updated).unwrap().clone(),
            ));
        }

        // Add Subsystems not in base.
        for added in subsystem_changes.added.iter() {
            deltas.push(StateDelta::AddSubsystem(
                added.to_string(),
                other.subsystems.get(added).unwrap().clone(),
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
pub enum SubsystemDelta {
    UpdateModel(String),
    UpdateSerial(String),

    AddHost(String),
    DelHost(String),

    AddNamespace(u32, Namespace),
    UpdateNamespace(u32, Namespace),
    DelNamespace(u32),
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
            deltas.push(SubsystemDelta::DelNamespace(*removed));
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
            deltas.push(SubsystemDelta::DelHost(removed_host.clone()));
        }

        deltas
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_hashmap_differences() {
        let mut base = HashMap::new();
        let mut new = HashMap::new();

        base.insert(1, "Hello");
        new.insert(1, "Hello");
        base.insert(2, "World!");
        new.insert(2, "cruel");
        new.insert(3, "World!");

        let delta = get_hashmap_differences(&base, &new);
        assert!(delta.same.contains(&1));
        assert!(delta.changed.contains(&2));
        assert!(delta.added.contains(&3));
        assert_eq!(delta.removed.len(), 0);
    }

    #[test]
    fn test_state_get_deltas_port() {
        let mut deltas: Vec<StateDelta>;
        let mut base_state = State::default();
        let mut new_state = State::default();

        deltas = base_state.get_deltas(&new_state);
        assert_eq!(deltas.len(), 0);

        new_state.ports.insert(1, Port::new(PortType::Loop, vec![]));
        deltas = base_state.get_deltas(&new_state);
        assert_eq!(deltas.len(), 1);
        assert_eq!(
            deltas[0],
            StateDelta::AddPort(1, Port::new(PortType::Loop, vec![]))
        );

        base_state = new_state.clone();
        deltas = base_state.get_deltas(&new_state);
        assert_eq!(deltas.len(), 0);

        new_state.ports.insert(
            1,
            Port::new(PortType::Tcp("127.0.0.1:4420".parse().unwrap()), vec![]),
        );
        deltas = base_state.get_deltas(&new_state);
        assert_eq!(deltas.len(), 1);
        assert_eq!(
            deltas[0],
            StateDelta::UpdatePort(
                1,
                Port::new(PortType::Tcp("127.0.0.1:4420".parse().unwrap()), vec![])
            )
        );

        base_state = new_state.clone();
        new_state.ports.remove(&1);
        deltas = base_state.get_deltas(&new_state);
        assert_eq!(deltas.len(), 1);
        assert_eq!(deltas[0], StateDelta::DelPort(1));
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
            StateDelta::UpdateSubsystem("testnqn".to_string(), testsub)
        );

        base_state = new_state.clone();
        new_state.subsystems.remove("testnqn");
        deltas = base_state.get_deltas(&new_state);
        assert_eq!(deltas.len(), 1);
        assert_eq!(deltas[0], StateDelta::DelSubsystem("testnqn".to_string()));
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
        assert_eq!(deltas[0], SubsystemDelta::DelHost("testnqn1".to_string()));

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
