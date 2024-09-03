use crate::{
    core::{
        engine::{
            compiledexpression::{
                CompiledExpression, CompiledExpressionCollection, CompiledExpressionVisitor,
                CompiledExpressionVisitorMut,
            },
            soundgraphcompiler::SoundGraphCompiler,
        },
        graph::graphobject::{ObjectType, WithObjectType},
        jit::compiledexpression::Discretization,
        sound::{
            context::{Context, LocalArrayList},
            expression::SoundExpressionHandle,
            soundgraphdata::SoundExpressionScope,
            soundprocessor::{DynamicSoundProcessor, StateAndTiming, StreamStatus},
            soundprocessortools::SoundProcessorTools,
        },
        soundchunk::SoundChunk,
    },
    ui_core::arguments::ParsedArguments,
};

pub struct WriteWaveform {
    pub waveform: SoundExpressionHandle,
}

pub struct WriteWaveformExpressions<'ctx> {
    waveform: CompiledExpression<'ctx>,
}

impl<'ctx> CompiledExpressionCollection<'ctx> for WriteWaveformExpressions<'ctx> {
    fn visit(&self, visitor: &mut dyn CompiledExpressionVisitor<'ctx>) {
        visitor.visit(&self.waveform);
    }

    fn visit_mut(&mut self, visitor: &mut dyn CompiledExpressionVisitorMut<'ctx>) {
        visitor.visit(&mut self.waveform);
    }
}

impl DynamicSoundProcessor for WriteWaveform {
    type StateType = ();
    type SoundInputType = ();
    type Expressions<'ctx> = WriteWaveformExpressions<'ctx>;

    fn new(mut tools: SoundProcessorTools, _args: &ParsedArguments) -> Result<Self, ()> {
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
        compiler: &SoundGraphCompiler<'a, 'ctx>,
    ) -> Self::Expressions<'ctx> {
        WriteWaveformExpressions {
            waveform: self.waveform.compile(compiler),
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
