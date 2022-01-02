use std::sync::Arc;

use super::key::Key;

pub struct KeyRange<K: Key> {
    keys: Vec<Arc<K>>,
}

impl<K: Key> KeyRange<K> {
    pub fn new() -> KeyRange<K> {
        KeyRange { keys: Vec::new() }
    }

    pub fn insert_key(&mut self, key: Arc<K>) -> usize {
        assert!(self.keys().iter().find(|k| Arc::ptr_eq(k, &key)).is_none());
        let index = self
            .keys
            .iter()
            .position(|k| *k > key)
            .unwrap_or(self.keys.len());
        self.keys.insert(index, key);
        index
    }

    pub fn erase_key(&mut self, index: usize) {
        assert!(index < self.keys.len());
        self.keys.remove(index);
    }

    pub fn keys(&self) -> &[Arc<K>] {
        &self.keys
    }
}

impl<K: Key> Clone for KeyRange<K> {
    fn clone(&self) -> Self {
        Self {
            keys: self.keys.iter().map(|k| Arc::clone(&k)).collect(),
        }
    }
}
