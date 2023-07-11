use std::any::{type_name, Any};

use crate::core::serialization::{Serializable, Serializer};

pub trait AnyObjectUiState: 'static {
    fn as_any(&self) -> &dyn Any;
    fn as_mut_any(&mut self) -> &mut dyn Any;
    fn get_language_type_name(&self) -> &'static str;
    fn serialize(&self, serializer: &mut Serializer);
}

impl<T: 'static + Serializable> AnyObjectUiState for T {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_mut_any(&mut self) -> &mut dyn Any {
        self
    }

    fn get_language_type_name(&self) -> &'static str {
        type_name::<T>()
    }

    fn serialize(&self, serializer: &mut Serializer) {
        serializer.object(self);
    }
}
