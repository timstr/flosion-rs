use std::sync::mpsc::{sync_channel, Receiver, SyncSender};

use parking_lot::Mutex;

use crate::core::{
    context::Context,
    graphobject::{ObjectInitialization, ObjectType, WithObjectType},
    numbersource::{NumberSourceHandle, NumberVisibility},
    soundchunk::SoundChunk,
    soundinputtypes::{KeyReuse, KeyedInputQueue, KeyedInputQueueNode},
    soundprocessor::StaticSoundProcessor,
    soundprocessortools::SoundProcessorTools,
    state::State,
};

type KeyId = u8;

pub struct KeyboardKeyState {
    frequency: f32,
}

impl State for KeyboardKeyState {
    fn reset(&mut self) {
        self.frequency = 0.0;
    }
}

enum KeyboardCommand {
    StartKey { id: KeyId, frequency: f32 },
    ReleaseKey { id: KeyId },
    ReleaseAllKeys,
}

pub struct Keyboard {
    pub input: KeyedInputQueue<KeyId, KeyboardKeyState>,
    pub key_frequency: NumberSourceHandle,
    command_sender: SyncSender<KeyboardCommand>,
    command_receiver: Mutex<Receiver<KeyboardCommand>>,
}

impl Keyboard {
    pub fn start_key(&self, id: KeyId, frequency: f32) {
        self.command_sender
            .send(KeyboardCommand::StartKey { id, frequency })
            .unwrap();
    }

    pub fn release_key(&self, id: KeyId) {
        self.command_sender
            .send(KeyboardCommand::ReleaseKey { id })
            .unwrap();
    }

    pub fn release_all_keys(&self) {
        self.command_sender
            .send(KeyboardCommand::ReleaseAllKeys)
            .unwrap();
    }
}

impl StaticSoundProcessor for Keyboard {
    type SoundInputType = KeyedInputQueue<KeyId, KeyboardKeyState>;

    type NumberInputType<'ctx> = ();

    fn new(mut tools: SoundProcessorTools, _init: ObjectInitialization) -> Result<Self, ()> {
        let message_queue_size = 16; // idk
        let input_queue_size = 8; // idk
        let (command_sender, command_receiver) = sync_channel(message_queue_size);
        let input = KeyedInputQueue::new(input_queue_size, &mut tools);
        let key_frequency = tools.add_input_scalar_number_source(
            input.id(),
            |state| state.downcast_if::<KeyboardKeyState>().unwrap().frequency,
            NumberVisibility::Public,
        );
        Ok(Keyboard {
            input,
            key_frequency,
            command_sender,
            command_receiver: Mutex::new(command_receiver),
        })
    }

    fn get_sound_input(&self) -> &Self::SoundInputType {
        &self.input
    }

    fn make_number_inputs<'ctx>(
        &self,
        _context: &'ctx inkwell::context::Context,
    ) -> Self::NumberInputType<'ctx> {
        ()
    }

    fn process_audio<'ctx>(
        &self,
        sound_input_node: &mut KeyedInputQueueNode<KeyId, KeyboardKeyState>,
        _number_inputs: &(),
        dst: &mut SoundChunk,
        context: Context,
    ) {
        let receiver = self.command_receiver.lock();
        let reuse = KeyReuse::StopOldStartNew;
        for msg in receiver.try_iter() {
            match msg {
                KeyboardCommand::StartKey { id, frequency } => {
                    sound_input_node.start_key(None, id, KeyboardKeyState { frequency }, reuse);
                }
                KeyboardCommand::ReleaseKey { id } => {
                    sound_input_node.release_key(id);
                }
                KeyboardCommand::ReleaseAllKeys => {
                    sound_input_node.release_all_keys();
                }
            }
        }

        sound_input_node.step(self, dst, &context);
    }
}

impl WithObjectType for Keyboard {
    const TYPE: ObjectType = ObjectType::new("keyboard");
}
