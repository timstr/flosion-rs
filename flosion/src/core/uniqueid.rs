use std::{fmt::Debug, hash::Hash, marker::PhantomData};

use hashstash::{Stashable, Stasher, UnstashError, Unstashable, Unstasher};
use rand::{thread_rng, Rng};

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

impl<T> Stashable for UniqueId<T> {
    fn stash(&self, stasher: &mut Stasher) {
        stasher.u64(self.value as _);
    }
}

impl<T> Unstashable for UniqueId<T> {
    fn unstash(unstasher: &mut Unstasher) -> Result<Self, UnstashError> {
        Ok(UniqueId::new(unstasher.u64()? as _))
    }
}

impl<T> UniqueId<T> {
    pub fn new_unique() -> Self {
        Self {
            value: thread_rng().gen(),
            phantom_data: PhantomData,
        }
    }

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
