use crate::core::{
    engine::{
        nodegen::NodeGen,
        compiledexpressionnode::{
            CompiledExpressionNode, ExpressionCollection, ExpressionVisitor, ExpressionVisitorMut,
        },
    },
    graph::graphobject::{ObjectInitialization, ObjectType, WithObjectType},
    samplefrequency::SAMPLE_FREQUENCY,
    sound::{
        context::{Context, LocalArrayList},
        expression::SoundExpressionHandle,
        soundgraphdata::SoundExpressionScope,
        soundinput::InputOptions,
        soundinputtypes::{SingleInput, SingleInputNode},
        soundprocessor::{DynamicSoundProcessor, StateAndTiming, StreamStatus},
        soundprocessortools::SoundProcessorTools,
        state::State,
    },
    soundchunk::{SoundChunk, CHUNK_SIZE},
};

#[derive(Debug)]
enum Phase {
    Init,
    Attack,
    Decay,
    Sustain,
    Release,
}

pub struct ADSRExpressions<'ctx> {
    attack_time: CompiledExpressionNode<'ctx>,
    decay_time: CompiledExpressionNode<'ctx>,
    sustain_level: CompiledExpressionNode<'ctx>,
    release_time: CompiledExpressionNode<'ctx>,
}

impl<'ctx> ExpressionCollection<'ctx> for ADSRExpressions<'ctx> {
    fn visit_expressions(&self, visitor: &mut dyn ExpressionVisitor<'ctx>) {
        visitor.visit_node(&self.attack_time);
        visitor.visit_node(&self.decay_time);
        visitor.visit_node(&self.sustain_level);
        visitor.visit_node(&self.release_time);
    }

    fn visit_expressions_mut(&mut self, visitor: &mut dyn ExpressionVisitorMut<'ctx>) {
        visitor.visit_node(&mut self.attack_time);
        visitor.visit_node(&mut self.decay_time);
        visitor.visit_node(&mut self.sustain_level);
        visitor.visit_node(&mut self.release_time);
    }
}

pub struct ADSRState {
    phase: Phase,
    phase_samples: usize,
    phase_samples_so_far: usize,
    prev_level: f32,
    next_level: f32,
    was_released: bool,
}

impl State for ADSRState {
    fn reset(&mut self) {
        self.phase = Phase::Init;
        self.was_released = false;
    }
}

pub struct ADSR {
    pub input: SingleInput,
    pub attack_time: SoundExpressionHandle,
    pub decay_time: SoundExpressionHandle,
    pub sustain_level: SoundExpressionHandle,
    pub release_time: SoundExpressionHandle,
}

// out_level : slice at the beginning of which to produce output level
// samples   : total length of the interpolation domain
// samples_so_far: samples already covered before the start of the output slice
// prev_level: level at start of domain
// next_level: level at end of domain
// ---
// returns: number of samples of out_level filled.
fn chunked_interp(
    out_level: &mut [f32],
    samples: usize,
    samples_so_far: usize,
    prev_level: f32,
    next_level: f32,
) -> usize {
    debug_assert!(samples_so_far <= samples);
    let samples_remaining = samples - samples_so_far;
    let first_value =
        prev_level + (samples_so_far as f32 / samples as f32) * (next_level - prev_level);
    if samples_remaining <= out_level.len() {
        let last_value = next_level;
        slicemath::linspace(
            &mut out_level[..samples_remaining],
            first_value,
            last_value,
            slicemath::EndPoint::Excluded,
        );
        samples_remaining
    } else {
        let samples_until_chunk_boundary = out_level.len();
        let last_value = prev_level
            + ((samples_so_far + samples_until_chunk_boundary) as f32 / samples as f32)
                * (next_level - prev_level);
        slicemath::linspace(
            &mut out_level[..],
            first_value,
            last_value,
            slicemath::EndPoint::Excluded,
        );
        samples_until_chunk_boundary
    }
}

impl DynamicSoundProcessor for ADSR {
    type StateType = ADSRState;

    type SoundInputType = SingleInput;

    type Expressions<'ctx> = ADSRExpressions<'ctx>;

    fn new(mut tools: SoundProcessorTools, _init: ObjectInitialization) -> Result<Self, ()> {
        Ok(ADSR {
            input: SingleInput::new(InputOptions::Synchronous, &mut tools),
            attack_time: tools
                .add_expression(0.01, SoundExpressionScope::without_processor_state()),
            decay_time: tools.add_expression(0.2, SoundExpressionScope::without_processor_state()),
            sustain_level: tools
                .add_expression(0.5, SoundExpressionScope::without_processor_state()),
            release_time: tools
                .add_expression(0.25, SoundExpressionScope::without_processor_state()),
        })
    }

    fn get_sound_input(&self) -> &Self::SoundInputType {
        &self.input
    }

