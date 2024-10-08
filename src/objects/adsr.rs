use crate::{
    core::{
        engine::soundgraphcompiler::SoundGraphCompiler,
        expression::context::ExpressionContext,
        jit::compiledexpression::Discretization,
        objecttype::{ObjectType, WithObjectType},
        samplefrequency::SAMPLE_FREQUENCY,
        sound::{
            context::{Context, LocalArrayList},
            expression::{ProcessorExpression, SoundExpressionScope},
            soundinput::InputOptions,
            soundinputtypes::SingleInput,
            soundprocessor::{
                ProcessorComponent, ProcessorComponentVisitor, ProcessorComponentVisitorMut,
                SoundProcessorId, StreamStatus, WhateverCompiledSoundProcessor,
                WhateverSoundProcessor,
            },
            state::State,
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

impl State for ADSRState {
    fn start_over(&mut self) {
        self.phase = Phase::Init;
        self.was_released = false;
    }
}

pub struct ADSR {
    pub input: SingleInput,
    pub attack_time: ProcessorExpression,
    pub decay_time: ProcessorExpression,
    pub sustain_level: ProcessorExpression,
    pub release_time: ProcessorExpression,
}

pub struct CompiledADSR<'ctx> {
    input: <SingleInput as ProcessorComponent>::CompiledType<'ctx>,
    attack_time: <ProcessorExpression as ProcessorComponent>::CompiledType<'ctx>,
    decay_time: <ProcessorExpression as ProcessorComponent>::CompiledType<'ctx>,
    sustain_level: <ProcessorExpression as ProcessorComponent>::CompiledType<'ctx>,
    release_time: <ProcessorExpression as ProcessorComponent>::CompiledType<'ctx>,
    state: ADSRState,
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

impl WhateverSoundProcessor for ADSR {
    type CompiledType<'ctx> = CompiledADSR<'ctx>;

    fn new(_args: &ParsedArguments) -> ADSR {
        ADSR {
            input: SingleInput::new(InputOptions::Synchronous),
            attack_time: ProcessorExpression::new(
                0.01,
                SoundExpressionScope::without_processor_state(),
            ),
            decay_time: ProcessorExpression::new(
                0.2,
                SoundExpressionScope::without_processor_state(),
            ),
            sustain_level: ProcessorExpression::new(
                0.5,
                SoundExpressionScope::without_processor_state(),
            ),
            release_time: ProcessorExpression::new(
                0.25,
                SoundExpressionScope::without_processor_state(),
            ),
        }
    }

    fn is_static(&self) -> bool {
        false
    }

    fn visit<'a>(&self, visitor: &'a mut dyn ProcessorComponentVisitor) {
        self.attack_time.visit(visitor);
        self.decay_time.visit(visitor);
        self.sustain_level.visit(visitor);
        self.release_time.visit(visitor);
    }

    fn visit_mut<'a>(&mut self, visitor: &'a mut dyn ProcessorComponentVisitorMut) {
        self.attack_time.visit_mut(visitor);
        self.decay_time.visit_mut(visitor);
        self.sustain_level.visit_mut(visitor);
        self.release_time.visit_mut(visitor);
    }

    fn compile<'ctx>(
        &self,
        id: SoundProcessorId,
        compiler: &mut SoundGraphCompiler<'_, 'ctx>,
    ) -> CompiledADSR<'ctx> {
        CompiledADSR {
            input: self.input.compile(id, compiler),
            attack_time: self.attack_time.compile(id, compiler),
            decay_time: self.decay_time.compile(id, compiler),
            sustain_level: self.sustain_level.compile(id, compiler),
            release_time: self.release_time.compile(id, compiler),
            state: ADSRState {
                phase: Phase::Init,
                phase_samples: 0,
                phase_samples_so_far: 0,
                prev_level: 0.0,
                next_level: 0.0,
                was_released: false,
            },
        }
    }
}

