use flosion_macros::ProcessorComponents;
use hashstash::{InplaceUnstasher, Stashable, Stasher, UnstashError, UnstashableInplace};
use rand::prelude::*;

use crate::{
    core::{
        expression::context::ExpressionContext,
        jit::compiledexpression::Discretization,
        objecttype::{ObjectType, WithObjectType},
        sound::{
            argument::{ArgumentScope, ProcessorArgument},
            argumenttypes::f32argument::F32Argument,
            context::Context,
            expression::ProcessorExpression,
            inputtypes::keyedinput::KeyedInput,
            soundinput::{InputContext, InputOptions},
            soundprocessor::{SoundProcessor, StartOver, StreamStatus},
        },
        soundchunk::SoundChunk,
        stashing::{StashingContext, UnstashingContext},
    },
    ui_core::arguments::ParsedArguments,
};

pub struct ScatterInputState {
    // TODO: add support for multiple values
    value: f32,
}

impl StartOver for ScatterInputState {
    fn start_over(&mut self) {
        self.value = 0.0;
    }
}

#[derive(ProcessorComponents)]
pub struct Scatter {
    pub sound_input: KeyedInput<ScatterInputState>,

    // TODO: generalize this to e.g. min and max of a uniform distribution,
    // mean and variance of a normal distribution, etc.
    // For now, zero mean uniform distribution with half width given by parameter.
    // ... or add randomness and equivalent distributions to expressions?
    pub parameter: ProcessorExpression,

    pub value: ProcessorArgument<F32Argument>,
}

impl SoundProcessor for Scatter {
    fn new(_args: &ParsedArguments) -> Scatter {
        let num_keys = 8; // idk
        let value = ProcessorArgument::new();
        let input = KeyedInput::new(
            InputOptions::Synchronous,
            num_keys,
            ArgumentScope::new(vec![value.id()]),
        );
        Scatter {
            sound_input: input,
            parameter: ProcessorExpression::new(1.0, ArgumentScope::new_empty()),
            value,
        }
    }

    fn is_static(&self) -> bool {
        false
    }

    fn process_audio(
        scatter: &mut Self::CompiledType<'_>,
        dst: &mut SoundChunk,
        context: &mut Context,
    ) -> StreamStatus {
        if context.current_processor_timing().just_started() {
            let param = scatter.parameter.eval_scalar(
                Discretization::chunkwise_temporal(),
                ExpressionContext::new(context),
            );

            for item in scatter.sound_input.items_mut() {
                item.set_state(ScatterInputState {
                    value: param * (-1.0 + 2.0 * thread_rng().gen::<f32>()),
                });
            }
        }

        dst.silence();
        let mut status = StreamStatus::Done;
        let mut temp_chunk = SoundChunk::new();
        for item in scatter.sound_input.items_mut() {
            let s = item.step(&mut temp_chunk, InputContext::new(context));

            if s == StreamStatus::Playing {
                status = StreamStatus::Playing;
            }

            // TODO: helper tools for mixing
            slicemath::mul_scalar_inplace(&mut temp_chunk.l, 0.1);
            slicemath::mul_scalar_inplace(&mut temp_chunk.r, 0.1);
            slicemath::add_inplace(&mut dst.l, &temp_chunk.l);
            slicemath::add_inplace(&mut dst.r, &temp_chunk.r);
        }

        status
    }
}

impl WithObjectType for Scatter {
    const TYPE: ObjectType = ObjectType::new("scatter");
}

impl Stashable<StashingContext> for Scatter {
    fn stash(&self, stasher: &mut Stasher<StashingContext>) {
        stasher.object(&self.sound_input);
        stasher.object(&self.parameter);
        stasher.object(&self.value);
    }
}

impl UnstashableInplace<UnstashingContext<'_>> for Scatter {
    fn unstash_inplace(
        &mut self,
        unstasher: &mut InplaceUnstasher<UnstashingContext<'_>>,
    ) -> Result<(), UnstashError> {
        unstasher.object_inplace(&mut self.sound_input)?;
        unstasher.object_inplace(&mut self.parameter)?;
        unstasher.object_inplace(&mut self.value)?;
        Ok(())
    }
}
