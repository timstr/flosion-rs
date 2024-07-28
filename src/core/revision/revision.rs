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

#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub(crate) struct RevisionHash(u64);

impl RevisionHash {
    pub(crate) fn new(value: u64) -> RevisionHash {
        RevisionHash(value)
    }

    pub(crate) fn value(&self) -> u64 {
        self.0
    }
}

pub(crate) trait Revision {
    fn get_revision(&self) -> RevisionHash;
}

impl<T: UniqueId> Revision for T {
    fn get_revision(&self) -> RevisionHash {
        RevisionHash::new(self.value() as u64)
    }
}

#[derive(Clone)]
pub(crate) struct Revised<T> {
    value: T,
    revision: Cell<Option<RevisionHash>>,
}

impl<T: Revision> Revised<T> {
    pub(crate) fn new(value: T) -> Revised<T> {
        Revised {
            value,
            revision: Cell::new(None),
        }
    }

    pub(crate) fn get_revision(&self) -> RevisionHash {
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

impl<T: Revision> Deref for Revised<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T: Revision> DerefMut for Revised<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.revision.set(None);
        &mut self.value
    }
}

impl<T: Revision> Revision for Revised<T> {
    fn get_revision(&self) -> RevisionHash {
        Revised::get_revision(&self)
    }
}

#[derive(Clone)]
pub(crate) struct RevisedHashMap<K, V> {
    map: HashMap<K, Revised<V>>,
}

impl<K: Hash + Eq + PartialEq, V: Revision> RevisedHashMap<K, V> {
    pub(crate) fn new() -> RevisedHashMap<K, V> {
        RevisedHashMap {
            map: HashMap::new(),
        }
    }

    pub(crate) fn len(&self) -> usize {
        self.map.len()
    }

    pub(crate) fn get(&self, k: &K) -> Option<&Revised<V>> {
        self.map.get(k)
    }

    pub(crate) fn get_mut(&mut self, k: &K) -> Option<&mut Revised<V>> {
        self.map.get_mut(k)
    }

    pub(crate) fn contains_key(&self, k: &K) -> bool {
        self.map.contains_key(k)
    }

    pub(crate) fn insert(&mut self, k: K, v: V) -> Option<Revised<V>> {
        self.map.insert(k, Revised::new(v))
    }

    pub(crate) fn remove(&mut self, k: &K) -> Option<Revised<V>> {
        self.map.remove(k)
    }

    pub(crate) fn keys(&self) -> hash_map::Keys<K, Revised<V>> {
        self.map.keys()
    }

    pub(crate) fn values(&self) -> hash_map::Values<K, Revised<V>> {
        self.map.values()
    }

    pub(crate) fn values_mut(&mut self) -> hash_map::ValuesMut<K, Revised<V>> {
        self.map.values_mut()
    }
}

impl<'a, K: Hash + Eq + PartialEq, V: Revision> IntoIterator for &'a RevisedHashMap<K, V> {
    type Item = (&'a K, &'a Revised<V>);

    type IntoIter = hash_map::Iter<'a, K, Revised<V>>;

    fn into_iter(self) -> Self::IntoIter {
        self.map.iter()
    }
}

impl<'a, K: Hash + Eq + PartialEq, V: Revision> IntoIterator for &'a mut RevisedHashMap<K, V> {
    type Item = (&'a K, &'a mut Revised<V>);

    type IntoIter = hash_map::IterMut<'a, K, Revised<V>>;

    fn into_iter(self) -> Self::IntoIter {
        self.map.iter_mut()
    }
}

impl<K: Hash + Eq + PartialEq, V: Revision> IntoIterator for RevisedHashMap<K, V> {
    type Item = (K, Revised<V>);

    type IntoIter = hash_map::IntoIter<K, Revised<V>>;

    fn into_iter(self) -> Self::IntoIter {
        self.map.into_iter()
    }
}

impl<K: Revision, V: Revision> Revision for RevisedHashMap<K, V> {
    fn get_revision(&self) -> RevisionHash {
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
        RevisionHash::new(hasher.finish())
    }
}
