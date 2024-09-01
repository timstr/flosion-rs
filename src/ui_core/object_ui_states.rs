use std::any::{type_name, Any};

use chive::ChiveIn;

pub trait AnyObjectUiState {
    fn as_any(&self) -> &dyn Any;
    fn as_mut_any(&mut self) -> &mut dyn Any;
    fn get_language_type_name(&self) -> &'static str;
    fn serialize(&self, chive_in: &mut ChiveIn);
}

impl<T: 'static> AnyObjectUiState for T {
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
        todo!()
        // chive_in.chivable(self);
    }
}
