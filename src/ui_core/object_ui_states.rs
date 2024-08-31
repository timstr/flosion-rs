use std::any::{type_name, Any};

use chive::{Chivable, ChiveIn};

pub trait AnyObjectUiState: 'static {
    fn as_any(&self) -> &dyn Any;
    fn as_mut_any(&mut self) -> &mut dyn Any;
    fn get_language_type_name(&self) -> &'static str;
    fn serialize(&self, chive_in: &mut ChiveIn);
}

// TODO: requiring Chivable here fails to distinguish
// between ui states that should entirely be written
// to disk, and ui states that are used just for
// book-keeping and caching between ui redraws.
// Remove this requirement and clean this up.
impl<T: 'static + Chivable> AnyObjectUiState for T {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_mut_any(&mut self) -> &mut dyn Any {
        self
    }

    fn get_language_type_name(&self) -> &'static str {
        type_name::<T>()
    }

    fn serialize(&self, chive_in: &mut ChiveIn) {
        chive_in.chivable(self);
    }
}
