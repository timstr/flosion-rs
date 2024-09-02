use std::sync::mpsc::{sync_channel, Receiver, SyncSender};

use parking_lot::Mutex;

use crate::{
    core::{
        engine::soundgraphcompiler::SoundGraphCompiler,
        graph::graphobject::{ObjectType, WithObjectType},
        sound::{
            context::{Context, LocalArrayList},
            expressionargument::SoundExpressionArgumentHandle,
            soundinputtypes::{KeyReuse, KeyedInputQueue, KeyedInputQueueNode},
            soundprocessor::{ProcessorTiming, StaticSoundProcessor, StaticSoundProcessorWithId},
            soundprocessortools::SoundProcessorTools,
            state::State,
        },
        soundchunk::SoundChunk,
    },
    ui_core::arguments::ParsedArguments,
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct KeyId(pub usize);

pub struct KeyboardKeyState {
    frequency: f32,
}

impl State for KeyboardKeyState {
    fn start_over(&mut self) {
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
    pub key_frequency: SoundExpressionArgumentHandle,
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

    type Expressions<'ctx> = ();

    type StateType = ();

    fn new(mut tools: SoundProcessorTools, _args: ParsedArguments) -> Result<Self, ()> {
        let message_queue_size = 16; // idk
        let input_queue_size = 8; // idk
        let (command_sender, command_receiver) = sync_channel(message_queue_size);
        let input = KeyedInputQueue::new(input_queue_size, &mut tools);
        let key_frequency = tools.add_input_scalar_argument(input.id(), |state| {
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

    fn compile_expressions<'a, 'ctx>(
        &self,
        _compiler: &SoundGraphCompiler<'a, 'ctx>,
    ) -> Self::Expressions<'ctx> {
        ()
    }

    fn make_state(&self) -> Self::StateType {
        ()
    }

    fn process_audio<'ctx>(
        keyboard: &StaticSoundProcessorWithId<Keyboard>,
        timing: &ProcessorTiming,
        sound_input_node: &mut KeyedInputQueueNode<KeyboardKeyState>,
        _expressions: &mut (),
        dst: &mut SoundChunk,
        context: Context,
    ) {
        let receiver = keyboard.command_receiver.lock();
        let reuse = KeyReuse::StopOldStartNew;
        for msg in receiver.try_iter() {
            match msg {
                KeyboardCommand::StartKey { id, frequency } => {
                    sound_input_node.start_key(None, id.0, KeyboardKeyState { frequency }, reuse);
                }
                KeyboardCommand::ReleaseKey { id } => {
                    sound_input_node.release_key(id.0);
                }
                KeyboardCommand::ReleaseAllKeys => {
                    sound_input_node.release_all_keys();
                }
            }
        }

        sound_input_node.step(timing, dst, &context, LocalArrayList::new());
    }
}

impl WithObjectType for Keyboard {
    const TYPE: ObjectType = ObjectType::new("keyboard");
}
