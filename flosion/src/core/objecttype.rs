#[derive(Clone, Copy, PartialEq, PartialOrd, Ord, Eq, Hash)]
pub struct ObjectType {
    name: &'static str,
}

impl ObjectType {
    pub const fn new(name: &'static str) -> ObjectType {
        ObjectType { name }
    }

    pub fn name(&self) -> &'static str {
        self.name
    }
}

// NOTE: this constant is NOT stored in the sound processor traits themselves
// because doing so would make them not object safe.
pub trait WithObjectType {
    const TYPE: ObjectType;
}
