use flosion_macros::ProcessorComponent;
use hashstash::{InplaceUnstasher, Stashable, Stasher, UnstashError, UnstashableInplace};

use crate::{
    core::{
        expression::context::ExpressionContext,
        jit::compiledexpression::Discretization,
        objecttype::{ObjectType, WithObjectType},
        sound::{
            argument::ArgumentScope,
            context::AudioContext,
            expression::ProcessorExpression,
            inputtypes::singleinput::SingleInput,
            soundinput::InputContext,
            soundprocessor::{
                ProcessorState, SoundProcessor, StartOver, StateMarker, StreamStatus,
            },
        },
        soundchunk::{SoundChunk, CHUNK_SIZE},
        stashing::{StashingContext, UnstashingContext},
    },
    ui_core::arguments::ParsedArguments,
};

#[derive(ProcessorComponent)]
pub struct Resampler {
    pub input: SingleInput,
    pub speed_ratio: ProcessorExpression,

    #[state]
    state: StateMarker<ResamplerState>,
}

pub struct ResamplerState {
    init: bool,
    input_chunk: SoundChunk,
    sample_index: usize,
    sample_offset: f32,
}

impl ProcessorState for ResamplerState {
    type Processor = Resampler;

    fn new(_: &Self::Processor) -> Self {
        ResamplerState {
            init: false,
            input_chunk: SoundChunk::new(),
            sample_index: 0,
            sample_offset: 0.0,
        }
    }
}

impl StartOver for ResamplerState {
    fn start_over(&mut self) {
        self.init = false;
        self.sample_index = 0;
        self.sample_offset = 0.0;
    }
}

impl SoundProcessor for Resampler {
    fn new(_args: &ParsedArguments) -> Resampler {
        Resampler {
            input: SingleInput::new_anisochronic(ArgumentScope::new_empty()),
            speed_ratio: ProcessorExpression::new(&[1.0], ArgumentScope::new_empty()),
            state: StateMarker::new(),
        }
    }

    fn is_static(&self) -> bool {
        false
    }

    fn process_audio(
        resampler: &mut Self::CompiledType<'_>,
        dst: &mut SoundChunk,
        context: &mut AudioContext,
    ) -> StreamStatus {
        // TODO: tell context about time speed
        if !resampler.state.init {
            resampler.input.start_over_at(0);
            let mut ch = SoundChunk::new();
            resampler.input.step(&mut ch, InputContext::new(context));
            resampler.state.input_chunk = ch;
            resampler.state.init = true;
        }
        let mut get_next_sample = |s: &mut ResamplerState| -> (f32, f32) {
            s.sample_index += 1;
            if s.sample_index >= CHUNK_SIZE {
                let mut ch: SoundChunk = SoundChunk::new();
                resampler.input.step(&mut ch, InputContext::new(context));
                s.input_chunk = ch;
                s.sample_index = 0;
            }
            let l = s.input_chunk.l[s.sample_index];
            let r = s.input_chunk.r[s.sample_index];
            (l, r)
        };

        // TODO: linear interpolation instead of constant,
        // consider storing previous sample in state
        let mut speedratio = context.get_scratch_space(CHUNK_SIZE);
        resampler.speed_ratio.eval(
            &mut [&mut speedratio],
            Discretization::samplewise_temporal(),
            ExpressionContext::new(context),
        );
        for (dst_sample, delta) in dst
            .samples_mut()
            .zip(speedratio.iter().map(|r| r.clamp(0.0, 16.0)))
        {
            debug_assert!(resampler.state.sample_index < CHUNK_SIZE);
            let curr_sample = resampler
                .state
                .input_chunk
                .sample(resampler.state.sample_index);
            debug_assert!(resampler.state.sample_offset < 1.0);
            let prev_offset = resampler.state.sample_offset;
            resampler.state.sample_offset += delta;
            if resampler.state.sample_offset < 1.0 {
                *dst_sample.0 = curr_sample.0;
                *dst_sample.1 = curr_sample.1;
            } else {
                let mut acc = (0.0, 0.0);
                let k_curr = 1.0 - prev_offset;
                acc.0 += k_curr * curr_sample.0;
                acc.1 += k_curr * curr_sample.1;
                for _ in 0..((resampler.state.sample_offset - 1.0).floor() as usize) {
                    let s = get_next_sample(&mut resampler.state);
                    acc.0 += s.0;
                    acc.1 += s.1;
                }
                let s = get_next_sample(&mut resampler.state);
                resampler.state.sample_offset = resampler.state.sample_offset.fract();
                let k_next = resampler.state.sample_offset;
                acc.0 += k_next * s.0;
                acc.1 += k_next * s.1;
                debug_assert!(delta > 0.0);
                let k_inv = 1.0 / delta;
                *dst_sample.0 = k_inv * acc.0;
                *dst_sample.1 = k_inv * acc.1;
            }
        }
        if resampler.input.timing().is_done() {
            StreamStatus::Done
        } else {
            StreamStatus::Playing
        }
    }
}

impl WithObjectType for Resampler {
    const TYPE: ObjectType = ObjectType::new("resampler");
}

impl Stashable<StashingContext> for Resampler {
    fn stash(&self, stasher: &mut Stasher<StashingContext>) {
        stasher.object(&self.input);
        stasher.object(&self.speed_ratio);
    }
}

impl UnstashableInplace<UnstashingContext<'_>> for Resampler {
    fn unstash_inplace(
        &mut self,
        unstasher: &mut InplaceUnstasher<UnstashingContext>,
    ) -> Result<(), UnstashError> {
        unstasher.object_inplace(&mut self.input)?;
        unstasher.object_inplace(&mut self.speed_ratio)?;
        Ok(())
    }
}
