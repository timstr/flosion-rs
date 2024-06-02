use std::{
    cell::Cell,
    collections::{
        hash_map::{self},
        HashMap,
    },
    hash::{Hash, Hasher},
    ops::{BitXor, Deref, DerefMut},
};

use crate::core::uniqueid::UniqueId;

// TODO: rename to revision hash
#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub(crate) struct RevisionNumber(u64);

impl RevisionNumber {
    pub(crate) fn new(value: u64) -> RevisionNumber {
        RevisionNumber(value)
    }

    pub(crate) fn value(&self) -> u64 {
        self.0
    }
}

pub(crate) trait Revision {
    fn get_revision(&self) -> RevisionNumber;
}

impl<T: UniqueId> Revision for T {
    fn get_revision(&self) -> RevisionNumber {
        RevisionNumber::new(self.value() as u64)
    }
}

#[derive(Clone)]
pub(crate) struct Versioned<T> {
    value: T,
    revision: Cell<Option<RevisionNumber>>,
}

impl<T: Revision> Versioned<T> {
    pub(crate) fn new(value: T) -> Versioned<T> {
        Versioned {
            value,
            revision: Cell::new(None),
        }
    }

    pub(crate) fn get_revision(&self) -> RevisionNumber {
        match self.revision.get() {
            Some(v) => v,
            None => {
                let v = self.value.get_revision();
                self.revision.set(Some(v));
                v
            }
        }
    }
}

impl<T: Revision> Deref for Versioned<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T: Revision> DerefMut for Versioned<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.revision.set(None);
        &mut self.value
    }
}

impl<T: Revision> Revision for Versioned<T> {
    fn get_revision(&self) -> RevisionNumber {
        Versioned::get_revision(&self)
    }
}

#[derive(Clone)]
pub(crate) struct VersionedHashMap<K, V> {
    map: HashMap<K, Versioned<V>>,
}

impl<K: Hash + Eq + PartialEq, V: Revision> VersionedHashMap<K, V> {
    pub(crate) fn new() -> VersionedHashMap<K, V> {
        VersionedHashMap {
            map: HashMap::new(),
        }
    }

    pub(crate) fn len(&self) -> usize {
        self.map.len()
    }

    pub(crate) fn get(&self, k: &K) -> Option<&Versioned<V>> {
        self.map.get(k)
    }

    pub(crate) fn get_mut(&mut self, k: &K) -> Option<&mut Versioned<V>> {
        self.map.get_mut(k)
    }

    pub(crate) fn contains_key(&self, k: &K) -> bool {
        self.map.contains_key(k)
    }

    pub(crate) fn insert(&mut self, k: K, v: V) -> Option<Versioned<V>> {
        self.map.insert(k, Versioned::new(v))
    }

    pub(crate) fn remove(&mut self, k: &K) -> Option<Versioned<V>> {
        self.map.remove(k)
    }

    pub(crate) fn keys(&self) -> hash_map::Keys<K, Versioned<V>> {
        self.map.keys()
    }

    pub(crate) fn values(&self) -> hash_map::Values<K, Versioned<V>> {
        self.map.values()
    }

    pub(crate) fn values_mut(&mut self) -> hash_map::ValuesMut<K, Versioned<V>> {
        self.map.values_mut()
    }
}

impl<'a, K: Hash + Eq + PartialEq, V: Revision> IntoIterator for &'a VersionedHashMap<K, V> {
    type Item = (&'a K, &'a Versioned<V>);

    type IntoIter = hash_map::Iter<'a, K, Versioned<V>>;

    fn into_iter(self) -> Self::IntoIter {
        self.map.iter()
    }
}

impl<'a, K: Hash + Eq + PartialEq, V: Revision> IntoIterator for &'a mut VersionedHashMap<K, V> {
    type Item = (&'a K, &'a mut Versioned<V>);

    type IntoIter = hash_map::IterMut<'a, K, Versioned<V>>;

    fn into_iter(self) -> Self::IntoIter {
        self.map.iter_mut()
    }
}

impl<K: Hash + Eq + PartialEq, V: Revision> IntoIterator for VersionedHashMap<K, V> {
    type Item = (K, Versioned<V>);

    type IntoIter = hash_map::IntoIter<K, Versioned<V>>;

    fn into_iter(self) -> Self::IntoIter {
        self.map.into_iter()
    }
}

impl<K: Revision, V: Revision> Revision for VersionedHashMap<K, V> {
    fn get_revision(&self) -> RevisionNumber {
        let mut items_hash: u64 = 0;
        for (key, value) in &self.map {
            let mut item_hasher = seahash::SeaHasher::new();
            item_hasher.write_u8(0x1);
            item_hasher.write_u64(key.get_revision().value());
            item_hasher.write_u8(0x2);
            item_hasher.write_u64(value.get_revision().value());
            // Use xor to combine hashes of different items so as
            // to not depend on the order of items in the hash map
            items_hash = items_hash.bitxor(item_hasher.finish());
        }
        let mut hasher = seahash::SeaHasher::new();
        hasher.write_usize(self.map.len());
        hasher.write_u64(items_hash);
        RevisionNumber::new(hasher.finish())
    }
}
