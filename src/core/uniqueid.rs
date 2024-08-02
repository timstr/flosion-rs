use std::{
    fmt::Debug,
    hash::{Hash, Hasher},
    marker::PhantomData,
};

use hashrevise::{Revisable, RevisionHash, RevisionHasher};

pub struct UniqueId<T> {
    value: usize,
    phantom_data: PhantomData<T>,
}

// The following traits are explicitly implemented to avoid #[derive(...)] silently
// skipping them if T does not implement them, even though it is only used in PhantomData

impl<T> Copy for UniqueId<T> {}

impl<T> Clone for UniqueId<T> {
    fn clone(&self) -> Self {
        Self {
            value: self.value,
            phantom_data: PhantomData,
        }
    }
}

impl<T> Eq for UniqueId<T> {}

impl<T> PartialEq for UniqueId<T> {
    fn eq(&self, other: &UniqueId<T>) -> bool {
        self.value == other.value
    }
}

impl<T> Hash for UniqueId<T> {
    fn hash<H>(&self, hasher: &mut H)
    where
        H: std::hash::Hasher,
    {
        self.value.hash(hasher);
    }
}

impl<T> Debug for UniqueId<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UniqueId")
            .field("value", &self.value)
            .finish()
    }
}

impl<T> Revisable for UniqueId<T> {
    fn get_revision(&self) -> RevisionHash {
        let mut hasher = RevisionHasher::new();
        hasher.write_usize(self.value());
        hasher.into_revision()
    }
}

impl<T> UniqueId<T> {
    pub const fn new(value: usize) -> Self {
        Self {
            value,
            phantom_data: PhantomData,
        }
    }

    pub fn value(&self) -> usize {
        self.value
    }

    fn next(&self) -> Self {
        Self {
            value: self.value + 1,
            phantom_data: PhantomData,
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct IdGenerator<T> {
    current_id: T,
}

impl<T> IdGenerator<UniqueId<T>> {
    pub(crate) fn new() -> IdGenerator<UniqueId<T>> {
        IdGenerator {
            current_id: UniqueId::new(1),
        }
    }

    pub(crate) fn next_id(&mut self) -> UniqueId<T> {
        let ret = self.current_id;
        self.current_id = self.current_id.next();
        ret
    }
}
