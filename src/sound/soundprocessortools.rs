use std::sync::mpsc::Sender;

use super::{
    key::Key,
    soundengine::SoundEngineMessage,
    soundinput::{
        InputOptions, KeyedSoundInput, KeyedSoundInputHandle, SingleSoundInput,
        SingleSoundInputHandle, SoundInputId,
    },
    soundprocessor::SoundProcessorId,
    soundstate::SoundState,
    uniqueid::IdGenerator,
};

// TODO: this interface is meant for both constructing AND modifying sound processors.
// This means that half the time, the sound processor is known to the sound graph/engine
// and half the time, it isn't.
// It follows that changes made here can't rely on the sound graph/engine being (synchronously)
// modified.
// The Id of the sound processor will already be known, however. In addition, even if the sound
// processor has yet to be added to the sound engine, it should be safe to send a message to
// the sound engine adding the input to the processor by its id *as long as* that message
// is guaranteed to be received after the message adding the processor itself. This can be done
// simply by sharing the same mpsc Sender, and adding a sound engine message type for adding
// (and removing?) an input to a processor

pub struct SoundProcessorTools<'a> {
    processor_id: SoundProcessorId,
    message_queue: Vec<SoundEngineMessage>,
    input_idgen: &'a mut IdGenerator<SoundInputId>,
}

impl<'a> SoundProcessorTools<'a> {
    pub(super) fn new(
        id: SoundProcessorId,
        input_idgen: &'a mut IdGenerator<SoundInputId>,
    ) -> SoundProcessorTools<'a> {
        SoundProcessorTools {
            processor_id: id,
            message_queue: Vec::new(),
            input_idgen,
        }
    }

    pub fn add_single_input(&mut self, options: InputOptions) -> SingleSoundInputHandle {
        let input_id = self.input_idgen.next_id();
        let (input, handle) = SingleSoundInput::new(input_id, options);
        self.message_queue
            .push(SoundEngineMessage::AddSoundInput(input, self.processor_id));
        handle
    }

    pub fn add_keyed_input<K: Key + 'static, T: SoundState + 'static>(
        &mut self,
        options: InputOptions,
    ) -> KeyedSoundInputHandle<K, T> {
        let input_id = self.input_idgen.next_id();
        let (input, handle) = KeyedSoundInput::<K, T>::new(input_id, options);
        self.message_queue
            .push(SoundEngineMessage::AddSoundInput(input, self.processor_id));
        handle
    }

    pub(super) fn deliver_messages(&mut self, sender: &'a Sender<SoundEngineMessage>) {
        let msgs = std::mem::take(&mut self.message_queue);
        for m in msgs {
            sender.send(m).unwrap();
        }
    }
}
