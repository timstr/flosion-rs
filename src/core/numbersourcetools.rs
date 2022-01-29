use std::sync::mpsc::Sender;

use super::{
    numberinput::{NumberInputHandle, NumberInputId, NumberInputOwner},
    numbersource::NumberSourceId,
    resultfuture::ResultFuture,
    soundengine::SoundEngineMessage,
    uniqueid::IdGenerator,
};

pub struct NumberSourceTools<'a> {
    number_source_id: NumberSourceId,
    message_queue: Vec<SoundEngineMessage>,
    number_input_idgen: &'a mut IdGenerator<NumberInputId>,
}

impl<'a> NumberSourceTools<'a> {
    pub(super) fn new(
        number_source_id: NumberSourceId,
        number_input_idgen: &'a mut IdGenerator<NumberInputId>,
    ) -> NumberSourceTools<'a> {
        NumberSourceTools {
            number_source_id,
            message_queue: Vec::new(),
            number_input_idgen,
        }
    }

    pub fn add_number_input(&mut self) -> (NumberInputHandle, ResultFuture<(), ()>) {
        let input_id = self.number_input_idgen.next_id();
        let (result_future, outbound_result) = ResultFuture::<(), ()>::new();
        let handle = NumberInputHandle::new(
            input_id,
            NumberInputOwner::NumberSource(self.number_source_id),
        );
        self.message_queue.push(SoundEngineMessage::AddNumberInput {
            input: handle.clone(),
            result: outbound_result,
        });
        (handle, result_future)
    }

    pub fn remove_number_input(&mut self, handle: NumberInputHandle) -> ResultFuture<(), ()> {
        let (result_future, outbound_result) = ResultFuture::<(), ()>::new();
        self.message_queue
            .push(SoundEngineMessage::RemoveNumberInput {
                input_id: handle.id(),
                result: outbound_result,
            });
        result_future
    }

    pub(super) fn deliver_messages(&mut self, sender: &'a Sender<SoundEngineMessage>) {
        let msgs = std::mem::take(&mut self.message_queue);
        for m in msgs {
            sender.send(m).unwrap();
        }
    }
}
