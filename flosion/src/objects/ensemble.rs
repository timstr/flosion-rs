use flosion_macros::ProcessorComponents;
use hashstash::{InplaceUnstasher, Stashable, Stasher, UnstashError, UnstashableInplace};
use rand::prelude::*;

use crate::{
    core::{
        expression::context::ExpressionContext,
        jit::compiledexpression::Discretization,
        objecttype::{ObjectType, WithObjectType},
        sound::{
            argument::ProcessorArgument,
            argumenttypes::f32argument::F32Argument,
            context::Context,
            expression::{ProcessorExpression, SoundExpressionScope},
            inputtypes::keyedinput::KeyedInput,
            soundinput::{InputContext, InputOptions},
            soundprocessor::{SoundProcessor, StreamStatus},
        },
        soundchunk::SoundChunk,
        stashing::{StashingContext, UnstashingContext},
    },
    ui_core::arguments::ParsedArguments,
};

pub struct VoiceState {
    spread_ratio: f32,
    frequency: f32,
}

impl Default for VoiceState {
    fn default() -> Self {
        Self {
            spread_ratio: 0.0,
            frequency: 0.0,
        }
    }
}

#[derive(ProcessorComponents)]
pub struct Ensemble {
    pub input: KeyedInput<VoiceState>,
    pub frequency_in: ProcessorExpression,
    pub frequency_spread: ProcessorExpression,
    pub voice_frequency: ProcessorArgument<F32Argument>,
}

impl Ensemble {
    pub fn num_voices(&self) -> usize {
        self.input.num_keys()
    }

    pub fn set_num_voices(&mut self, num_voices: usize) {
        self.input.set_num_keys(num_voices);
    }
}

impl SoundProcessor for Ensemble {
    fn new(_args: &ParsedArguments) -> Ensemble {
        let num_keys = 4; // idk
        let input = KeyedInput::new(InputOptions::Synchronous, num_keys);
        let voice_frequency = ProcessorArgument::new();
        Ensemble {
            input,
            frequency_in: ProcessorExpression::new(250.0, SoundExpressionScope::new_empty()),
            frequency_spread: ProcessorExpression::new(0.01, SoundExpressionScope::new_empty()),
            voice_frequency,
        }
    }

    fn is_static(&self) -> bool {
        false
    }

    fn process_audio(
        ensemble: &mut Self::CompiledType<'_>,
        dst: &mut SoundChunk,
        context: &mut Context,
    ) -> StreamStatus {
        // TODO: add a way (generally) to make random initial values in
        // expressions (which would supercede this)

        let freq_in = ensemble.frequency_in.eval_scalar(
            Discretization::chunkwise_temporal(),
            ExpressionContext::new(context),
        );
        let freq_spread = ensemble.frequency_spread.eval_scalar(
            Discretization::chunkwise_temporal(),
            ExpressionContext::new(context),
        );
        for item in ensemble.input.items_mut() {
            if item.state().is_none() {
                item.set_state(VoiceState {
                    spread_ratio: -1.0 + 2.0 * thread_rng().gen::<f32>(),
                    frequency: 0.0,
                });
            }
            let voice_state = item.state_mut().unwrap();
            voice_state.frequency = freq_in * (1.0 + (freq_spread * voice_state.spread_ratio));
        }

        dst.silence();
        let mut temp_chunk = SoundChunk::new();
        for item in ensemble.input.items_mut() {
            item.step(
                &mut temp_chunk,
                InputContext::new(context)
                    .push(ensemble.voice_frequency, item.state().unwrap().frequency),
            );

            // TODO: helper tools for mixing
            slicemath::mul_scalar_inplace(&mut temp_chunk.l, 0.1);
            slicemath::mul_scalar_inplace(&mut temp_chunk.r, 0.1);
            slicemath::add_inplace(&mut dst.l, &temp_chunk.l);
            slicemath::add_inplace(&mut dst.r, &temp_chunk.r);
        }

        StreamStatus::Playing
    }
}

impl WithObjectType for Ensemble {
    const TYPE: ObjectType = ObjectType::new("ensemble");
}

impl Stashable<StashingContext> for Ensemble {
    fn stash(&self, stasher: &mut Stasher<StashingContext>) {
        stasher.object(&self.input);
        stasher.object(&self.frequency_in);
        stasher.object(&self.frequency_spread);
        stasher.object(&self.voice_frequency);
    }
}

impl UnstashableInplace<UnstashingContext<'_>> for Ensemble {
    fn unstash_inplace(
        &mut self,
        unstasher: &mut InplaceUnstasher<UnstashingContext<'_>>,
    ) -> Result<(), UnstashError> {
        unstasher.object_inplace(&mut self.input)?;
        unstasher.object_inplace(&mut self.frequency_in)?;
        unstasher.object_inplace(&mut self.frequency_spread)?;
        unstasher.object_inplace(&mut self.voice_frequency)?;
        Ok(())
    }
}
