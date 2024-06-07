use crate::core::{
    engine::{
        compiledexpressionnode::{
            CompiledExpressionNode, ExpressionCollection, ExpressionVisitor, ExpressionVisitorMut,
        },
        nodegen::NodeGen,
    },
    graph::graphobject::{ObjectInitialization, ObjectType, WithObjectType},
    jit::compiledexpression::Discretization,
    sound::{
        context::{Context, LocalArrayList},
        expression::SoundExpressionHandle,
        expressionargument::{SoundExpressionArgumentHandle, SoundExpressionArgumentId},
        soundgraphdata::SoundExpressionScope,
        soundinput::InputOptions,
        soundinputtypes::{SingleInput, SingleInputNode},
        soundprocessor::{DynamicSoundProcessor, StateAndTiming, StreamStatus},
        soundprocessortools::SoundProcessorTools,
    },
    soundchunk::SoundChunk,
};

pub struct ReadWriteWaveform {
    pub sound_input: SingleInput,
    // TODO: multiple outputs to enable stereo
    pub waveform: SoundExpressionHandle,
    pub input_l: SoundExpressionArgumentHandle,
    pub input_r: SoundExpressionArgumentHandle,
}

pub struct ReadWriteWaveformExpressions<'ctx> {
    waveform: CompiledExpressionNode<'ctx>,
    input_l: SoundExpressionArgumentId,
    input_r: SoundExpressionArgumentId,
}

impl<'ctx> ExpressionCollection<'ctx> for ReadWriteWaveformExpressions<'ctx> {
    fn visit_expressions(&self, visitor: &mut dyn ExpressionVisitor<'ctx>) {
        visitor.visit_node(&self.waveform);
    }

    fn visit_expressions_mut(&mut self, visitor: &mut dyn ExpressionVisitorMut<'ctx>) {
        visitor.visit_node(&mut self.waveform);
    }
}

impl DynamicSoundProcessor for ReadWriteWaveform {
    type StateType = ();
    type SoundInputType = SingleInput;
    type Expressions<'ctx> = ReadWriteWaveformExpressions<'ctx>;

    fn new(mut tools: SoundProcessorTools, _init: ObjectInitialization) -> Result<Self, ()> {
        let input_l = tools.add_local_array_argument();
        let input_r = tools.add_local_array_argument();
        let waveform_scope = SoundExpressionScope::with_processor_state()
            .add_local(input_l.id())
            .add_local(input_r.id());
        Ok(ReadWriteWaveform {
            sound_input: SingleInput::new(InputOptions::Synchronous, &mut tools),
            waveform: tools.add_expression(0.0, waveform_scope),
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

    fn compile_expressions<'a, 'ctx>(
        &self,
        nodegen: &NodeGen<'a, 'ctx>,
    ) -> Self::Expressions<'ctx> {
        ReadWriteWaveformExpressions {
            waveform: self.waveform.make_node(nodegen),
            input_l: self.input_l.id(),
            input_r: self.input_r.id(),
        }
    }

    fn process_audio(
        state: &mut StateAndTiming<()>,
        sound_input: &mut SingleInputNode,
        expressions: &mut ReadWriteWaveformExpressions,
        dst: &mut SoundChunk,
        context: Context,
    ) -> StreamStatus {
        let mut tmp = SoundChunk::new();
        sound_input.step(state, &mut tmp, &context, LocalArrayList::new());
        expressions.waveform.eval(
            &mut dst.l,
            Discretization::samplewise_temporal(),
            &context.push_processor_state(
                state,
                LocalArrayList::new()
                    .push(&tmp.l, expressions.input_l)
                    .push(&tmp.r, expressions.input_r),
            ),
        );
        slicemath::copy(&dst.l, &mut dst.r);

        StreamStatus::Playing
    }
}

impl WithObjectType for ReadWriteWaveform {
    const TYPE: ObjectType = ObjectType::new("readwritewaveform");
}
