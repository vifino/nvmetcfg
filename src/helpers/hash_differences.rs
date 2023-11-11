use std::collections::{BTreeMap, BTreeSet};

#[derive(Default)]
pub struct BTreeSetDelta<K> {
    pub same: BTreeSet<K>,
    pub removed: BTreeSet<K>,
    pub added: BTreeSet<K>,
}

#[must_use]
pub fn get_btreeset_differences<K>(base: &BTreeSet<K>, new: &BTreeSet<K>) -> BTreeSetDelta<K>
where
    K: Eq + std::hash::Hash + Clone + Ord + Default,
{
    let mut delta = BTreeSetDelta::default();
    for base_key in base {
        if new.contains(base_key) {
            delta.same.insert(base_key.clone());
        } else {
            delta.removed.insert(base_key.clone());
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
pub struct BTreeMapDelta<K> {
    pub same: BTreeSet<K>,
    pub removed: BTreeSet<K>,
    pub changed: BTreeSet<K>,
    pub added: BTreeSet<K>,
}

#[must_use]
pub fn get_btreemap_differences<K, V>(
    base: &BTreeMap<K, V>,
    new: &BTreeMap<K, V>,
) -> BTreeMapDelta<K>
where
    V: Eq,
    K: Eq + std::hash::Hash + Ord + Clone + Default,
{
    let mut delta = BTreeMapDelta::default();
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
    fn test_get_btreemap_differences() {
        let mut base = BTreeMap::new();
        let mut new = BTreeMap::new();

        base.insert(1, "Hello");
        new.insert(1, "Hello");
        base.insert(2, "World!");
        new.insert(2, "cruel");
        new.insert(3, "World!");

        let delta = get_btreemap_differences(&base, &new);
        assert!(delta.same.contains(&1));
        assert!(delta.changed.contains(&2));
        assert!(delta.added.contains(&3));
        assert_eq!(delta.removed.len(), 0);
    }
}
