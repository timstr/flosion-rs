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
        context::Context,
        soundinput::InputOptions,
        soundinputtypes::{KeyedInput, KeyedInputNode},
        soundnumberinput::SoundNumberInputHandle,
        soundnumbersource::SoundNumberSourceHandle,
        soundprocessor::{DynamicSoundProcessor, StateAndTiming, StreamStatus},
        soundprocessortools::SoundProcessorTools,
        state::State,
    },
    soundchunk::SoundChunk,
};

pub struct VoiceState {
    spread_ratio: f32,
    frequency: f32,
}

impl State for VoiceState {
    fn reset(&mut self) {
        self.spread_ratio = 0.0;
        self.frequency = 0.0;
    }
}

impl Default for VoiceState {
    fn default() -> Self {
        Self {
            spread_ratio: 0.0,
            frequency: 0.0,
        }
    }
}

pub struct Ensemble {
    pub input: KeyedInput<VoiceState>,
    pub frequency_in: SoundNumberInputHandle,
    pub frequency_spread: SoundNumberInputHandle,
    pub voice_frequency: SoundNumberSourceHandle,
}

pub struct EnsembleNumberInputs<'ctx> {
    frequency_in: SoundNumberInputNode<'ctx>,
    frequency_spread: SoundNumberInputNode<'ctx>,
}

impl<'ctx> SoundNumberInputNodeCollection<'ctx> for EnsembleNumberInputs<'ctx> {
    fn visit_number_inputs(&self, visitor: &mut dyn SoundNumberInputNodeVisitor<'ctx>) {
        visitor.visit_node(&self.frequency_in);
        visitor.visit_node(&self.frequency_spread);
    }

    fn visit_number_inputs_mut(&mut self, visitor: &mut dyn SoundNumberInputNodeVisitorMut<'ctx>) {
        visitor.visit_node(&mut self.frequency_in);
        visitor.visit_node(&mut self.frequency_spread);
    }
}

impl DynamicSoundProcessor for Ensemble {
    type StateType = ();

    type SoundInputType = KeyedInput<VoiceState>;

    type NumberInputType<'ctx> = EnsembleNumberInputs<'ctx>;

    fn new(mut tools: SoundProcessorTools, _init: ObjectInitialization) -> Result<Self, ()> {
        let num_keys = 8; // idk
        let input = KeyedInput::new(InputOptions::Synchronous, &mut tools, num_keys);
        let voice_frequency = tools.add_input_scalar_number_source(input.id(), |state| {
            state.downcast_if::<VoiceState>().unwrap().frequency
        });
        Ok(Ensemble {
            input,
            frequency_in: tools.add_number_input(250.0),
            frequency_spread: tools.add_number_input(0.01),
            voice_frequency,
        })
    }

    fn get_sound_input(&self) -> &Self::SoundInputType {
        &self.input
    }

    fn make_state(&self) -> Self::StateType {
        ()
    }

    fn make_number_inputs<'a, 'ctx>(
        &self,
        nodegen: &NodeGen<'a, 'ctx>,
    ) -> Self::NumberInputType<'ctx> {
        EnsembleNumberInputs {
            frequency_in: self.frequency_in.make_node(nodegen),
            frequency_spread: self.frequency_spread.make_node(nodegen),
        }
    }

    fn process_audio<'ctx>(
        state: &mut StateAndTiming<()>,
        sound_inputs: &mut KeyedInputNode<'ctx, VoiceState>,
        number_inputs: &Self::NumberInputType<'ctx>,
        dst: &mut SoundChunk,
        context: Context,
    ) -> StreamStatus {
        let freq_in = number_inputs.frequency_in.eval_scalar(&context);
        let freq_spread = number_inputs.frequency_spread.eval_scalar(&context);
        for mut item in sound_inputs.items_mut() {
            let voice_state = item.state_mut();
            if state.just_started() {
                voice_state.spread_ratio = -1.0 + 2.0 * thread_rng().gen::<f32>();
            }
            voice_state.frequency = freq_in * (1.0 + (freq_spread * voice_state.spread_ratio));
        }

        dst.silence();
        let mut temp_chunk = SoundChunk::new();
        for mut item in sound_inputs.items_mut() {
            if item.timing().needs_reset() {
                item.reset(0);
            }
            item.step(state, &mut temp_chunk, &context);

            // TODO: helper tools for mixing
            numeric::mul_scalar_inplace(&mut temp_chunk.l, 0.1);
            numeric::mul_scalar_inplace(&mut temp_chunk.r, 0.1);
            numeric::add_inplace(&mut dst.l, &temp_chunk.l);
            numeric::add_inplace(&mut dst.r, &temp_chunk.r);
        }

        StreamStatus::Playing
    }
}

impl WithObjectType for Ensemble {
    const TYPE: ObjectType = ObjectType::new("ensemble");
}
