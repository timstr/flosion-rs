use super::{numberinput::NumberInputId, uniqueid::IdGenerator};

pub struct NumberSourceTools<'a> {
    number_input_idgen: &'a mut IdGenerator<NumberInputId>,
}
