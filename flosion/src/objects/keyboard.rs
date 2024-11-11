use std::cell::RefCell;

use flosion_macros::ProcessorComponents;
use hashstash::{InplaceUnstasher, Stashable, Stasher, UnstashError, UnstashableInplace};

use crate::{
    core::{
        objecttype::{ObjectType, WithObjectType},
        sound::{
            argument::{ArgumentScope, ProcessorArgument},
            argumenttypes::f32argument::F32Argument,
            context::Context,
            inputtypes::keyedinputqueue::{KeyReuse, KeyedInputQueue},
            soundinput::InputOptions,
            soundprocessor::{
                ProcessorState, SoundProcessor, StartOver, StateMarker, StreamStatus,
            },
        },
        soundchunk::SoundChunk,
        stashing::{StashingContext, UnstashingContext},
    },
    ui_core::arguments::ParsedArguments,
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct KeyId(pub usize);

pub struct KeyboardKeyState {
    frequency: f32,
}

impl StartOver for KeyboardKeyState {
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

pub struct KeyboardState {
    command_reader: spmcq::Reader<KeyboardCommand>,
}

impl ProcessorState for KeyboardState {
    type Processor = Keyboard;

    fn new(processor: &Keyboard) -> Self {
        KeyboardState {
            command_reader: processor.command_reader.clone(),
        }
    }
}

impl StartOver for KeyboardState {
    fn start_over(&mut self) {
        // ???
    }
}

#[derive(ProcessorComponents)]
pub struct Keyboard {
    pub input: KeyedInputQueue<KeyboardKeyState>,
    pub key_frequency: ProcessorArgument<F32Argument>,

    #[not_a_component]
    command_reader: spmcq::Reader<KeyboardCommand>,

    #[not_a_component]
    command_writer: RefCell<spmcq::Writer<KeyboardCommand>>,

    #[state]
    state: StateMarker<KeyboardState>,
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

impl SoundProcessor for Keyboard {
    fn new(_args: &ParsedArguments) -> Keyboard {
        let message_queue_size = 16; // idk
        let input_queue_size = 8; // idk
        let key_frequency = ProcessorArgument::new();
        let (command_reader, command_writer) = spmcq::ring_buffer(message_queue_size);
        let input = KeyedInputQueue::new(
            InputOptions::Synchronous,
            input_queue_size,
            ArgumentScope::new(vec![key_frequency.id()]),
        );
        Keyboard {
            input,
            key_frequency,
            command_writer: RefCell::new(command_writer),
            command_reader: command_reader,
            state: StateMarker::new(),
        }
    }

    fn is_static(&self) -> bool {
        true
    }

    fn process_audio(
        keyboard: &mut Self::CompiledType<'_>,
        dst: &mut SoundChunk,
        context: &mut Context,
    ) -> StreamStatus {
        let reuse = KeyReuse::StopOldStartNew;
        while let Some(msg) = keyboard.state.command_reader.read().value() {
            match msg {
                KeyboardCommand::StartKey { id, frequency } => {
                    keyboard
                        .input
                        .start_key(None, id.0, KeyboardKeyState { frequency }, reuse);
                }
                KeyboardCommand::ReleaseKey { id } => {
                    keyboard.input.release_key(id.0);
                }
                KeyboardCommand::ReleaseAllKeys => {
                    keyboard.input.release_all_keys();
                }
            }
        }

        keyboard.input.step_active_keys(dst, context, |s, ctx| {
            ctx.push(keyboard.key_frequency, s.frequency)
        });

        StreamStatus::Playing
    }
}

impl WithObjectType for Keyboard {
    const TYPE: ObjectType = ObjectType::new("keyboard");
}

impl Stashable<StashingContext> for Keyboard {
    fn stash(&self, stasher: &mut Stasher<StashingContext>) {
        stasher.object(&self.input);
        stasher.object(&self.key_frequency);
    }
}

impl UnstashableInplace<UnstashingContext<'_>> for Keyboard {
    fn unstash_inplace(
        &mut self,
        unstasher: &mut InplaceUnstasher<UnstashingContext<'_>>,
    ) -> Result<(), UnstashError> {
        unstasher.object_inplace(&mut self.input)?;
        unstasher.object_inplace(&mut self.key_frequency)?;
        Ok(())
    }
}
