use crate::{
    core::{
        anydata::AnyData,
        engine::{
            compiledexpression::{
                CompiledExpression, CompiledExpressionCollection, CompiledExpressionVisitor,
                CompiledExpressionVisitorMut,
            },
            soundgraphcompiler::SoundGraphCompiler,
        },
        jit::compiledexpression::Discretization,
        objecttype::{ObjectType, WithObjectType},
        samplefrequency::SAMPLE_FREQUENCY,
        sound::{
            context::{Context, LocalArrayList},
            expression::SoundExpressionHandle,
            expressionargument::SoundExpressionArgumentHandle,
            soundgraphdata::SoundExpressionScope,
            soundprocessor::{DynamicSoundProcessor, StateAndTiming, StreamStatus},
            soundprocessortools::SoundProcessorTools,
            state::State,
        },
        soundchunk::{SoundChunk, CHUNK_SIZE},
    },
    ui_core::arguments::ParsedArguments,
};

pub struct WaveGenerator {
    pub phase: SoundExpressionArgumentHandle,
    pub amplitude: SoundExpressionHandle,
    pub frequency: SoundExpressionHandle,
}

pub struct WaveGeneratorExpressions<'ctx> {
    frequency: CompiledExpression<'ctx>,
    amplitude: CompiledExpression<'ctx>,
}

impl<'ctx> CompiledExpressionCollection<'ctx> for WaveGeneratorExpressions<'ctx> {
    fn visit(&self, visitor: &mut dyn CompiledExpressionVisitor<'ctx>) {
        visitor.visit(&self.frequency);
        visitor.visit(&self.amplitude);
    }

    fn visit_mut(&mut self, visitor: &mut dyn CompiledExpressionVisitorMut<'ctx>) {
        visitor.visit(&mut self.frequency);
        visitor.visit(&mut self.amplitude);
    }
}

pub struct WaveGeneratorState {
    phase: [f32; CHUNK_SIZE],
}

impl State for WaveGeneratorState {
    fn start_over(&mut self) {
        slicemath::fill(&mut self.phase, 0.0);
    }
}

impl DynamicSoundProcessor for WaveGenerator {
    type StateType = WaveGeneratorState;
    type SoundInputType = ();
    type Expressions<'ctx> = WaveGeneratorExpressions<'ctx>;

    fn new(mut tools: SoundProcessorTools, _args: &ParsedArguments) -> Result<Self, ()> {
        Ok(WaveGenerator {
            // TODO: bypass this array entirely?
            phase: tools.add_processor_array_argument(|state: &AnyData| -> &[f32] {
                &state.downcast_if::<WaveGeneratorState>().unwrap().phase
            }),
            amplitude: tools.add_expression(0.0, SoundExpressionScope::with_processor_state()),
            frequency: tools.add_expression(250.0, SoundExpressionScope::with_processor_state()),
        })
    }

    fn get_sound_input(&self) -> &Self::SoundInputType {
        &()
    }

    fn make_state(&self) -> Self::StateType {
        WaveGeneratorState {
            phase: [0.0; CHUNK_SIZE],
        }
    }

    fn compile_expressions<'a, 'ctx>(
        &self,
        compile: &SoundGraphCompiler<'a, 'ctx>,
    ) -> Self::Expressions<'ctx> {
        WaveGeneratorExpressions {
            frequency: self.frequency.compile(compile),
            amplitude: self.amplitude.compile(compile),
        }
    }

    fn process_audio(
        state: &mut StateAndTiming<WaveGeneratorState>,
        _sound_inputs: &mut (),
        expressions: &mut WaveGeneratorExpressions,
        dst: &mut SoundChunk,
        context: Context,
    ) -> StreamStatus {
        let prev_phase = *state.phase.last().unwrap();
        {
            let mut tmp = context.get_scratch_space(state.phase.len());
            expressions.frequency.eval(
                &mut tmp,
                Discretization::samplewise_temporal(),
                &context.push_processor_state(state, LocalArrayList::new()),
            );
            slicemath::copy(&tmp, &mut state.phase);
        }
        slicemath::div_scalar_inplace(&mut state.phase, SAMPLE_FREQUENCY as f32);
        slicemath::exclusive_scan_inplace(&mut state.phase, prev_phase, |p1, p2| p1 + p2);
        slicemath::apply_unary_inplace(&mut state.phase, |x| x - x.floor());

        expressions.amplitude.eval(
            &mut dst.l,
            Discretization::samplewise_temporal(),
            &context.push_processor_state(state, LocalArrayList::new()),
        );
        slicemath::copy(&dst.l, &mut dst.r);

        StreamStatus::Playing
    }
}

impl WithObjectType for WaveGenerator {
    const TYPE: ObjectType = ObjectType::new("wavegenerator");
}
