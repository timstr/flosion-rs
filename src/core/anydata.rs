use std::any::Any;

#[derive(Clone, Copy)]
pub struct AnyData<'a> {
    data: &'a dyn Any,
}

// TODO: this is silly. Just store &Any directly in Context and don't even expose it
// except through generic method similar to downcast_if below
impl<'a> AnyData<'a> {
    pub fn new(data: &'a dyn Any) -> Self {
        Self { data }
    }

    pub fn downcast_if<T: 'static>(&self) -> Option<&'a T> {
        // TODO: perform an unchecked cast in release mode
        let r = self.data.downcast_ref::<T>();
        debug_assert!(r.is_some());
        Some(r.unwrap())
    }
}
