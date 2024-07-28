use std::{
    cell::Cell,
    collections::HashMap,
    hash::{Hash, Hasher},
    ops::{BitXor, Deref, DerefMut},
};

use crate::core::uniqueid::UniqueId;

/// RevisionHash is an integer summary of the contents of a data structure,
/// based on hashing, intended to be used in distinguishing whether data
/// structures have changed or not.
#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub(crate) struct RevisionHash(u64);

impl RevisionHash {
    /// Create a new RevisionHash with the given integer value
    pub(crate) fn new(value: u64) -> RevisionHash {
        RevisionHash(value)
    }

    /// Get the integer value of the RevisionHash
    pub(crate) fn value(&self) -> u64 {
        self.0
    }
}

/// Revisable is a trait for types for which a RevisionHash can be computed.
/// Something that implements Revisable can have changes to its contents
/// tracked by watching its RevisionHash alone.
pub(crate) trait Revisable {
    /// Compute the RevisionHash of the object's contents. This should hash
    /// together everything that is relevant to the meaning of the object's
    /// contents and should be a pure function, i.e. it should produce the
    /// exact same result if the object is unchanged or has been changed to
    /// something which is semantically identical.
    fn get_revision(&self) -> RevisionHash;
}

/// Blanket implementation for UniqueId
impl<T: UniqueId> Revisable for T {
    fn get_revision(&self) -> RevisionHash {
        RevisionHash::new(self.value() as u64)
    }
}

/// Revised is a wrapper struct for efficiently tracking the RevisionHash of
/// a desired type T. The RevisionHash is computed lazily and is only
/// recomputed when the object has been accessed mutably. This is achieved
/// transparently using the Deref and DerefMut traits such that Revised<T>
/// behaves in code just like a plain old T, except that computing its
/// RevisionHash is optimized to avoid redundant recursions through all its
/// contents to compute hash values.
#[derive(Clone)]
pub(crate) struct Revised<T> {
    /// The stored object
    value: T,

    /// The revision hash of the stored object, if its up to date
    revision: Cell<Option<RevisionHash>>,
}

impl<T: Revisable> Revised<T> {
    /// Construct a new Revised object containing the given object
    pub(crate) fn new(value: T) -> Revised<T> {
        Revised {
            value,
            revision: Cell::new(None),
        }
    }

    /// Get the contained object's RevisionHash. If the object is
    /// not mutated, this will compute the RevisionHash only once
    /// and cache it for reuse.
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

/// Revised<T> can deref to &T
impl<T: Revisable> Deref for Revised<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

/// Revised<T> can deref to &mut T
impl<T: Revisable> DerefMut for Revised<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.revision.set(None);
        &mut self.value
    }
}

/// Revised<T> is Revisable (obviously?)
impl<T: Revisable> Revisable for Revised<T> {
    fn get_revision(&self) -> RevisionHash {
        Revised::get_revision(&self)
    }
}

/// [T] where T is Revisable is also Revisable
impl<T> Revisable for [T]
where
    T: Revisable,
{
    fn get_revision(&self) -> RevisionHash {
        let mut hasher = seahash::SeaHasher::new();

        // Hash the length first
        hasher.write_usize(self.len());

        // Hash the individual items
        for item in self {
            hasher.write_u64(item.get_revision().value());
        }

        RevisionHash::new(hasher.finish())
    }
}

/// HashMap<K, T> where K and T are both Revisable is also Revisable
impl<K, T> Revisable for HashMap<K, T>
where
    K: Revisable,
    T: Revisable,
{
    fn get_revision(&self) -> RevisionHash {
        // Get an order-independent hash of all items
        let mut items_hash: u64 = 0;
        for (key, value) in self {
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

        // Hash the length first
        hasher.write_usize(self.len());

        // Add the hash value of all items
        hasher.write_u64(items_hash);

        RevisionHash::new(hasher.finish())
    }
}

/// RevisedHashMap<K, T> is shorthand for HashMap<K, Revised<T>>.
/// K and T should be Revisable (but generic type aliases do not
/// enforce constraints currently)
pub(crate) type RevisedHashMap<K, T> = HashMap<K, Revised<T>>;
