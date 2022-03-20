use parking_lot::{MappedRwLockReadGuard, RwLock, RwLockReadGuard};

use super::key::Key;

pub struct KeyRange<K: Key> {
    keys: RwLock<Vec<K>>,
}

impl<K: Key> KeyRange<K> {
    pub fn new() -> KeyRange<K> {
        KeyRange {
            keys: RwLock::new(Vec::new()),
        }
    }

    pub fn insert_key(&self, key: K) -> usize {
        let mut keys = self.keys.write();
        let index = keys.iter().position(|k| *k > key).unwrap_or(keys.len());
        keys.insert(index, key);
        index
    }

    pub fn erase_key(&self, index: usize) {
        let mut keys = self.keys.write();
        debug_assert!(index < keys.len());
        keys.remove(index);
    }

    pub fn read_all_keys<'a>(&'a self) -> MappedRwLockReadGuard<'a, [K]> {
        RwLockReadGuard::map(self.keys.read(), |keys| &keys[..])
    }

    pub fn read_key<'a>(&'a self, index: usize) -> MappedRwLockReadGuard<'a, K> {
        RwLockReadGuard::map(self.keys.read(), |keys| &keys[index])
    }
}
