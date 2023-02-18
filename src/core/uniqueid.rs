use std::hash::Hash;

pub trait UniqueId: Default + Copy + Clone + PartialEq + Eq + Hash {
    fn value(&self) -> usize;
    fn next(&self) -> Self;
}

pub(crate) struct IdGenerator<T: UniqueId> {
    current_id: T,
}

impl<T: UniqueId> IdGenerator<T> {
    pub(crate) fn new() -> IdGenerator<T> {
        IdGenerator {
            current_id: T::default(),
        }
    }

    pub(crate) fn next_id(&mut self) -> T {
        let ret = self.current_id;
        self.current_id = self.current_id.next();
        ret
    }
}
