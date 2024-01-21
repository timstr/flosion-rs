use crate::core::{
    engine::{
        nodegen::NodeGen,
        soundnumberinputnode::{
            SoundNumberInputNode, SoundNumberInputNodeCollection, SoundNumberInputNodeVisitor,
            SoundNumberInputNodeVisitorMut,
        },
    },
    graph::graphobject::{ObjectInitialization, ObjectType, WithObjectType},
    jit::compilednumberinput::Discretization,
    sound::{
        context::{Context, LocalArrayList},
        soundgraphdata::SoundNumberInputScope,
        soundnumberinput::SoundNumberInputHandle,
        soundprocessor::{DynamicSoundProcessor, StateAndTiming, StreamStatus},
        soundprocessortools::SoundProcessorTools,
    },
    soundchunk::SoundChunk,
};

pub struct WriteWaveform {
    pub waveform: SoundNumberInputHandle,
}

pub struct WriteWaveformNumberInputs<'ctx> {
    waveform: SoundNumberInputNode<'ctx>,
}

impl<'ctx> SoundNumberInputNodeCollection<'ctx> for WriteWaveformNumberInputs<'ctx> {
    fn visit_number_inputs(&self, visitor: &mut dyn SoundNumberInputNodeVisitor<'ctx>) {
        visitor.visit_node(&self.waveform);
    }

    fn visit_number_inputs_mut(&mut self, visitor: &mut dyn SoundNumberInputNodeVisitorMut<'ctx>) {
        visitor.visit_node(&mut self.waveform);
    }
}

impl DynamicSoundProcessor for WriteWaveform {
    type StateType = ();
    type SoundInputType = ();
    type NumberInputType<'ctx> = WriteWaveformNumberInputs<'ctx>;

    fn new(mut tools: SoundProcessorTools, _init: ObjectInitialization) -> Result<Self, ()> {
        Ok(WriteWaveform {
            waveform: tools.add_number_input(0.0, SoundNumberInputScope::with_processor_state()),
        })
    }

    fn get_sound_input(&self) -> &Self::SoundInputType {
        &()
    }

    fn make_state(&self) -> Self::StateType {
        ()
    }

    fn make_number_inputs<'a, 'ctx>(
        &self,
        nodegen: &NodeGen<'a, 'ctx>,
    ) -> Self::NumberInputType<'ctx> {
        WriteWaveformNumberInputs {
            waveform: self.waveform.make_node(nodegen),
        }
    }

    fn process_audio(
        state: &mut StateAndTiming<()>,
        _sound_inputs: &mut (),
        number_inputs: &mut WriteWaveformNumberInputs,
        dst: &mut SoundChunk,
        context: Context,
    ) -> StreamStatus {
        number_inputs.waveform.eval(
            &mut dst.l,
            Discretization::samplewise_temporal(),
            &context.push_processor_state(state, LocalArrayList::new()),
        );
        slicemath::copy(&dst.l, &mut dst.r);

        StreamStatus::Playing
    }
}

impl WithObjectType for WriteWaveform {
    const TYPE: ObjectType = ObjectType::new("writewaveform");
}