impl<'ctx> WhateverCompiledSoundProcessor<'ctx> for CompiledADSR<'ctx> {
    fn process_audio(&mut self, dst: &mut SoundChunk, mut context: Context) -> StreamStatus {
        let pending_release = context.take_pending_release();

        if let Phase::Init = self.state.phase {
            self.state.phase = Phase::Attack;
            self.state.prev_level = 0.0;
            self.state.next_level = 1.0;
            self.state.phase_samples = (self.attack_time.eval_scalar(
                Discretization::chunkwise_temporal(),
                ExpressionContext::new_minimal(context),
            ) * SAMPLE_FREQUENCY as f32) as usize;
            self.state.phase_samples_so_far = 0;
        }

        let mut cursor: usize = 0;
        let mut level = context.get_scratch_space(CHUNK_SIZE);
        let mut status = StreamStatus::Playing;

        if let Phase::Attack = self.state.phase {
            let samples_covered = chunked_interp(
                &mut level[..],
                self.state.phase_samples,
                self.state.phase_samples_so_far,
                self.state.prev_level,
                self.state.next_level,
            );
            self.state.phase_samples_so_far += samples_covered;
            cursor += samples_covered;
            debug_assert!(cursor <= CHUNK_SIZE);

            if cursor < CHUNK_SIZE {
                self.state.phase = Phase::Decay;
                self.state.phase_samples_so_far = 0;
                self.state.phase_samples = (self.decay_time.eval_scalar(
                    Discretization::chunkwise_temporal(),
                    ExpressionContext::new_minimal(context),
                ) * SAMPLE_FREQUENCY as f32) as usize;
                self.state.prev_level = 1.0;
                self.state.next_level = self
                    .sustain_level
                    .eval_scalar(
                        Discretization::chunkwise_temporal(),
                        ExpressionContext::new_minimal(context),
                    )
                    .clamp(0.0, 1.0);
            }
        }

        if let Phase::Decay = self.state.phase {
            let samples_covered = chunked_interp(
                &mut level[cursor..],
                self.state.phase_samples,
                self.state.phase_samples_so_far,
                self.state.prev_level,
                self.state.next_level,
            );
            self.state.phase_samples_so_far += samples_covered;
            cursor += samples_covered;
            debug_assert!(cursor <= CHUNK_SIZE);

            if cursor < CHUNK_SIZE {
                self.state.phase = Phase::Sustain;
                // NOTE: sustain is held until release message is received
                self.state.phase_samples = 0;
                self.state.phase_samples_so_far = 0;
                // NOTE: self.state.next_level already holds sustain level after transition to decay phase
            }
        }

        if let Phase::Sustain = self.state.phase {
            let sample_offset = if self.state.was_released {
                Some(0)
            } else {
                pending_release
            };

            if let Some(sample_offset) = sample_offset {
                // TODO: consider optionally propagating, e.g.
                // inputs.request_release(sample_offset);
                if sample_offset > cursor {
                    slicemath::fill(&mut level[cursor..sample_offset], self.state.next_level);
                    cursor = sample_offset;
                }
                self.state.phase = Phase::Release;
                self.state.phase_samples = (self.release_time.eval_scalar(
                    Discretization::chunkwise_temporal(),
                    ExpressionContext::new_minimal(context),
                ) * SAMPLE_FREQUENCY as f32) as usize;
                self.state.phase_samples_so_far = 0;
                self.state.prev_level = self.state.next_level;
                self.state.next_level = 0.0;
            } else {
                slicemath::fill(&mut level[cursor..], self.state.next_level);
                cursor = CHUNK_SIZE;
            }
        }

        if let Phase::Release = self.state.phase {
            let samples_covered = chunked_interp(
                &mut level[cursor..],
                self.state.phase_samples,
                self.state.phase_samples_so_far,
                self.state.prev_level,
                0.0,
            );
            self.state.phase_samples_so_far += samples_covered;
            cursor += samples_covered;
            debug_assert!(cursor <= CHUNK_SIZE);

            if cursor < CHUNK_SIZE {
                slicemath::fill(&mut level[cursor..], 0.0);
                cursor = CHUNK_SIZE;
                status = StreamStatus::Done;
            }
        }

        if pending_release.is_some() {
            self.state.was_released = true;
        }

        debug_assert!(cursor == CHUNK_SIZE);

        self.input.step(dst, None, LocalArrayList::new(), context);
        slicemath::mul_inplace(&mut dst.l, &level);
        slicemath::mul_inplace(&mut dst.r, &level);

        status
    }

    fn start_over(&mut self) {
        todo!()
    }
}

impl WithObjectType for ADSR {
    const TYPE: ObjectType = ObjectType::new("adsr");
}
