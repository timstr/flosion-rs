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
        soundinput::InputOptions,
        soundinputtypes::{SingleInput, SingleInputNode},
        soundnumberinput::SoundNumberInputHandle,
        soundnumbersource::{SoundNumberSourceHandle, SoundNumberSourceId},
        soundprocessor::{DynamicSoundProcessor, StateAndTiming, StreamStatus},
        soundprocessortools::SoundProcessorTools,
    },
    soundchunk::SoundChunk,
};

pub struct ReadWriteWaveform {
    pub sound_input: SingleInput,
    // TODO: multiple outputs to enable stereo
    pub waveform: SoundNumberInputHandle,
    pub input_l: SoundNumberSourceHandle,
    pub input_r: SoundNumberSourceHandle,
}

pub struct ReadWriteWaveformNumberInputs<'ctx> {
    waveform: SoundNumberInputNode<'ctx>,
    input_l: SoundNumberSourceId,
    input_r: SoundNumberSourceId,
}

impl<'ctx> SoundNumberInputNodeCollection<'ctx> for ReadWriteWaveformNumberInputs<'ctx> {
    fn visit_number_inputs(&self, visitor: &mut dyn SoundNumberInputNodeVisitor<'ctx>) {
        visitor.visit_node(&self.waveform);
    }

    fn visit_number_inputs_mut(&mut self, visitor: &mut dyn SoundNumberInputNodeVisitorMut<'ctx>) {
        visitor.visit_node(&mut self.waveform);
    }
}

impl DynamicSoundProcessor for ReadWriteWaveform {
    type StateType = ();
    type SoundInputType = SingleInput;
    type NumberInputType<'ctx> = ReadWriteWaveformNumberInputs<'ctx>;

    fn new(mut tools: SoundProcessorTools, _init: ObjectInitialization) -> Result<Self, ()> {
        let input_l = tools.add_local_array_number_source();
        let input_r = tools.add_local_array_number_source();
        let waveform_scope = SoundNumberInputScope::with_processor_state()
            .add_local(input_l.id())
            .add_local(input_r.id());
        Ok(ReadWriteWaveform {
            sound_input: SingleInput::new(InputOptions::Synchronous, &mut tools),
            waveform: tools.add_number_input(0.0, waveform_scope),
            input_l,
            input_r,
        })
    }

    fn get_sound_input(&self) -> &Self::SoundInputType {
        &self.sound_input
    }

    fn make_state(&self) -> Self::StateType {
        ()
    }

    fn make_number_inputs<'a, 'ctx>(
        &self,
        nodegen: &NodeGen<'a, 'ctx>,
    ) -> Self::NumberInputType<'ctx> {
        ReadWriteWaveformNumberInputs {
            waveform: self.waveform.make_node(nodegen),
            input_l: self.input_l.id(),
            input_r: self.input_r.id(),
        }
    }

    fn process_audio(
        state: &mut StateAndTiming<()>,
        sound_input: &mut SingleInputNode,
        number_inputs: &mut ReadWriteWaveformNumberInputs,
        dst: &mut SoundChunk,
        context: Context,
    ) -> StreamStatus {
        let mut tmp = SoundChunk::new();
        sound_input.step(state, &mut tmp, &context, LocalArrayList::new());
        number_inputs.waveform.eval(
            &mut dst.l,
            Discretization::samplewise_temporal(),
            &context.push_processor_state(
                state,
                LocalArrayList::new()
                    .push(&tmp.l, number_inputs.input_l)
                    .push(&tmp.r, number_inputs.input_r),
            ),
        );
        slicemath::copy(&dst.l, &mut dst.r);

        StreamStatus::Playing
    }
}

impl WithObjectType for ReadWriteWaveform {
    const TYPE: ObjectType = ObjectType::new("readwritewaveform");
}
