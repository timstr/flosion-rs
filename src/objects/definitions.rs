use crate::core::{
    engine::{
        compiledexpressionnode::{
            CompiledExpressionNode, ExpressionCollection, ExpressionVisitor, ExpressionVisitorMut,
        },
        nodegen::NodeGen,
    },
    graph::graphobject::{ObjectInitialization, ObjectType, WithObjectType},
    jit::compiledexpression::Discretization,
    sound::{
        context::{Context, LocalArrayList},
        expression::SoundExpressionHandle,
        expressionargument::{SoundExpressionArgumentHandle, SoundExpressionArgumentId},
        soundgraphdata::SoundExpressionScope,
        soundinput::InputOptions,
        soundinputtypes::{SingleInput, SingleInputNode},
        soundprocessor::{DynamicSoundProcessor, StateAndTiming, StreamStatus},
        soundprocessortools::SoundProcessorTools,
    },
    soundchunk::{SoundChunk, CHUNK_SIZE},
};

pub struct Definitions {
    pub sound_input: SingleInput,

    // TODO: store these in a vector. Might need to rethink how DefinitionsExpressions works,
    // e.g. does it need to use Vec or can it use something friendlier to the audio thread?
    pub expression: SoundExpressionHandle,
    pub argument: SoundExpressionArgumentHandle,
}

pub struct DefinitionsExpressions<'ctx> {
    input: CompiledExpressionNode<'ctx>,
    argument_id: SoundExpressionArgumentId,
}

impl<'ctx> ExpressionCollection<'ctx> for DefinitionsExpressions<'ctx> {
    fn visit_expressions(&self, visitor: &mut dyn ExpressionVisitor<'ctx>) {
        visitor.visit_node(&self.input);
    }

    fn visit_expressions_mut(&mut self, visitor: &'_ mut dyn ExpressionVisitorMut<'ctx>) {
        visitor.visit_node(&mut self.input);
    }
}

impl DynamicSoundProcessor for Definitions {
    type StateType = ();

    type SoundInputType = SingleInput;

    type Expressions<'ctx> = DefinitionsExpressions<'ctx>;

    fn new(mut tools: SoundProcessorTools, _init: ObjectInitialization) -> Result<Self, ()> {
        Ok(Definitions {
            sound_input: SingleInput::new(InputOptions::Synchronous, &mut tools),
            expression: tools.add_expression(0.0, SoundExpressionScope::with_processor_state()),
            argument: tools.add_local_array_argument(),
        })
    }

    fn get_sound_input(&self) -> &Self::SoundInputType {
        &self.sound_input
    }

    fn make_state(&self) -> Self::StateType {
        ()
    }

    fn compile_expressions<'a, 'ctx>(
        &self,
        nodegen: &NodeGen<'a, 'ctx>,
    ) -> Self::Expressions<'ctx> {
        DefinitionsExpressions {
            input: self.expression.make_node(nodegen),
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
