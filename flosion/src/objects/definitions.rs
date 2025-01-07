use flosion_macros::ProcessorComponent;
use hashstash::{InplaceUnstasher, Stashable, Stasher, UnstashError, UnstashableInplace};

use crate::{
    core::{
        expression::context::ExpressionContext,
        jit::compiledexpression::Discretization,
        objecttype::{ObjectType, WithObjectType},
        sound::{
            argument::{ArgumentScope, ProcessorArgument},
            argumenttypes::plainf32array::PlainF32ArrayArgument,
            context::AudioContext,
            expression::ProcessorExpression,
            inputtypes::singleinput::SingleInput,
            soundinput::InputContext,
            soundprocessor::{SoundProcessor, StreamStatus},
        },
        soundchunk::{SoundChunk, CHUNK_SIZE},
        stashing::{StashingContext, UnstashingContext},
    },
    ui_core::arguments::ParsedArguments,
};

#[derive(ProcessorComponent)]
pub struct Definitions {
    pub sound_input: SingleInput,

    // TODO: store these in a vector
    pub expression: ProcessorExpression,
    pub argument: ProcessorArgument<PlainF32ArrayArgument>,
}

impl SoundProcessor for Definitions {
    fn new(_args: &ParsedArguments) -> Definitions {
        let argument = ProcessorArgument::new();
        Definitions {
            sound_input: SingleInput::new_isochronic(ArgumentScope::new(vec![argument.id()])),
            expression: ProcessorExpression::new(&[0.0], ArgumentScope::new_empty()),
            argument,
        }
    }

    fn is_static(&self) -> bool {
        false
    }

    fn process_audio(
        defns: &mut Self::CompiledType<'_>,
        dst: &mut SoundChunk,
        context: &mut AudioContext,
    ) -> StreamStatus {
        let mut buffer = context.get_scratch_space(CHUNK_SIZE);

        defns.expression.eval(
            &mut [&mut buffer],
            Discretization::samplewise_temporal(),
            ExpressionContext::new(context),
        );

        defns.sound_input.step(
            dst,
            InputContext::new(context).push(defns.argument, &buffer),
        )
    }
}

impl WithObjectType for Definitions {
    const TYPE: ObjectType = ObjectType::new("definitions");
}

impl Stashable<StashingContext> for Definitions {
    fn stash(&self, stasher: &mut Stasher<StashingContext>) {
        stasher.object(&self.sound_input);
        stasher.object(&self.expression);
        stasher.object(&self.argument);
    }
}

impl<'a> UnstashableInplace<UnstashingContext<'a>> for Definitions {
    fn unstash_inplace(
        &mut self,
        unstasher: &mut InplaceUnstasher<UnstashingContext>,
    ) -> Result<(), UnstashError> {
        unstasher.object_inplace(&mut self.sound_input)?;
        unstasher.object_inplace(&mut self.expression)?;
        unstasher.object_inplace(&mut self.argument)?;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use hashstash::test_stash_roundtrip_inplace;

    use crate::{
        core::{
            expression::expressionobject::ExpressionObjectFactory,
            sound::{
                soundinput::AnyProcessorInput,
                soundobject::SoundObjectFactory,
                soundprocessor::{SoundProcessor, SoundProcessorId},
            },
            stashing::{StashingContext, UnstashingContext},
        },
        ui_core::arguments::ParsedArguments,
    };

    use super::Definitions;

    #[test]
    fn test_stash() {
        let obj_fac = SoundObjectFactory::new_empty();
        let expr_fac = ExpressionObjectFactory::new_empty();

        test_stash_roundtrip_inplace(
            || Definitions::new(&ParsedArguments::new_empty()),
            |definitions| {
                definitions
                    .sound_input
                    .set_target(Some(SoundProcessorId::new(0123)))
            },
            StashingContext::new_stashing_normally(),
            UnstashingContext::new(&obj_fac, &expr_fac),
        )
        .unwrap();
    }
}
