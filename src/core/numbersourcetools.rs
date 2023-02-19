use super::{
    numberinput::{NumberInputHandle, NumberInputId, NumberInputOwner},
    numbersource::NumberSourceId,
    soundgraphdata::NumberInputData,
    soundgraphedit::SoundGraphEdit,
    uniqueid::IdGenerator,
};

pub struct NumberSourceTools<'a> {
    number_source_id: NumberSourceId,
    number_input_idgen: &'a mut IdGenerator<NumberInputId>,
    edit_queue: &'a mut Vec<SoundGraphEdit>,
}

impl<'a> NumberSourceTools<'a> {
    pub(crate) fn new(
        number_source_id: NumberSourceId,
        number_input_idgen: &'a mut IdGenerator<NumberInputId>,
        edit_queue: &'a mut Vec<SoundGraphEdit>,
    ) -> NumberSourceTools<'a> {
        NumberSourceTools {
            number_source_id,
            number_input_idgen,
            edit_queue,
        }
    }

    pub fn add_number_input(&mut self, default_value: f32) -> NumberInputHandle {
        let id = self.number_input_idgen.next_id();
        let target = None;
        let owner = NumberInputOwner::NumberSource(self.number_source_id);
        let data = NumberInputData::new(id, target, owner, default_value);
        self.edit_queue.push(SoundGraphEdit::AddNumberInput(data));
        NumberInputHandle::new(id, owner)
    }

    pub fn remove_number_input(&mut self, handle: NumberInputHandle) {
        self.edit_queue.push(SoundGraphEdit::RemoveNumberInput(
            handle.id(),
            handle.owner(),
        ));
    }
}
