use std::any::Any;

pub trait Key: 'static + Sized + Ord + Sync + Send {}

pub struct TypeErasedKey {
    key: Box<dyn Any + Sync + Send>,
}

impl TypeErasedKey {
    pub(super) fn new<K: Key>(key: K) -> TypeErasedKey {
        TypeErasedKey { key: Box::new(key) }
    }

    pub(super) fn into<K: Key>(self) -> K {
        *self.key.downcast::<K>().unwrap()
    }
}
