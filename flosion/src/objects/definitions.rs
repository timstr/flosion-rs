use flosion_macros::ProcessorComponents;

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
        soundchunk::{SoundChunk, CHUNK_SIZE},
    },
    ui_core::arguments::ParsedArguments,
};

#[derive(ProcessorComponents)]
pub struct Definitions {
    pub sound_input: SingleInput,

    // TODO: store these in a vector. Might need to rethink how DefinitionsExpressions works,
    // e.g. does it need to use Vec or can it use something friendlier to the audio thread?
    pub expression: ProcessorExpression,
    pub argument: ProcessorArgument,
}

impl SoundProcessor for Definitions {
    fn new(_args: &ParsedArguments) -> Definitions {
        Definitions {
            sound_input: SingleInput::new(InputOptions::Synchronous),
            expression: ProcessorExpression::new(0.0, SoundExpressionScope::with_processor_state()),
            argument: ProcessorArgument::new_local_array(),
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
            ExpressionContext::new_minimal(context),
        );

        defns.sound_input.step(
            dst,
            None,
            LocalArrayList::new().push(&buffer, &defns.argument),
            context,
        )
    }
}

impl WithObjectType for Definitions {
    const TYPE: ObjectType = ObjectType::new("definitions");
}
