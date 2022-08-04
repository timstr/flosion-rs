use std::hash::Hash;

pub trait UniqueId: Default + Copy + Clone + PartialEq + Eq + Hash {
    fn value(&self) -> usize;
    fn next(&self) -> Self;
}

pub struct IdGenerator<T: UniqueId> {
    current_id: T,
}

impl<T: UniqueId> IdGenerator<T> {
    pub fn new() -> IdGenerator<T> {
        IdGenerator {
            current_id: T::default(),
        }
    }

    pub fn next_id(&mut self) -> T {
        let ret = self.current_id;
        self.current_id = self.current_id.next();
        ret
    }
}
