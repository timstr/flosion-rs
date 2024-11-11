use std::sync::Arc;

use flosion_macros::ProcessorComponents;
use hashstash::{InplaceUnstasher, Stashable, Stasher, UnstashError, UnstashableInplace};
use parking_lot::Mutex;

use crate::{
    core::{
        objecttype::{ObjectType, WithObjectType},
        sound::{
            argument::ArgumentScope,
            context::AudioContext,
            inputtypes::singleinput::SingleInput,
            soundinput::{InputContext, InputOptions},
            soundprocessor::{
                ProcessorState, SoundProcessor, StartOver, StateMarker, StreamStatus,
            },
        },
        soundchunk::SoundChunk,
        stashing::{StashingContext, UnstashingContext},
    },
    ui_core::arguments::ParsedArguments,
};

#[derive(ProcessorComponents)]
pub struct Oscilloscope {
    pub input: SingleInput,

    #[not_a_component]
    chunk_reader: spmcq::Reader<SoundChunk>,
    // NOTE: using Arc<Mutex<...>> because spmcq::Writer can't be cloned.
    // It might be worth using a different queue or somehow guaranteeing
    // at the type system level that only once instance of a static processor's
    // state exists at one time
    #[not_a_component]
    chunk_writer: Arc<Mutex<spmcq::Writer<SoundChunk>>>,

    #[state]
    state: StateMarker<OscilloscopeState>,
}

impl Oscilloscope {
    pub fn get_buffer_reader(&self) -> spmcq::Reader<SoundChunk> {
        self.chunk_reader.clone()
    }
}

pub struct OscilloscopeState {
    chunk_writer: Arc<Mutex<spmcq::Writer<SoundChunk>>>,
}

impl ProcessorState for OscilloscopeState {
    type Processor = Oscilloscope;

    fn new(processor: &Self::Processor) -> Self {
        OscilloscopeState {
            chunk_writer: Arc::clone(&processor.chunk_writer),
        }
    }
}

impl StartOver for OscilloscopeState {
    fn start_over(&mut self) {
        // ???
    }
}

impl SoundProcessor for Oscilloscope {
    fn new(_args: &ParsedArguments) -> Oscilloscope {
        let (reader, writer) = spmcq::ring_buffer(64);
        Oscilloscope {
            input: SingleInput::new(InputOptions::Synchronous, ArgumentScope::new_empty()),
            chunk_reader: reader,
            chunk_writer: Arc::new(Mutex::new(writer)),
            state: StateMarker::new(),
        }
    }

    fn is_static(&self) -> bool {
        true
    }

    fn process_audio(
        oscilloscope: &mut Self::CompiledType<'_>,
        dst: &mut SoundChunk,
        context: &mut AudioContext,
    ) -> StreamStatus {
        oscilloscope.input.step(dst, InputContext::new(context));
        oscilloscope.state.chunk_writer.lock().write(*dst);
        StreamStatus::Playing
    }
}

impl WithObjectType for Oscilloscope {
    const TYPE: ObjectType = ObjectType::new("oscilloscope");
}

impl Stashable<StashingContext> for Oscilloscope {
    fn stash(&self, stasher: &mut Stasher<StashingContext>) {
        stasher.object(&self.input);
    }
}

impl UnstashableInplace<UnstashingContext<'_>> for Oscilloscope {
    fn unstash_inplace(
        &mut self,
        unstasher: &mut InplaceUnstasher<UnstashingContext>,
    ) -> Result<(), UnstashError> {
        unstasher.object_inplace(&mut self.input)?;
        Ok(())
    }
}
