use flosion_macros::ProcessorComponents;
use hashstash::{InplaceUnstasher, Stashable, Stasher, UnstashError};

use crate::{
    core::{
        expression::{context::ExpressionContext, expressionobject::ExpressionObjectFactory},
        jit::compiledexpression::Discretization,
        objecttype::{ObjectType, WithObjectType},
        samplefrequency::SAMPLE_FREQUENCY,
        sound::{
            context::Context,
            expression::{ProcessorExpression, SoundExpressionScope},
            inputtypes::singleinput::SingleInput,
            soundinput::{InputContext, InputOptions},
            soundprocessor::{
                ProcessorState, SoundProcessor, StartOver, StateMarker, StreamStatus,
            },
        },
        soundchunk::{SoundChunk, CHUNK_SIZE},
    },
    ui_core::arguments::ParsedArguments,
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
    was_released: bool,
}

#[derive(ProcessorComponents)]
pub struct ADSR {
    pub input: SingleInput,
    pub attack_time: ProcessorExpression,
    pub decay_time: ProcessorExpression,
    pub sustain_level: ProcessorExpression,
    pub release_time: ProcessorExpression,

    #[state]
    state: StateMarker<ADSRState>,
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

impl SoundProcessor for ADSR {
    fn new(_args: &ParsedArguments) -> ADSR {
        let adsr = ADSR {
            input: SingleInput::new(InputOptions::Synchronous),
            attack_time: ProcessorExpression::new(0.01, SoundExpressionScope::new_empty()),
            decay_time: ProcessorExpression::new(0.2, SoundExpressionScope::new_empty()),
            sustain_level: ProcessorExpression::new(0.5, SoundExpressionScope::new_empty()),
            release_time: ProcessorExpression::new(0.25, SoundExpressionScope::new_empty()),
            state: StateMarker::new(),
        };

        adsr
    }

    fn is_static(&self) -> bool {
        false
    }

