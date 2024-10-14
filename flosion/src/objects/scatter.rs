use rand::prelude::*;

use crate::{
    core::{
        engine::{
            compiledexpression::{
                CompiledExpression, CompiledExpressionCollection, CompiledExpressionVisitor,
                CompiledExpressionVisitorMut,
            },
            soundgraphcompiler::SoundGraphCompiler,
        },
        objecttype::{ObjectType, WithObjectType},
        sound::{
            context::{Context, LocalArrayList},
            expression::SoundExpressionHandle,
            expressionargument::SoundExpressionArgumentHandle,
            soundgraphdata::SoundExpressionScope,
            soundinput::InputOptions,
            soundinputtypes::{KeyedInput, KeyedInputNode},
            soundprocessor::{DynamicSoundProcessor, StateAndTiming, StreamStatus},
            soundprocessortools::SoundProcessorTools,
            state::State,
        },
        soundchunk::SoundChunk,
    },
    ui_core::arguments::ParsedArguments,
};

pub struct ScatterInputState {
    // TODO: add support for multiple values
    value: f32,
}

impl State for ScatterInputState {
    fn start_over(&mut self) {
        self.value = 0.0;
    }
}

impl Default for ScatterInputState {
    fn default() -> Self {
        Self { value: 0.0 }
    }
}

pub struct Scatter {
    pub sound_input: KeyedInput<ScatterInputState>,

    // TODO: generalize this to e.g. min and max of a uniform distribution,
    // mean and variance of a normal distribution, etc.
    // For now, zero mean uniform distribution with half width given by parameter.
    pub parameter: SoundExpressionHandle,

    pub value: SoundExpressionArgumentHandle,
}

pub struct ScatterExpressions<'ctx> {
    parameter: CompiledExpression<'ctx>,
}

impl<'ctx> CompiledExpressionCollection<'ctx> for ScatterExpressions<'ctx> {
    fn visit(&self, visitor: &mut dyn CompiledExpressionVisitor<'ctx>) {
        visitor.visit(&self.parameter);
    }

    fn visit_mut(&mut self, visitor: &mut dyn CompiledExpressionVisitorMut<'ctx>) {
        visitor.visit(&mut self.parameter);
    }
}

impl DynamicSoundProcessor for Scatter {
    type StateType = ();

    type SoundInputType = KeyedInput<ScatterInputState>;

    type Expressions<'ctx> = ScatterExpressions<'ctx>;

    fn new(mut tools: SoundProcessorTools, _args: &ParsedArguments) -> Result<Self, ()> {
        let num_keys = 8; // idk
        let input = KeyedInput::new(InputOptions::Synchronous, &mut tools, num_keys);
        let value = tools.add_input_scalar_argument(input.id(), |state| {
            state.downcast_if::<ScatterInputState>().unwrap().value
        });
        Ok(Scatter {
            sound_input: input,
            parameter: tools.add_expression(1.0, SoundExpressionScope::with_processor_state()),
            value,
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
        compiler: &SoundGraphCompiler<'a, 'ctx>,
    ) -> Self::Expressions<'ctx> {
        ScatterExpressions {
            parameter: self.parameter.compile(compiler),
        }
    }

    fn process_audio<'ctx>(
        state: &mut StateAndTiming<()>,
        sound_inputs: &mut KeyedInputNode<'ctx, ScatterInputState>,
        expressions: &mut Self::Expressions<'ctx>,
        dst: &mut SoundChunk,
        context: Context,
    ) -> StreamStatus {
        if state.just_started() {
            let param = expressions
                .parameter
                .eval_scalar(&context.push_processor_state(state, LocalArrayList::new()));

            for mut item in sound_inputs.items_mut() {
                let voice_state = item.state_mut();
                debug_assert!(state.just_started());
                voice_state.value = param * (-1.0 + 2.0 * thread_rng().gen::<f32>());
            }
        }

        dst.silence();
        let mut status = StreamStatus::Done;
        let mut temp_chunk = SoundChunk::new();
        for mut item in sound_inputs.items_mut() {
            let s = item.step(state, &mut temp_chunk, &context, LocalArrayList::new());

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
