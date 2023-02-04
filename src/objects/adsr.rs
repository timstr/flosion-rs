use crate::core::{
    context::Context,
    graphobject::{ObjectInitialization, ObjectType, WithObjectType},
    numberinput::NumberInputHandle,
    numberinputnode::{
        NumberInputNode, NumberInputNodeCollection, NumberInputNodeVisitor,
        NumberInputNodeVisitorMut,
    },
    numeric,
    samplefrequency::SAMPLE_FREQUENCY,
    soundchunk::{SoundChunk, CHUNK_SIZE},
    soundinput::InputOptions,
    soundinputtypes::{SingleInput, SingleInputNode},
    soundprocessor::{DynamicSoundProcessor, StateAndTiming, StreamStatus},
    soundprocessortools::SoundProcessorTools,
    state::State,
};

#[derive(Debug)]
enum Phase {
    Init,
    Attack,
    Decay,
    Sustain,
    Release,
}

pub struct ADSRNumberInputs<'ctx> {
    attack_time: NumberInputNode<'ctx>,
    decay_time: NumberInputNode<'ctx>,
    sustain_level: NumberInputNode<'ctx>,
    release_time: NumberInputNode<'ctx>,
}

impl<'ctx> NumberInputNodeCollection<'ctx> for ADSRNumberInputs<'ctx> {
    fn visit_number_inputs(&self, visitor: &mut dyn NumberInputNodeVisitor<'ctx>) {
        visitor.visit_node(&self.attack_time);
        visitor.visit_node(&self.decay_time);
        visitor.visit_node(&self.sustain_level);
        visitor.visit_node(&self.release_time);
    }

    fn visit_number_inputs_mut(&mut self, visitor: &mut dyn NumberInputNodeVisitorMut<'ctx>) {
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
}

impl State for ADSRState {
    fn reset(&mut self) {
        self.phase = Phase::Init;
    }
}

pub struct ADSR {
    pub input: SingleInput,
    pub attack_time: NumberInputHandle,
    pub decay_time: NumberInputHandle,
    pub sustain_level: NumberInputHandle,
    pub release_time: NumberInputHandle,
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
        numeric::linspace(&mut out_level[..samples_remaining], first_value, last_value);
        samples_remaining
    } else {
        let samples_until_chunk_boundary = out_level.len();
        let last_value = prev_level
            + ((samples_so_far + samples_until_chunk_boundary) as f32 / samples as f32)
                * (next_level - prev_level);
        numeric::linspace(&mut out_level[..], first_value, last_value);
        samples_until_chunk_boundary
    }
}

impl DynamicSoundProcessor for ADSR {
    type StateType = ADSRState;

    type SoundInputType = SingleInput;

    type NumberInputType<'ctx> = ADSRNumberInputs<'ctx>;

    fn new(mut tools: SoundProcessorTools, _init: ObjectInitialization) -> Result<Self, ()> {
        Ok(ADSR {
            input: SingleInput::new(InputOptions::Synchronous, &mut tools),
            attack_time: tools.add_number_input(0.01),
            decay_time: tools.add_number_input(0.2),
            sustain_level: tools.add_number_input(0.5),
            release_time: tools.add_number_input(0.25),
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
        }
    }

    fn make_number_inputs<'ctx>(
        &self,
        context: &'ctx inkwell::context::Context,
    ) -> Self::NumberInputType<'ctx> {
        ADSRNumberInputs {
            attack_time: self.attack_time.make_node(context),
            decay_time: self.decay_time.make_node(context),
            sustain_level: self.sustain_level.make_node(context),
            release_time: self.release_time.make_node(context),
        }
    }

    fn process_audio(
        state: &mut StateAndTiming<ADSRState>,
        sound_input: &mut SingleInputNode,
        number_input: &ADSRNumberInputs,
        dst: &mut SoundChunk,
        mut context: Context,
    ) -> StreamStatus {
        if let Phase::Init = state.phase {
            state.phase = Phase::Attack;
            state.prev_level = 0.0;
            state.next_level = 1.0;
            state.phase_samples =
                (number_input.attack_time.eval_scalar(&context) * SAMPLE_FREQUENCY as f32) as usize;
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
                state.phase_samples = (number_input.decay_time.eval_scalar(&context)
                    * SAMPLE_FREQUENCY as f32) as usize;
                state.prev_level = 1.0;
                state.next_level = number_input
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
            if let Some(sample_offset) = context.take_pending_release() {
                // TODO: consider optionally propagating, e.g.
                // inputs.request_release(sample_offset);
                if sample_offset > cursor {
                    numeric::fill(&mut level[cursor..sample_offset], state.next_level);
                    cursor = sample_offset;
                }
                state.phase = Phase::Release;
                state.phase_samples = (number_input.release_time.eval_scalar(&context)
                    * SAMPLE_FREQUENCY as f32) as usize;
                state.phase_samples_so_far = 0;
                state.prev_level = state.next_level;
                state.next_level = 0.0;
            } else {
                numeric::fill(&mut level[cursor..], state.next_level);
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
                numeric::fill(&mut level[cursor..], 0.0);
                cursor = CHUNK_SIZE;
                status = StreamStatus::Done;
            }
        }

        debug_assert!(cursor == CHUNK_SIZE);
        // TODO: pass level through exponential
        if sound_input.needs_reset() {
            sound_input.reset(0);
        }
        sound_input.step(state, dst, &context);
        numeric::mul_inplace(&mut dst.l, &level);
        numeric::mul_inplace(&mut dst.r, &level);

        // TODO: consider stopping early if input is done
        status
    }
}

impl WithObjectType for ADSR {
    const TYPE: ObjectType = ObjectType::new("adsr");
}
