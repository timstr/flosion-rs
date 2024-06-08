use crate::core::{
    engine::{
        nodegen::NodeGen,
        compiledexpression::{
            CompiledExpression, CompiledExpressionCollection, CompiledExpressionVisitor, CompiledExpressionVisitorMut,
        },
    },
    graph::graphobject::{ObjectInitialization, ObjectType, WithObjectType},
    jit::compiledexpression::Discretization,
    sound::{
        context::{Context, LocalArrayList},
        expression::SoundExpressionHandle,
        soundgraphdata::SoundExpressionScope,
        soundprocessor::{DynamicSoundProcessor, StateAndTiming, StreamStatus},
        soundprocessortools::SoundProcessorTools,
    },
    soundchunk::SoundChunk,
};

pub struct WriteWaveform {
    pub waveform: SoundExpressionHandle,
}

pub struct WriteWaveformExpressions<'ctx> {
    waveform: CompiledExpression<'ctx>,
}

impl<'ctx> CompiledExpressionCollection<'ctx> for WriteWaveformExpressions<'ctx> {
    fn visit_expressions(&self, visitor: &mut dyn CompiledExpressionVisitor<'ctx>) {
        visitor.visit_node(&self.waveform);
    }

    fn visit_expressions_mut(&mut self, visitor: &mut dyn CompiledExpressionVisitorMut<'ctx>) {
        visitor.visit_node(&mut self.waveform);
    }
}

impl DynamicSoundProcessor for WriteWaveform {
    type StateType = ();
    type SoundInputType = ();
    type Expressions<'ctx> = WriteWaveformExpressions<'ctx>;

    fn new(mut tools: SoundProcessorTools, _init: ObjectInitialization) -> Result<Self, ()> {
        Ok(WriteWaveform {
            waveform: tools.add_expression(0.0, SoundExpressionScope::with_processor_state()),
        })
    }

    fn get_sound_input(&self) -> &Self::SoundInputType {
        &()
    }

    fn make_state(&self) -> Self::StateType {
        ()
    }

    fn compile_expressions<'a, 'ctx>(
        &self,
        nodegen: &NodeGen<'a, 'ctx>,
    ) -> Self::Expressions<'ctx> {
        WriteWaveformExpressions {
            waveform: self.waveform.make_node(nodegen),
        }
    }

    fn process_audio(
        state: &mut StateAndTiming<()>,
        _sound_inputs: &mut (),
        expressions: &mut WriteWaveformExpressions,
        dst: &mut SoundChunk,
        context: Context,
    ) -> StreamStatus {
        expressions.waveform.eval(
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
