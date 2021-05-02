#[derive(Copy, Clone, Eq, PartialEq, PartialOrd, Hash)]
struct Id<T> {
    id: usize,
}

struct IdGenerator<T> {
    next_id: usize,
}

impl<T> IdGenerator<T> {
    fn new() -> IdGenerator<T> {
        IdGenerator<T> {
            next_id: 0
        }
    }

    fn next(&mut self) -> Id<T> {
        let i = Id<T> {
            id: self.next_id
        };
        self.next_id += 1;
        i
    }
}