    fn process_audio(
        adsr: &mut CompiledADSR,
        dst: &mut SoundChunk,
        context: &mut Context,
    ) -> StreamStatus {
        let pending_release = context.take_pending_release();

        if let Phase::Init = adsr.state.phase {
            adsr.state.phase = Phase::Attack;
            adsr.state.prev_level = 0.0;
            adsr.state.next_level = 1.0;
            adsr.state.phase_samples = (adsr.attack_time.eval_scalar(
                Discretization::chunkwise_temporal(),
                ExpressionContext::new(context),
            ) * SAMPLE_FREQUENCY as f32) as usize;
            adsr.state.phase_samples_so_far = 0;
        }

        let mut cursor: usize = 0;
        let mut level = context.get_scratch_space(CHUNK_SIZE);
        let mut status = StreamStatus::Playing;

        if let Phase::Attack = adsr.state.phase {
            let samples_covered = chunked_interp(
                &mut level[..],
                adsr.state.phase_samples,
                adsr.state.phase_samples_so_far,
                adsr.state.prev_level,
                adsr.state.next_level,
            );
            adsr.state.phase_samples_so_far += samples_covered;
            cursor += samples_covered;
            debug_assert!(cursor <= CHUNK_SIZE);

            if cursor < CHUNK_SIZE {
                adsr.state.phase = Phase::Decay;
                adsr.state.phase_samples_so_far = 0;
                adsr.state.phase_samples = (adsr.decay_time.eval_scalar(
                    Discretization::chunkwise_temporal(),
                    ExpressionContext::new(context),
                ) * SAMPLE_FREQUENCY as f32) as usize;
                adsr.state.prev_level = 1.0;
                adsr.state.next_level = adsr
                    .sustain_level
                    .eval_scalar(
                        Discretization::chunkwise_temporal(),
                        ExpressionContext::new(context),
                    )
                    .clamp(0.0, 1.0);
            }
        }

        if let Phase::Decay = adsr.state.phase {
            let samples_covered = chunked_interp(
                &mut level[cursor..],
                adsr.state.phase_samples,
                adsr.state.phase_samples_so_far,
                adsr.state.prev_level,
                adsr.state.next_level,
            );
            adsr.state.phase_samples_so_far += samples_covered;
            cursor += samples_covered;
            debug_assert!(cursor <= CHUNK_SIZE);

            if cursor < CHUNK_SIZE {
                adsr.state.phase = Phase::Sustain;
                // NOTE: sustain is held until release message is received
                adsr.state.phase_samples = 0;
                adsr.state.phase_samples_so_far = 0;
                // NOTE: adsr.state.next_level already holds sustain level after transition to decay phase
            }
        }

        if let Phase::Sustain = adsr.state.phase {
            let sample_offset = if adsr.state.was_released {
                Some(0)
            } else {
                pending_release
            };

            if let Some(sample_offset) = sample_offset {
                // TODO: consider optionally propagating, e.g.
                // inputs.request_release(sample_offset);
                if sample_offset > cursor {
                    slicemath::fill(&mut level[cursor..sample_offset], adsr.state.next_level);
                    cursor = sample_offset;
                }
                adsr.state.phase = Phase::Release;
                adsr.state.phase_samples = (adsr.release_time.eval_scalar(
                    Discretization::chunkwise_temporal(),
                    ExpressionContext::new(context),
                ) * SAMPLE_FREQUENCY as f32) as usize;
                adsr.state.phase_samples_so_far = 0;
                adsr.state.prev_level = adsr.state.next_level;
                adsr.state.next_level = 0.0;
            } else {
                slicemath::fill(&mut level[cursor..], adsr.state.next_level);
                cursor = CHUNK_SIZE;
            }
        }

        if let Phase::Release = adsr.state.phase {
            let samples_covered = chunked_interp(
                &mut level[cursor..],
                adsr.state.phase_samples,
                adsr.state.phase_samples_so_far,
                adsr.state.prev_level,
                0.0,
            );
            adsr.state.phase_samples_so_far += samples_covered;
            cursor += samples_covered;
            debug_assert!(cursor <= CHUNK_SIZE);

            if cursor < CHUNK_SIZE {
                slicemath::fill(&mut level[cursor..], 0.0);
                cursor = CHUNK_SIZE;
                status = StreamStatus::Done;
            }
        }

        if pending_release.is_some() {
            adsr.state.was_released = true;
        }

        debug_assert!(cursor == CHUNK_SIZE);

        adsr.input.step(dst, InputContext::new(context));
        slicemath::mul_inplace(&mut dst.l, &level);
        slicemath::mul_inplace(&mut dst.r, &level);

        status
    }

    fn unstash_inplace(
        &mut self,
        unstasher: &mut InplaceUnstasher,
        factory: &ExpressionObjectFactory,
    ) -> Result<(), UnstashError> {
        unstasher.object_inplace(&mut self.input)?;
        unstasher.object_proxy_inplace(|unstasher| {
            self.attack_time.unstash_inplace(unstasher, factory)
        })?;
        unstasher.object_proxy_inplace(|unstasher| {
            self.decay_time.unstash_inplace(unstasher, factory)
        })?;
        unstasher.object_proxy_inplace(|unstasher| {
            self.sustain_level.unstash_inplace(unstasher, factory)
        })?;
        unstasher.object_proxy_inplace(|unstasher| {
            self.release_time.unstash_inplace(unstasher, factory)
        })?;
        Ok(())
    }
}

impl ProcessorState for ADSRState {
    type Processor = ADSR;

    fn new(_processor: &Self::Processor) -> ADSRState {
        ADSRState {
            phase: Phase::Init,
            phase_samples: 0,
            phase_samples_so_far: 0,
            prev_level: 0.0,
            next_level: 0.0,
            was_released: false,
        }
    }
}

impl StartOver for ADSRState {
    fn start_over(&mut self) {
        self.phase = Phase::Init;
        self.was_released = false;
    }
}

impl WithObjectType for ADSR {
    const TYPE: ObjectType = ObjectType::new("adsr");
}

impl Stashable for ADSR {
    fn stash(&self, stasher: &mut Stasher) {
        stasher.object(&self.input);
        stasher.object(&self.attack_time);
        stasher.object(&self.decay_time);
        stasher.object(&self.sustain_level);
        stasher.object(&self.release_time);
    }
}
