use crate::{
    core::{
        engine::{
            compiledexpression::{
                CompiledExpression, CompiledExpressionCollection, CompiledExpressionVisitor,
                CompiledExpressionVisitorMut,
            },
            soundgraphcompiler::SoundGraphCompiler,
        },
        jit::compiledexpression::Discretization,
        objecttype::{ObjectType, WithObjectType},
        sound::{
            context::{Context, LocalArrayList},
            expression::SoundExpressionHandle,
            expressionargument::{SoundExpressionArgumentHandle, SoundExpressionArgumentId},
            soundgraphdata::SoundExpressionScope,
            soundinput::InputOptions,
            soundinputtypes::{SingleInput, SingleInputNode},
            soundprocessor::{StateAndTiming, StreamStatus, WhateverSoundProcessor},
            soundprocessortools::SoundProcessorTools,
        },
        soundchunk::{SoundChunk, CHUNK_SIZE},
    },
    ui_core::arguments::ParsedArguments,
};

pub struct Definitions {
    pub sound_input: SingleInput,

    // TODO: store these in a vector. Might need to rethink how DefinitionsExpressions works,
    // e.g. does it need to use Vec or can it use something friendlier to the audio thread?
    pub expression: SoundExpressionHandle,
    pub argument: SoundExpressionArgumentHandle,
}

pub struct DefinitionsExpressions<'ctx> {
    input: CompiledExpression<'ctx>,
    argument_id: SoundExpressionArgumentId,
}

impl<'ctx> CompiledExpressionCollection<'ctx> for DefinitionsExpressions<'ctx> {
    fn visit(&self, visitor: &mut dyn CompiledExpressionVisitor<'ctx>) {
        visitor.visit(&self.input);
    }

    fn visit_mut(&mut self, visitor: &'_ mut dyn CompiledExpressionVisitorMut<'ctx>) {
        visitor.visit(&mut self.input);
    }
}

impl WhateverSoundProcessor for Definitions {
    type StateType = ();

    type SoundInputType = SingleInput;

    type Expressions<'ctx> = DefinitionsExpressions<'ctx>;

    fn new(mut tools: SoundProcessorTools, _args: &ParsedArguments) -> Result<Self, ()> {
        Ok(Definitions {
            sound_input: SingleInput::new(InputOptions::Synchronous, &mut tools),
            expression: tools.add_expression(0.0, SoundExpressionScope::with_processor_state()),
            argument: tools.add_local_array_argument(),
        })
    }

    fn is_static(&self) -> bool {
        false
    }

    fn get_sound_input(&self) -> &Self::SoundInputType {
        &self.sound_input
    }

    fn make_state(&self) -> Self::StateType {
        ()
    }

    fn compile_expressions<'a, 'ctx>(
        &self,
        compile: &SoundGraphCompiler<'a, 'ctx>,
    ) -> Self::Expressions<'ctx> {
        DefinitionsExpressions {
            input: self.expression.compile(compile),
            argument_id: self.argument.id(),
        }
    }

    fn process_audio<'ctx>(
        state: &mut StateAndTiming<()>,
        sound_inputs: &mut SingleInputNode<'ctx>,
        expressions: &mut DefinitionsExpressions<'ctx>,
        dst: &mut SoundChunk,
        context: Context,
    ) -> StreamStatus {
        let mut buffer = context.get_scratch_space(CHUNK_SIZE);

        expressions.input.eval(
            &mut buffer,
            Discretization::samplewise_temporal(),
            &context.push_processor_state(state, LocalArrayList::new()),
        );

        sound_inputs.step(
            state,
            dst,
            &context,
            LocalArrayList::new().push(&buffer, expressions.argument_id),
        )
    }
}

impl WithObjectType for Definitions {
    const TYPE: ObjectType = ObjectType::new("definitions");
}
