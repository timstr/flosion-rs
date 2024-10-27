use flosion_macros::ProcessorComponents;
use hashstash::{InplaceUnstasher, Stashable, Stasher, UnstashError};

use crate::{
    core::{
        expression::{context::ExpressionContext, expressionobject::ExpressionObjectFactory},
        jit::compiledexpression::Discretization,
        objecttype::{ObjectType, WithObjectType},
        sound::{
            argument::ProcessorArgument,
            argumenttypes::plainf32array::PlainF32Array,
            context::Context,
            expression::{ProcessorExpression, SoundExpressionScope},
            inputtypes::singleinput::SingleInput,
            soundinput::{InputContext, InputOptions},
            soundprocessor::{SoundProcessor, StreamStatus},
        },
        soundchunk::{SoundChunk, CHUNK_SIZE},
        stashing::StashingContext,
    },
    ui_core::arguments::ParsedArguments,
};

#[derive(ProcessorComponents)]
pub struct Definitions {
    pub sound_input: SingleInput,

    // TODO: store these in a vector
    pub expression: ProcessorExpression,
    pub argument: ProcessorArgument<PlainF32Array>,
}

impl SoundProcessor for Definitions {
    fn new(_args: &ParsedArguments) -> Definitions {
        Definitions {
            sound_input: SingleInput::new(InputOptions::Synchronous),
            expression: ProcessorExpression::new(0.0, SoundExpressionScope::new_empty()),
            argument: ProcessorArgument::new(),
        }
    }

    fn is_static(&self) -> bool {
        false
    }

    fn process_audio(
        defns: &mut Self::CompiledType<'_>,
        dst: &mut SoundChunk,
        context: &mut Context,
    ) -> StreamStatus {
        let mut buffer = context.get_scratch_space(CHUNK_SIZE);

        defns.expression.eval(
            &mut buffer,
            Discretization::samplewise_temporal(),
            ExpressionContext::new(context),
        );

        defns.sound_input.step(
            dst,
            InputContext::new(context).push(defns.argument, &buffer),
        )
    }

    fn unstash_inplace(
        &mut self,
        unstasher: &mut InplaceUnstasher,
        factory: &ExpressionObjectFactory,
    ) -> Result<(), UnstashError> {
        unstasher.object_inplace(&mut self.sound_input)?;
        unstasher.object_proxy_inplace(|unstasher| {
            self.expression.unstash_inplace(unstasher, factory)
        })?;
        unstasher.object_inplace(&mut self.argument)?;
        Ok(())
    }
}

impl WithObjectType for Definitions {
    const TYPE: ObjectType = ObjectType::new("definitions");
}

impl Stashable for Definitions {
    type Context = StashingContext;

    fn stash(&self, stasher: &mut Stasher<StashingContext>) {
        stasher.object(&self.sound_input);
        stasher.object(&self.expression);
        stasher.object(&self.argument);
    }
}
