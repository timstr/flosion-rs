use crate::core::{
    context::Context,
    graphobject::{ObjectType, WithObjectType},
    numberinput::NumberInputHandle,
    numeric,
    samplefrequency::SAMPLE_FREQUENCY,
    soundchunk::{SoundChunk, CHUNK_SIZE},
    soundinput::InputOptions,
    soundprocessor::{SoundProcessor, StreamStatus},
    soundprocessortools::SoundProcessorTools,
    statetree::{NumberInputNode, ProcessorState, SingleInput, SingleInputNode, State},
};

#[derive(Debug)]
enum Phase {
    Init,
    Attack,
    Decay,
    Sustain,
    Release,
}

pub struct ADSRState {
    phase: Phase,
    phase_samples: usize,
    phase_samples_so_far: usize,
    prev_level: f32,
    next_level: f32,
    attack_time: NumberInputNode,
    decay_time: NumberInputNode,
    sustain_level: NumberInputNode,
    release_time: NumberInputNode,
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

impl SoundProcessor for ADSR {
    const IS_STATIC: bool = false;

    type StateType = ADSRState;

    type InputType = SingleInput;

    fn new(mut tools: SoundProcessorTools) -> Self {
        ADSR {
            input: SingleInput::new(
                InputOptions {
                    interruptible: false,
                    realtime: true,
                },
                &mut tools,
            ),
            attack_time: tools.add_number_input(0.01),
            decay_time: tools.add_number_input(0.2),
            sustain_level: tools.add_number_input(0.5),
            release_time: tools.add_number_input(0.25),
        }
    }

    fn get_input(&self) -> &Self::InputType {
        &self.input
    }

    fn make_state(&self) -> Self::StateType {
        ADSRState {
            phase: Phase::Init,
            phase_samples: 0,
            phase_samples_so_far: 0,
            prev_level: 0.0,
            next_level: 0.0,
            attack_time: self.attack_time.make_node(),
            decay_time: self.decay_time.make_node(),
            sustain_level: self.sustain_level.make_node(),
            release_time: self.release_time.make_node(),
        }
    }

    fn process_audio(
        state: &mut ProcessorState<ADSRState>,
        input: &mut SingleInputNode,
        dst: &mut SoundChunk,
        mut context: Context,
    ) -> StreamStatus {
        if let Phase::Init = state.phase {
            state.phase = Phase::Attack;
            state.prev_level = 0.0;
            state.next_level = 1.0;
            state.phase_samples =
                (state.attack_time.eval_scalar(&context) * SAMPLE_FREQUENCY as f32) as usize;
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
                state.phase_samples =
                    (state.decay_time.eval_scalar(&context) * SAMPLE_FREQUENCY as f32) as usize;
                state.prev_level = 1.0;
                state.next_level = state.sustain_level.eval_scalar(&context).clamp(0.0, 1.0);
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
                state.phase_samples =
                    (state.release_time.eval_scalar(&context) * SAMPLE_FREQUENCY as f32) as usize;
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
        if input.needs_reset() {
            input.reset(0);
        }
        input.step(&state, dst, &context);
        numeric::mul_inplace(&mut dst.l, &level);
        numeric::mul_inplace(&mut dst.r, &level);

        // TODO: consider stopping early if input is done
        status
    }
}

impl WithObjectType for ADSR {
    const TYPE: ObjectType = ObjectType::new("adsr");
}