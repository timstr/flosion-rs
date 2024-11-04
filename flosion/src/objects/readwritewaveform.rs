use flosion_macros::ProcessorComponents;
use hashstash::{InplaceUnstasher, Stashable, Stasher, UnstashError, UnstashableInplace};

use crate::{
    core::{
        expression::context::ExpressionContext,
        jit::compiledexpression::Discretization,
        objecttype::{ObjectType, WithObjectType},
        sound::{
            argument::ProcessorArgument,
            argumenttypes::plainf32array::PlainF32ArrayArgument,
            context::Context,
            expression::{ProcessorExpression, SoundExpressionScope},
            inputtypes::singleinput::SingleInput,
            soundinput::{InputContext, InputOptions},
            soundprocessor::{SoundProcessor, StreamStatus},
        },
        soundchunk::SoundChunk,
        stashing::{StashingContext, UnstashingContext},
    },
    ui_core::arguments::ParsedArguments,
};

#[derive(ProcessorComponents)]
pub struct ReadWriteWaveform {
    pub sound_input: SingleInput,
    // TODO: multiple outputs to enable stereo
    pub waveform: ProcessorExpression,
    pub input_l: ProcessorArgument<PlainF32ArrayArgument>,
    pub input_r: ProcessorArgument<PlainF32ArrayArgument>,
}

impl SoundProcessor for ReadWriteWaveform {
    fn new(_args: &ParsedArguments) -> Self {
        let input_l = ProcessorArgument::new();
        let input_r = ProcessorArgument::new();
        let waveform_scope = SoundExpressionScope::new(vec![input_l.id(), input_r.id()]);
        ReadWriteWaveform {
            sound_input: SingleInput::new(InputOptions::Synchronous),
            waveform: ProcessorExpression::new(0.0, waveform_scope),
            input_l,
            input_r,
        }
    }

    fn is_static(&self) -> bool {
        false
    }

    fn process_audio(
        rwwf: &mut Self::CompiledType<'_>,
        dst: &mut SoundChunk,
        context: &mut Context,
    ) -> StreamStatus {
        let mut tmp = SoundChunk::new();
        rwwf.sound_input.step(&mut tmp, InputContext::new(context));
        rwwf.waveform.eval(
            &mut dst.l,
            Discretization::samplewise_temporal(),
            ExpressionContext::new(context)
                .push(rwwf.input_l, &tmp.l)
                .push(rwwf.input_r, &tmp.r),
        );
        slicemath::copy(&dst.l, &mut dst.r);

        StreamStatus::Playing
    }
}

impl WithObjectType for ReadWriteWaveform {
    const TYPE: ObjectType = ObjectType::new("readwritewaveform");
}

impl Stashable<StashingContext> for ReadWriteWaveform {
    fn stash(&self, stasher: &mut Stasher<StashingContext>) {
        stasher.object(&self.sound_input);
        stasher.object(&self.waveform);
        stasher.object(&self.input_l);
        stasher.object(&self.input_r);
    }
}

impl<'a> UnstashableInplace<UnstashingContext<'a>> for ReadWriteWaveform {
    fn unstash_inplace(
        &mut self,
        unstasher: &mut InplaceUnstasher<UnstashingContext>,
    ) -> Result<(), UnstashError> {
        unstasher.object_inplace(&mut self.sound_input)?;
        unstasher.object_inplace(&mut self.waveform)?;
        unstasher.object_inplace(&mut self.input_l)?;
        unstasher.object_inplace(&mut self.input_r)?;
        Ok(())
    }
}
