use flosion_macros::ProcessorComponents;
use hashstash::{InplaceUnstasher, Stashable, Stasher, UnstashError, UnstashableInplace};

use crate::{
    core::{
        expression::context::ExpressionContext,
        jit::compiledexpression::Discretization,
        objecttype::{ObjectType, WithObjectType},
        sound::{
            context::{Context, LocalArrayList},
            expression::{ProcessorExpression, SoundExpressionScope},
            expressionargument::ProcessorArgument,
            input::singleinput::SingleInput,
            soundinput::InputOptions,
            soundprocessor::{SoundProcessor, StreamStatus},
        },
        soundchunk::SoundChunk,
    },
    ui_core::arguments::ParsedArguments,
};

#[derive(ProcessorComponents)]
pub struct ReadWriteWaveform {
    pub sound_input: SingleInput,
    // TODO: multiple outputs to enable stereo
    pub waveform: ProcessorExpression,
    pub input_l: ProcessorArgument,
    pub input_r: ProcessorArgument,
}

impl SoundProcessor for ReadWriteWaveform {
    fn new(_args: &ParsedArguments) -> Self {
        let input_l = ProcessorArgument::new_local_array();
        let input_r = ProcessorArgument::new_local_array();
        let waveform_scope = SoundExpressionScope::with_processor_state()
            .add_local(input_l.id())
            .add_local(input_r.id());
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
        rwwf.sound_input
            .step(&mut tmp, None, LocalArrayList::new(), context);
        rwwf.waveform.eval(
            &mut dst.l,
            Discretization::samplewise_temporal(),
            ExpressionContext::new_with_arrays(
                context,
                LocalArrayList::new()
                    .push(&tmp.l, &rwwf.input_l)
                    .push(&tmp.r, &rwwf.input_r),
            ),
        );
        slicemath::copy(&dst.l, &mut dst.r);

        StreamStatus::Playing
    }
}

impl WithObjectType for ReadWriteWaveform {
    const TYPE: ObjectType = ObjectType::new("readwritewaveform");
}

impl Stashable for ReadWriteWaveform {
    fn stash(&self, stasher: &mut Stasher) {
        stasher.object(&self.sound_input);
        stasher.object(&self.waveform);
        stasher.object(&self.input_l);
        stasher.object(&self.input_r);
    }
}

impl UnstashableInplace for ReadWriteWaveform {
    fn unstash_inplace(&mut self, unstasher: &mut InplaceUnstasher) -> Result<(), UnstashError> {
        unstasher.object_inplace(&mut self.sound_input)?;
        unstasher.object_inplace(&mut self.waveform)?;
        unstasher.object_inplace(&mut self.input_l)?;
        unstasher.object_inplace(&mut self.input_r)?;
        Ok(())
    }
}
