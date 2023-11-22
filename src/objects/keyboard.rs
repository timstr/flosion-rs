use std::sync::mpsc::{sync_channel, Receiver, SyncSender};

use parking_lot::Mutex;

use crate::core::{
    engine::nodegen::NodeGen,
    graph::graphobject::{ObjectInitialization, ObjectType, WithObjectType},
    sound::{
        context::Context,
        soundinputtypes::{KeyReuse, KeyedInputQueue, KeyedInputQueueNode},
        soundnumbersource::SoundNumberSourceHandle,
        soundprocessor::{ProcessorTiming, StaticSoundProcessor, StaticSoundProcessorWithId},
        soundprocessortools::SoundProcessorTools,
        state::State,
    },
    soundchunk::SoundChunk,
};

type KeyId = usize;

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
    pub input: KeyedInputQueue<KeyboardKeyState>,
    pub key_frequency: SoundNumberSourceHandle,
    command_sender: SyncSender<KeyboardCommand>,
    command_receiver: Mutex<Receiver<KeyboardCommand>>,
}

impl Keyboard {
    pub fn start_key(&self, id: KeyId, frequency: f32) {
        self.command_sender
            .try_send(KeyboardCommand::StartKey { id, frequency })
            .unwrap();
    }

    pub fn release_key(&self, id: KeyId) {
        self.command_sender
            .try_send(KeyboardCommand::ReleaseKey { id })
            .unwrap();
    }

    pub fn release_all_keys(&self) {
        self.command_sender
            .try_send(KeyboardCommand::ReleaseAllKeys)
            .unwrap();
    }
}

impl StaticSoundProcessor for Keyboard {
    type SoundInputType = KeyedInputQueue<KeyboardKeyState>;

    type NumberInputType<'ctx> = ();

    fn new(mut tools: SoundProcessorTools, _init: ObjectInitialization) -> Result<Self, ()> {
        let message_queue_size = 16; // idk
        let input_queue_size = 8; // idk
        let (command_sender, command_receiver) = sync_channel(message_queue_size);
        let input = KeyedInputQueue::new(input_queue_size, &mut tools);
        let key_frequency = tools.add_input_scalar_number_source(input.id(), |state| {
            state.downcast_if::<KeyboardKeyState>().unwrap().frequency
        });
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

    fn make_number_inputs<'a, 'ctx>(
        &self,
        _nodegen: &NodeGen<'a, 'ctx>,
    ) -> Self::NumberInputType<'ctx> {
        ()
    }

    fn process_audio<'ctx>(
        keyboard: &StaticSoundProcessorWithId<Keyboard>,
        timing: &ProcessorTiming,
        sound_input_node: &mut KeyedInputQueueNode<KeyboardKeyState>,
        _number_inputs: &mut (),
        dst: &mut SoundChunk,
        context: Context,
    ) {
        let receiver = keyboard.command_receiver.lock();
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

        sound_input_node.step(timing, dst, &context);
    }
}

impl WithObjectType for Keyboard {
    const TYPE: ObjectType = ObjectType::new("keyboard");
}
