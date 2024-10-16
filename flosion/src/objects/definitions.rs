use crate::{
    core::{
        engine::{compiledexpression::CompiledExpression, soundgraphcompiler::SoundGraphCompiler},
        expression::context::ExpressionContext,
        jit::compiledexpression::Discretization,
        objecttype::{ObjectType, WithObjectType},
        sound::{
            context::{Context, LocalArrayList},
            expression::{ProcessorExpression, SoundExpressionScope},
            expressionargument::{ProcessorArgument, ProcessorArgumentId},
            input::singleinput::{CompiledSingleInput, SingleInput},
            soundinput::InputOptions,
            soundprocessor::{
                ProcessorComponent, ProcessorComponentVisitor, ProcessorComponentVisitorMut,
                SoundProcessor, SoundProcessorId, StartOver, StreamStatus,
            },
        },
        soundchunk::{SoundChunk, CHUNK_SIZE},
    },
    ui_core::arguments::ParsedArguments,
};

pub struct Definitions {
    pub sound_input: SingleInput,

    // TODO: store these in a vector. Might need to rethink how DefinitionsExpressions works,
    // e.g. does it need to use Vec or can it use something friendlier to the audio thread?
    pub expression: ProcessorExpression,
    pub argument: ProcessorArgument,
}

pub struct CompiledDefinitions<'ctx> {
    sound_input: CompiledSingleInput<'ctx>,
    expression: CompiledExpression<'ctx>,
    argument_id: ProcessorArgumentId,
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
        defns: &mut CompiledDefinitions,
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
            LocalArrayList::new().push(&buffer, defns.argument_id),
            context,
        )
    }
}

impl ProcessorComponent for Definitions {
    type CompiledType<'ctx> = CompiledDefinitions<'ctx>;

    fn visit<'a>(&self, visitor: &'a mut dyn ProcessorComponentVisitor) {
        self.sound_input.visit(visitor);
        self.expression.visit(visitor);
        self.argument.visit(visitor);
    }

    fn visit_mut<'a>(&mut self, visitor: &'a mut dyn ProcessorComponentVisitorMut) {
        self.sound_input.visit_mut(visitor);
        self.expression.visit_mut(visitor);
        self.argument.visit_mut(visitor);
    }

    fn compile<'ctx>(
        &self,
        id: SoundProcessorId,
        compiler: &mut SoundGraphCompiler<'_, 'ctx>,
    ) -> CompiledDefinitions<'ctx> {
        CompiledDefinitions {
            sound_input: self.sound_input.compile(id, compiler),
            expression: self.expression.compile(id, compiler),
            argument_id: self.argument.id(),
        }
    }
}

impl<'ctx> StartOver for CompiledDefinitions<'ctx> {
    fn start_over(&mut self) {
        self.sound_input.start_over_at(0);
        self.expression.start_over();
    }
}

impl WithObjectType for Definitions {
    const TYPE: ObjectType = ObjectType::new("definitions");
}