    fn make_state(&self) -> Self::StateType {
        ADSRState {
            phase: Phase::Init,
            phase_samples: 0,
            phase_samples_so_far: 0,
            prev_level: 0.0,
            next_level: 0.0,
            was_released: false,
        }
    }

    fn compile_expressions<'a, 'ctx>(
        &self,
        nodegen: &NodeGen<'a, 'ctx>,
    ) -> Self::Expressions<'ctx> {
        ADSRExpressions {
            attack_time: self.attack_time.make_node(nodegen),
            decay_time: self.decay_time.make_node(nodegen),
            sustain_level: self.sustain_level.make_node(nodegen),
            release_time: self.release_time.make_node(nodegen),
        }
    }

    fn process_audio(
        state: &mut StateAndTiming<ADSRState>,
        sound_input: &mut SingleInputNode,
        expressions: &mut ADSRExpressions,
        dst: &mut SoundChunk,
        mut context: Context,
    ) -> StreamStatus {
        let pending_release = context.take_pending_release();

        if let Phase::Init = state.phase {
            state.phase = Phase::Attack;
            state.prev_level = 0.0;
            state.next_level = 1.0;
            state.phase_samples =
                (expressions.attack_time.eval_scalar(&context) * SAMPLE_FREQUENCY as f32) as usize;
            state.phase_samples_so_far = 0;
        }

        let mut cursor: usize = 0;
        let mut level = context.get_scratch_space(CHUNK_SIZE);
        let mut status = StreamStatus::Playing;

        if let Phase::Attack = state.phase {
            let samples_covered = chunked_interp(
                &mut level[..],
                state.phase_samples,
                state.phase_samples_so_far,
                state.prev_level,
                state.next_level,
            );
            state.phase_samples_so_far += samples_covered;
            cursor += samples_covered;
            debug_assert!(cursor <= CHUNK_SIZE);

            if cursor < CHUNK_SIZE {
                state.phase = Phase::Decay;
                state.phase_samples_so_far = 0;
                state.phase_samples = (expressions.decay_time.eval_scalar(&context)
                    * SAMPLE_FREQUENCY as f32) as usize;
                state.prev_level = 1.0;
                state.next_level = expressions
                    .sustain_level
                    .eval_scalar(&context)
                    .clamp(0.0, 1.0);
            }
        }

        if let Phase::Decay = state.phase {
            let samples_covered = chunked_interp(
                &mut level[cursor..],
                state.phase_samples,
                state.phase_samples_so_far,
                state.prev_level,
                state.next_level,
            );
            state.phase_samples_so_far += samples_covered;
            cursor += samples_covered;
            debug_assert!(cursor <= CHUNK_SIZE);

            if cursor < CHUNK_SIZE {
                state.phase = Phase::Sustain;
                // NOTE: sustain is held until release message is received
                state.phase_samples = 0;
                state.phase_samples_so_far = 0;
                // NOTE: state.next_level already holds sustain level after transition to decay phase
            }
        }

        if let Phase::Sustain = state.phase {
            let sample_offset = if state.was_released {
                Some(0)
            } else {
                pending_release
            };

            if let Some(sample_offset) = sample_offset {
                // TODO: consider optionally propagating, e.g.
                // inputs.request_release(sample_offset);
                if sample_offset > cursor {
                    slicemath::fill(&mut level[cursor..sample_offset], state.next_level);
                    cursor = sample_offset;
                }
                state.phase = Phase::Release;
                state.phase_samples = (expressions.release_time.eval_scalar(&context)
                    * SAMPLE_FREQUENCY as f32) as usize;
                state.phase_samples_so_far = 0;
                state.prev_level = state.next_level;
                state.next_level = 0.0;
            } else {
                slicemath::fill(&mut level[cursor..], state.next_level);
                cursor = CHUNK_SIZE;
            }
        }

        if let Phase::Release = state.phase {
            let samples_covered = chunked_interp(
                &mut level[cursor..],
                state.phase_samples,
                state.phase_samples_so_far,
                state.prev_level,
                0.0,
            );
            state.phase_samples_so_far += samples_covered;
            cursor += samples_covered;
            debug_assert!(cursor <= CHUNK_SIZE);

            if cursor < CHUNK_SIZE {
                slicemath::fill(&mut level[cursor..], 0.0);
                cursor = CHUNK_SIZE;
                status = StreamStatus::Done;
            }
        }

        if pending_release.is_some() {
            state.was_released = true;
        }

        debug_assert!(cursor == CHUNK_SIZE);

        sound_input.step(state, dst, &context, LocalArrayList::new());
        slicemath::mul_inplace(&mut dst.l, &level);
        slicemath::mul_inplace(&mut dst.r, &level);

        status
    }
}

impl WithObjectType for ADSR {
    const TYPE: ObjectType = ObjectType::new("adsr");
}
