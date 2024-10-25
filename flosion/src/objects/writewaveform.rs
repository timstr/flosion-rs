use flosion_macros::ProcessorComponents;
use hashstash::{InplaceUnstasher, Stashable, Stasher, UnstashError, UnstashableInplace};

use crate::{
    core::{
        expression::context::ExpressionContext,
        jit::compiledexpression::Discretization,
        objecttype::{ObjectType, WithObjectType},
        sound::{
            context::Context,
            expression::{ProcessorExpression, SoundExpressionScope},
            soundprocessor::{SoundProcessor, StreamStatus},
        },
        soundchunk::SoundChunk,
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
            waveform: ProcessorExpression::new(0.0, SoundExpressionScope::new_empty()),
        }
    }

    fn is_static(&self) -> bool {
        false
    }

    fn process_audio(
        wwf: &mut Self::CompiledType<'_>,
        dst: &mut SoundChunk,
        context: &mut Context,
    ) -> StreamStatus {
        wwf.waveform.eval(
            &mut dst.l,
            Discretization::samplewise_temporal(),
            ExpressionContext::new(context),
        );
        slicemath::copy(&dst.l, &mut dst.r);

        StreamStatus::Playing
    }
}

impl WithObjectType for WriteWaveform {
    const TYPE: ObjectType = ObjectType::new("writewaveform");
}

impl Stashable for WriteWaveform {
    fn stash(&self, stasher: &mut Stasher) {
        stasher.object(&self.waveform);
    }
}

impl UnstashableInplace for WriteWaveform {
    fn unstash_inplace(&mut self, unstasher: &mut InplaceUnstasher) -> Result<(), UnstashError> {
        unstasher.object_inplace(&mut self.waveform)
    }
}
