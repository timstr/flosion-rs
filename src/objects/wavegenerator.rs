use crate::core::{
    anydata::AnyData,
    engine::{
        nodegen::NodeGen,
        compiledexpressionnode::{
            CompiledExpressionNode, ExpressionCollection, ExpressionVisitor, ExpressionVisitorMut,
        },
    },
    graph::graphobject::{ObjectInitialization, ObjectType, WithObjectType},
    jit::compiledexpression::Discretization,
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
};

pub struct WaveGenerator {
    pub phase: SoundExpressionArgumentHandle,
    pub amplitude: SoundExpressionHandle,
    pub frequency: SoundExpressionHandle,
}

pub struct WaveGeneratorExpressions<'ctx> {
    frequency: CompiledExpressionNode<'ctx>,
    amplitude: CompiledExpressionNode<'ctx>,
}

impl<'ctx> ExpressionCollection<'ctx> for WaveGeneratorExpressions<'ctx> {
    fn visit_expressions(&self, visitor: &mut dyn ExpressionVisitor<'ctx>) {
        visitor.visit_node(&self.frequency);
        visitor.visit_node(&self.amplitude);
    }

    fn visit_expressions_mut(&mut self, visitor: &mut dyn ExpressionVisitorMut<'ctx>) {
        visitor.visit_node(&mut self.frequency);
        visitor.visit_node(&mut self.amplitude);
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

    fn new(mut tools: SoundProcessorTools, _init: ObjectInitialization) -> Result<Self, ()> {
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
        nodegen: &NodeGen<'a, 'ctx>,
    ) -> Self::Expressions<'ctx> {
        WaveGeneratorExpressions {
            frequency: self.frequency.make_node(nodegen),
            amplitude: self.amplitude.make_node(nodegen),
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
