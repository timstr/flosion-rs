use rand::prelude::*;

use crate::core::{
    engine::{
        nodegen::NodeGen,
        soundnumberinputnode::{
            SoundNumberInputNode, SoundNumberInputNodeCollection, SoundNumberInputNodeVisitor,
            SoundNumberInputNodeVisitorMut,
        },
    },
    graph::graphobject::{ObjectInitialization, ObjectType, WithObjectType},
    sound::{
        context::{Context, LocalArrayList},
        soundgraphdata::SoundExpressionScope,
        soundinput::InputOptions,
        soundinputtypes::{KeyedInput, KeyedInputNode},
        expression::SoundExpressionHandle,
        expressionargument::SoundExpressionArgumentHandle,
        soundprocessor::{DynamicSoundProcessor, StateAndTiming, StreamStatus},
        soundprocessortools::SoundProcessorTools,
        state::State,
    },
    soundchunk::SoundChunk,
};

pub struct ScatterInputState {
    // TODO: add support for multiple values
    value: f32,
}

impl State for ScatterInputState {
    fn reset(&mut self) {
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

pub struct ScatterNumberInputs<'ctx> {
    parameter: SoundNumberInputNode<'ctx>,
}

impl<'ctx> SoundNumberInputNodeCollection<'ctx> for ScatterNumberInputs<'ctx> {
    fn visit_number_inputs(&self, visitor: &mut dyn SoundNumberInputNodeVisitor<'ctx>) {
        visitor.visit_node(&self.parameter);
    }

    fn visit_number_inputs_mut(&mut self, visitor: &mut dyn SoundNumberInputNodeVisitorMut<'ctx>) {
        visitor.visit_node(&mut self.parameter);
    }
}

impl DynamicSoundProcessor for Scatter {
    type StateType = ();

    type SoundInputType = KeyedInput<ScatterInputState>;

    type NumberInputType<'ctx> = ScatterNumberInputs<'ctx>;

    fn new(mut tools: SoundProcessorTools, _init: ObjectInitialization) -> Result<Self, ()> {
        let num_keys = 8; // idk
        let input = KeyedInput::new(InputOptions::Synchronous, &mut tools, num_keys);
        let value = tools.add_input_scalar_number_source(input.id(), |state| {
            state.downcast_if::<ScatterInputState>().unwrap().value
        });
        Ok(Scatter {
            sound_input: input,
            parameter: tools.add_number_input(1.0, SoundExpressionScope::with_processor_state()),
            value,
        })
    }

    fn get_sound_input(&self) -> &Self::SoundInputType {
        &self.sound_input
    }

    fn make_state(&self) -> Self::StateType {
        ()
    }

    fn make_number_inputs<'a, 'ctx>(
        &self,
        nodegen: &NodeGen<'a, 'ctx>,
    ) -> Self::NumberInputType<'ctx> {
        ScatterNumberInputs {
            parameter: self.parameter.make_node(nodegen),
        }
    }

    fn process_audio<'ctx>(
        state: &mut StateAndTiming<()>,
        sound_inputs: &mut KeyedInputNode<'ctx, ScatterInputState>,
        number_inputs: &mut Self::NumberInputType<'ctx>,
        dst: &mut SoundChunk,
        context: Context,
    ) -> StreamStatus {
        if state.just_started() {
            let param = number_inputs
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
