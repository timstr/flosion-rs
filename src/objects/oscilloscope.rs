use parking_lot::Mutex;

use crate::core::{
    engine::nodegen::NodeGen,
    graph::graphobject::{ObjectInitialization, ObjectType, WithObjectType},
    sound::{
        context::{Context, LocalArrayList},
        soundinput::InputOptions,
        soundinputtypes::{SingleInput, SingleInputNode},
        soundprocessor::{ProcessorTiming, StaticSoundProcessor, StaticSoundProcessorWithId},
        soundprocessortools::SoundProcessorTools,
    },
    soundchunk::SoundChunk,
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

    type NumberInputType<'ctx> = ();

    fn new(mut tools: SoundProcessorTools, _init: ObjectInitialization) -> Result<Self, ()> {
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

    fn make_number_inputs<'a, 'ctx>(
        &self,
        _nodegen: &NodeGen<'a, 'ctx>,
    ) -> Self::NumberInputType<'ctx> {
        ()
    }

    fn process_audio<'ctx>(
        processor: &StaticSoundProcessorWithId<Self>,
        timing: &ProcessorTiming,
        sound_input: &mut SingleInputNode<'ctx>,
        _number_inputs: &mut (),
        dst: &mut SoundChunk,
        context: Context,
    ) {
        sound_input.step(timing, dst, &context, LocalArrayList::new());
        processor.chunk_writer.lock().write(*dst);
    }
}

impl WithObjectType for Oscilloscope {
    const TYPE: ObjectType = ObjectType::new("oscilloscope");
}
