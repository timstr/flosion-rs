use parking_lot::Mutex;

use crate::{
    core::{
        engine::soundgraphcompiler::SoundGraphCompiler,
        graph::graphobject::{ObjectType, WithObjectType},
        sound::{
            context::{Context, LocalArrayList},
            soundinput::InputOptions,
            soundinputtypes::{SingleInput, SingleInputNode},
            soundprocessor::{
                ProcessorTiming, StateAndTiming, StaticSoundProcessor, StaticSoundProcessorWithId,
            },
            soundprocessortools::SoundProcessorTools,
        },
        soundchunk::SoundChunk,
    },
    ui_core::arguments::ParsedArguments,
};

pub struct Oscilloscope {
    pub input: SingleInput,
    // TODO: AAAH I don't want this Mutex here
    // but it's currently required to be Sync.
    // Sure it's cool to be able to access the
    // sound processor from the GUI but is there
    // a way to do so without slowing down the
    // audio thread? Maybe have some parts of
    // static processors live on the audio thread
    // only, similar to state for dynamic processors?
    chunk_reader: Mutex<spmcq::Reader<SoundChunk>>,
    chunk_writer: Mutex<spmcq::Writer<SoundChunk>>,
}

impl Oscilloscope {
    pub fn get_buffer_reader(&self) -> spmcq::Reader<SoundChunk> {
        self.chunk_reader.lock().clone()
    }
}

impl StaticSoundProcessor for Oscilloscope {
    type SoundInputType = SingleInput;

    type Expressions<'ctx> = ();

    type StateType = ();

    fn new(mut tools: SoundProcessorTools, _args: ParsedArguments) -> Result<Self, ()> {
        let (reader, writer) = spmcq::ring_buffer(64);
        Ok(Oscilloscope {
            input: SingleInput::new(InputOptions::Synchronous, &mut tools),
            chunk_reader: Mutex::new(reader),
            chunk_writer: Mutex::new(writer),
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
        processor: &StaticSoundProcessorWithId<Self>,
        state: &mut StateAndTiming<Self::StateType>,
        sound_input: &mut SingleInputNode<'ctx>,
        _expressions: &mut (),
        dst: &mut SoundChunk,
        context: Context,
    ) {
        sound_input.step(state, dst, &context, LocalArrayList::new());
        processor.chunk_writer.lock().write(*dst);
    }
}

impl WithObjectType for Oscilloscope {
    const TYPE: ObjectType = ObjectType::new("oscilloscope");
}
