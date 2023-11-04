use std::collections::{HashMap, HashSet};

#[derive(Default)]
pub struct HashSetDelta<K> {
    pub same: HashSet<K>,
    pub removed: HashSet<K>,
    pub added: HashSet<K>,
}

pub fn get_hashset_differences<K>(base: &HashSet<K>, new: &HashSet<K>) -> HashSetDelta<K>
where
    K: Eq + std::hash::Hash + Clone + Default,
{
    let mut delta = HashSetDelta::default();
    for base_key in base {
        if !new.contains(base_key) {
            delta.removed.insert(base_key.clone());
        } else {
            delta.same.insert(base_key.clone());
        }
    }

    for new_key in new {
        if !base.contains(new_key) {
            delta.added.insert(new_key.clone());
        }
    }
    delta
}

#[derive(Default)]
pub struct HashMapDelta<K> {
    pub same: HashSet<K>,
    pub removed: HashSet<K>,
    pub changed: HashSet<K>,
    pub added: HashSet<K>,
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
}
