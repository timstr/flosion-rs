use flosion_macros::ProcessorComponents;
use hashstash::{InplaceUnstasher, Stashable, Stasher, UnstashError, UnstashableInplace};

use crate::{
    core::{
        expression::context::ExpressionContext,
        jit::compiledexpression::Discretization,
        objecttype::{ObjectType, WithObjectType},
        samplefrequency::SAMPLE_FREQUENCY,
        sound::{
            argument::{ArgumentScope, ProcessorArgument},
            argumenttypes::plainf32array::PlainF32ArrayArgument,
            context::Context,
            expression::ProcessorExpression,
            soundprocessor::{
                ProcessorState, SoundProcessor, StartOver, StateMarker, StreamStatus,
            },
        },
        soundchunk::{SoundChunk, CHUNK_SIZE},
        stashing::{StashingContext, UnstashingContext},
    },
    ui_core::arguments::ParsedArguments,
};

pub struct WaveGeneratorState {
    phase: [f32; CHUNK_SIZE],
}

impl ProcessorState for WaveGeneratorState {
    type Processor = WaveGenerator;

    fn new(_processor: &Self::Processor) -> Self {
        WaveGeneratorState {
            phase: [0.0; CHUNK_SIZE],
        }
    }
}

impl StartOver for WaveGeneratorState {
    fn start_over(&mut self) {
        slicemath::fill(&mut self.phase, 0.0);
    }
}

#[derive(ProcessorComponents)]
pub struct WaveGenerator {
    pub phase: ProcessorArgument<PlainF32ArrayArgument>,
    pub amplitude: ProcessorExpression,
    pub frequency: ProcessorExpression,

    #[state]
    state: StateMarker<WaveGeneratorState>,
}

impl SoundProcessor for WaveGenerator {
    fn new(_args: &ParsedArguments) -> WaveGenerator {
        let phase = ProcessorArgument::new();
        let phase_id = phase.id();
        WaveGenerator {
            phase,
            amplitude: ProcessorExpression::new(0.0, ArgumentScope::new(vec![phase_id])),
            frequency: ProcessorExpression::new(250.0, ArgumentScope::new_empty()),
            state: StateMarker::new(),
        }
    }

    fn is_static(&self) -> bool {
        false
    }

    fn process_audio(
        wavegen: &mut Self::CompiledType<'_>,
        dst: &mut SoundChunk,
        context: &mut Context,
    ) -> StreamStatus {
        // NOTE: this is made redundant by WriteWaveform and WrappingIntegrator

        let prev_phase: f32 = wavegen.state.phase.last().unwrap().clone();
        wavegen.frequency.eval(
            &mut wavegen.state.phase,
            Discretization::samplewise_temporal(),
            ExpressionContext::new(context),
        );
        slicemath::div_scalar_inplace(&mut wavegen.state.phase, SAMPLE_FREQUENCY as f32);
        slicemath::exclusive_scan_inplace(&mut wavegen.state.phase, prev_phase, |p1, p2| p1 + p2);
        slicemath::apply_unary_inplace(&mut wavegen.state.phase, |x| x - x.floor());

        wavegen.amplitude.eval(
            &mut dst.l,
            Discretization::samplewise_temporal(),
            ExpressionContext::new(context).push(wavegen.phase, &wavegen.state.phase),
        );
        slicemath::copy(&dst.l, &mut dst.r);

        StreamStatus::Playing
    }
}

impl WithObjectType for WaveGenerator {
    const TYPE: ObjectType = ObjectType::new("wavegenerator");
}

impl Stashable<StashingContext> for WaveGenerator {
    fn stash(&self, stasher: &mut Stasher<StashingContext>) {
        stasher.object(&self.phase);
        stasher.object(&self.amplitude);
        stasher.object(&self.frequency);
    }
}

impl UnstashableInplace<UnstashingContext<'_>> for WaveGenerator {
    fn unstash_inplace(
        &mut self,
        unstasher: &mut InplaceUnstasher<UnstashingContext>,
    ) -> Result<(), UnstashError> {
        unstasher.object_inplace(&mut self.phase)?;
        unstasher.object_inplace(&mut self.amplitude)?;
        unstasher.object_inplace(&mut self.frequency)?;
        Ok(())
    }
}
