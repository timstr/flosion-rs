use std::cell::RefCell;

use crate::{
    core::{
        engine::soundgraphcompiler::SoundGraphCompiler,
        objecttype::{ObjectType, WithObjectType},
        sound::{
            context::{Context, LocalArrayList},
            expressionargument::SoundExpressionArgumentHandle,
            soundinputtypes::{KeyReuse, KeyedInputQueue, KeyedInputQueueNode},
            soundprocessor::{StateAndTiming, StaticSoundProcessor},
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

#[derive(Clone, Copy)]
enum KeyboardCommand {
    StartKey { id: KeyId, frequency: f32 },
    ReleaseKey { id: KeyId },
    ReleaseAllKeys,
}

// TODO: remove 'Default from spmcq, allow uninit
impl Default for KeyboardCommand {
    fn default() -> Self {
        KeyboardCommand::ReleaseAllKeys
    }
}

pub struct Keyboard {
    pub input: KeyedInputQueue<KeyboardKeyState>,
    pub key_frequency: SoundExpressionArgumentHandle,

    command_reader: spmcq::Reader<KeyboardCommand>,
    command_writer: RefCell<spmcq::Writer<KeyboardCommand>>,
}

impl Keyboard {
    pub fn start_key(&self, id: KeyId, frequency: f32) {
        self.command_writer
            .borrow_mut()
            .write(KeyboardCommand::StartKey { id, frequency });
    }

    pub fn release_key(&self, id: KeyId) {
        self.command_writer
            .borrow_mut()
            .write(KeyboardCommand::ReleaseKey { id });
    }

    pub fn release_all_keys(&self) {
        self.command_writer
            .borrow_mut()
            .write(KeyboardCommand::ReleaseAllKeys);
    }
}

pub struct KeyboardState {
    command_reader: spmcq::Reader<KeyboardCommand>,
}

impl State for KeyboardState {
    fn start_over(&mut self) {
        // ???
    }
}

impl StaticSoundProcessor for Keyboard {
    type SoundInputType = KeyedInputQueue<KeyboardKeyState>;

    type Expressions<'ctx> = ();

    type StateType = KeyboardState;

    fn new(mut tools: SoundProcessorTools, _args: &ParsedArguments) -> Result<Self, ()> {
        let message_queue_size = 16; // idk
        let input_queue_size = 8; // idk
        let (command_reader, command_writer) = spmcq::ring_buffer(message_queue_size);
        let input = KeyedInputQueue::new(input_queue_size, &mut tools);
        let key_frequency = tools.add_input_scalar_argument(input.id(), |state| {
            state.downcast_if::<KeyboardKeyState>().unwrap().frequency
        });
        Ok(Keyboard {
            input,
            key_frequency,
            command_writer: RefCell::new(command_writer),
            command_reader: command_reader,
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
        KeyboardState {
            command_reader: self.command_reader.clone(),
        }
    }

    fn process_audio<'ctx>(
        state: &mut StateAndTiming<Self::StateType>,
        sound_input_node: &mut KeyedInputQueueNode<KeyboardKeyState>,
        _expressions: &mut (),
        dst: &mut SoundChunk,
        context: Context,
    ) {
        let reuse = KeyReuse::StopOldStartNew;
        while let Some(msg) = state.command_reader.read().value() {
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

        sound_input_node.step(state, dst, &context, LocalArrayList::new());
    }
}

impl WithObjectType for Keyboard {
    const TYPE: ObjectType = ObjectType::new("keyboard");
}
