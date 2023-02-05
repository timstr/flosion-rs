use std::any::Any;

#[derive(Clone, Copy)]
pub struct AnyData<'a> {
    data: &'a dyn Any,
}

impl<'a> AnyData<'a> {
    pub fn new(data: &'a dyn Any) -> Self {
        Self { data }
    }

    pub fn downcast_if<T: 'static>(&self) -> Option<&T> {
        // TODO: perform an unchecked cast in release mode
        let r = self.data.downcast_ref::<T>();
        debug_assert!(r.is_some());
        Some(r.unwrap())
    }
}
