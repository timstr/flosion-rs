use std::any::Any;

use super::uniqueid::UniqueId;

#[derive(Clone, Copy)]
pub struct AnyData<'a, I: UniqueId> {
    owner_id: I,
    data: &'a dyn Any,
}

impl<'a, I: UniqueId> AnyData<'a, I> {
    pub fn new(owner_id: I, data: &'a dyn Any) -> Self {
        Self { owner_id, data }
    }

    pub fn owner_id(&self) -> I {
        self.owner_id
    }

    pub fn downcast_if<T: 'static>(&self, owner_id: I) -> Option<&T> {
        if owner_id != self.owner_id {
            return None;
        }
        // TODO: perform an unchecked cast in release mode
        let r = self.data.downcast_ref::<T>();
        debug_assert!(r.is_some());
        Some(r.unwrap())
    }
}
