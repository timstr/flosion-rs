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
            expressionargument::{SoundExpressionArgumentHandle, SoundExpressionArgumentId},
            soundgraphdata::SoundExpressionScope,
            soundinput::InputOptions,
            soundinputtypes::{SingleInput, SingleInputNode},
            soundprocessor::{DynamicSoundProcessor, StateAndTiming, StreamStatus},
            soundprocessortools::SoundProcessorTools,
        },
        soundchunk::SoundChunk,
    },
    ui_core::arguments::ParsedArguments,
};

pub struct ReadWriteWaveform {
    pub sound_input: SingleInput,
    // TODO: multiple outputs to enable stereo
    pub waveform: SoundExpressionHandle,
    pub input_l: SoundExpressionArgumentHandle,
    pub input_r: SoundExpressionArgumentHandle,
}

pub struct ReadWriteWaveformExpressions<'ctx> {
    waveform: CompiledExpression<'ctx>,
    input_l: SoundExpressionArgumentId,
    input_r: SoundExpressionArgumentId,
}

impl<'ctx> CompiledExpressionCollection<'ctx> for ReadWriteWaveformExpressions<'ctx> {
    fn visit(&self, visitor: &mut dyn CompiledExpressionVisitor<'ctx>) {
        visitor.visit(&self.waveform);
    }

    fn visit_mut(&mut self, visitor: &mut dyn CompiledExpressionVisitorMut<'ctx>) {
        visitor.visit(&mut self.waveform);
    }
}

impl DynamicSoundProcessor for ReadWriteWaveform {
    type StateType = ();
    type SoundInputType = SingleInput;
    type Expressions<'ctx> = ReadWriteWaveformExpressions<'ctx>;

    fn new(mut tools: SoundProcessorTools, _args: ParsedArguments) -> Result<Self, ()> {
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
        compiler: &SoundGraphCompiler<'a, 'ctx>,
    ) -> Self::Expressions<'ctx> {
        ReadWriteWaveformExpressions {
            waveform: self.waveform.compile(compiler),
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
