use std::{collections::HashMap, hash::Hasher, ops::BitXor};

use super::uniqueid::UniqueId;

pub(crate) trait Revision {
    fn get_revision(&self) -> u64;
}

impl<K, V> Revision for HashMap<K, V>
where
    K: UniqueId,
    V: Revision,
{
    fn get_revision(&self) -> u64 {
        let mut items_hash: u64 = 0;
        for (id, value) in self {
            let mut item_hasher = seahash::SeaHasher::new();
            item_hasher.write_u8(0x1);
            item_hasher.write_usize(id.value());
            item_hasher.write_u8(0x2);
            item_hasher.write_u64(value.get_revision());
            // Use xor to combine hashes of different items so as
            // to not depend on the order of items in the hash map
            items_hash = items_hash.bitxor(item_hasher.finish());
        }
        let mut hasher = seahash::SeaHasher::new();
        hasher.write_usize(self.len());
        hasher.write_u64(items_hash);
        hasher.finish()
    }
}
