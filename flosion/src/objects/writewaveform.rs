use flosion_macros::ProcessorComponents;
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
            soundprocessor::{SoundProcessor, StreamStatus},
        },
        soundchunk::SoundChunk,
        stashing::{StashingContext, UnstashingContext},
    },
    ui_core::arguments::ParsedArguments,
};

#[derive(ProcessorComponents)]
pub struct WriteWaveform {
    pub waveform: ProcessorExpression,
}

impl SoundProcessor for WriteWaveform {
    fn new(_args: &ParsedArguments) -> WriteWaveform {
        WriteWaveform {
            waveform: ProcessorExpression::new(&[0.0, 0.0], ArgumentScope::new_empty()),
        }
    }

    fn is_static(&self) -> bool {
        false
    }

    fn process_audio(
        wwf: &mut Self::CompiledType<'_>,
        dst: &mut SoundChunk,
        context: &mut AudioContext,
    ) -> StreamStatus {
        wwf.waveform.eval(
            &mut [&mut dst.l, &mut dst.r],
            Discretization::samplewise_temporal(),
            ExpressionContext::new(context),
        );

        StreamStatus::Playing
    }
}

impl WithObjectType for WriteWaveform {
    const TYPE: ObjectType = ObjectType::new("writewaveform");
}

impl Stashable<StashingContext> for WriteWaveform {
    fn stash(&self, stasher: &mut Stasher<StashingContext>) {
        stasher.object(&self.waveform);
    }
}

impl<'a> UnstashableInplace<UnstashingContext<'a>> for WriteWaveform {
    fn unstash_inplace(
        &mut self,
        unstasher: &mut InplaceUnstasher<UnstashingContext>,
    ) -> Result<(), UnstashError> {
        unstasher.object_inplace(&mut self.waveform)?;
        Ok(())
    }
}
