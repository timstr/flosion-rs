use rand::prelude::*;

use crate::core::{
    context::Context,
    graphobject::{ObjectInitialization, ObjectType, WithObjectType},
    numberinput::NumberInputHandle,
    numberinputnode::{
        NumberInputNode, NumberInputNodeCollection, NumberInputNodeVisitor,
        NumberInputNodeVisitorMut,
    },
    numbersource::StateNumberSourceHandle,
    numeric,
    soundchunk::SoundChunk,
    soundinput::InputOptions,
    soundinputtypes::{KeyedInput, KeyedInputNode},
    soundprocessor::{DynamicSoundProcessor, StateAndTiming, StreamStatus},
    soundprocessortools::SoundProcessorTools,
    state::State,
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
    pub frequency_in: NumberInputHandle,
    pub frequency_spread: NumberInputHandle,
    pub voice_frequency: StateNumberSourceHandle,
}

pub struct EnsembleNumberInputs<'ctx> {
    frequency_in: NumberInputNode<'ctx>,
    frequency_spread: NumberInputNode<'ctx>,
}

impl<'ctx> NumberInputNodeCollection<'ctx> for EnsembleNumberInputs<'ctx> {
    fn visit_number_inputs(&self, visitor: &mut dyn NumberInputNodeVisitor<'ctx>) {
        visitor.visit_node(&self.frequency_in);
        visitor.visit_node(&self.frequency_spread);
    }

    fn visit_number_inputs_mut(&mut self, visitor: &mut dyn NumberInputNodeVisitorMut<'ctx>) {
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

    fn make_number_inputs<'ctx>(
        &self,
        context: &'ctx inkwell::context::Context,
    ) -> Self::NumberInputType<'ctx> {
        EnsembleNumberInputs {
            frequency_in: self.frequency_in.make_node(context),
            frequency_spread: self.frequency_spread.make_node(context),
        }
    }

    fn process_audio<'ctx>(
        state: &mut StateAndTiming<()>,
        sound_inputs: &mut KeyedInputNode<'ctx, VoiceState>,
        number_inputs: &Self::NumberInputType<'ctx>,
        dst: &mut SoundChunk,
        context: Context,
    ) -> StreamStatus {
        // TODO: conceptually, the voice frequency is just a simple function
        // of the input frequency, frequency spread, and per-voice state.
        // It should be possible to evaluate the voice frequency as such
        // for each sample without having to store additional buffers
        // full of all temporary frequency values.
        // This could be implemented by having the Ensemble object add intermediate
        // number sources which do the interesting math after drawing upon the relevant
        // inputs. This would also be a useful pattern for splines in the Melody object.
        // This *might* be possible with zero or minimal changes just by storing
        // handles to the relevant number sources in the processor itself and connecting
        // them together via SoundProcessorTools
        let freq_in = number_inputs.frequency_in.eval_scalar(&context);
        let freq_spread = number_inputs.frequency_spread.eval_scalar(&context);
        for voice_data in sound_inputs.data_mut() {
            let voice_state = voice_data.state_mut();
            if state.just_started() {
                voice_state.spread_ratio = -1.0 + 2.0 * thread_rng().gen::<f32>();
            }
            voice_state.frequency = freq_in * (1.0 + (freq_spread * voice_state.spread_ratio));
        }

        dst.silence();
        let mut temp_chunk = SoundChunk::new();
        for voice_data in sound_inputs.data_mut() {
            if voice_data.needs_reset() {
                voice_data.reset(0);
            }
            voice_data.step(state, &mut temp_chunk, &context);

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
