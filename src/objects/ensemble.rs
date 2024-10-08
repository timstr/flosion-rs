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

pub struct VoiceState {
    spread_ratio: f32,
    frequency: f32,
}

impl State for VoiceState {
    fn start_over(&mut self) {
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
    pub frequency_in: SoundExpressionHandle,
    pub frequency_spread: SoundExpressionHandle,
    pub voice_frequency: SoundExpressionArgumentHandle,
}

impl Ensemble {
    pub fn num_voices(&self, tools: &SoundProcessorTools) -> usize {
        self.input.num_branches(tools)
    }

    pub fn set_num_voices(&self, num_voices: usize, tools: &mut SoundProcessorTools) {
        self.input.set_num_branches(num_voices, tools);
    }
}

pub struct EnsembleExpressions<'ctx> {
    frequency_in: CompiledExpression<'ctx>,
    frequency_spread: CompiledExpression<'ctx>,
}

impl<'ctx> CompiledExpressionCollection<'ctx> for EnsembleExpressions<'ctx> {
    fn visit(&self, visitor: &mut dyn CompiledExpressionVisitor<'ctx>) {
        visitor.visit(&self.frequency_in);
        visitor.visit(&self.frequency_spread);
    }

    fn visit_mut(&mut self, visitor: &mut dyn CompiledExpressionVisitorMut<'ctx>) {
        visitor.visit(&mut self.frequency_in);
        visitor.visit(&mut self.frequency_spread);
    }
}

impl DynamicSoundProcessor for Ensemble {
    type StateType = ();

    type SoundInputType = KeyedInput<VoiceState>;

    type Expressions<'ctx> = EnsembleExpressions<'ctx>;

    fn new(mut tools: SoundProcessorTools, _args: &ParsedArguments) -> Result<Self, ()> {
        let num_keys = 4; // idk
        let input = KeyedInput::new(InputOptions::Synchronous, &mut tools, num_keys);
        let voice_frequency = tools.add_input_scalar_argument(input.id(), |state| {
            state.downcast_if::<VoiceState>().unwrap().frequency
        });
        Ok(Ensemble {
            input,
            frequency_in: tools.add_expression(250.0, SoundExpressionScope::with_processor_state()),
            frequency_spread: tools
                .add_expression(0.01, SoundExpressionScope::with_processor_state()),
            voice_frequency,
        })
    }

    fn get_sound_input(&self) -> &Self::SoundInputType {
        &self.input
    }

    fn make_state(&self) -> Self::StateType {
        ()
    }

    fn compile_expressions<'a, 'ctx>(
        &self,
        compiler: &SoundGraphCompiler<'a, 'ctx>,
    ) -> Self::Expressions<'ctx> {
        EnsembleExpressions {
            frequency_in: self.frequency_in.compile(compiler),
            frequency_spread: self.frequency_spread.compile(compiler),
        }
    }

    fn process_audio<'ctx>(
        state: &mut StateAndTiming<()>,
        sound_inputs: &mut KeyedInputNode<'ctx, VoiceState>,
        expressions: &mut Self::Expressions<'ctx>,
        dst: &mut SoundChunk,
        context: Context,
    ) -> StreamStatus {
        // TODO: eval_scalar here is the reason that stateful expression nodes don't work,
        // since it implies no time discretization.
        // I want frequency and spread to vary smoothly.
        // How to do this without bloating keyed input state?
        // A buffer is needed to store intermediate results, but
        // how to use those without having to store it between
        // audio callbacks?
        // Consider adding a way to use a borrowed slice as part of the state
        // of an input in the audio context

        let freq_in = expressions
            .frequency_in
            .eval_scalar(&context.push_processor_state(state, LocalArrayList::new()));
        let freq_spread = expressions
            .frequency_spread
            .eval_scalar(&context.push_processor_state(state, LocalArrayList::new()));
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
            item.step(state, &mut temp_chunk, &context, LocalArrayList::new());

            // TODO: helper tools for mixing
            slicemath::mul_scalar_inplace(&mut temp_chunk.l, 0.1);
            slicemath::mul_scalar_inplace(&mut temp_chunk.r, 0.1);
            slicemath::add_inplace(&mut dst.l, &temp_chunk.l);
            slicemath::add_inplace(&mut dst.r, &temp_chunk.r);
        }

        StreamStatus::Playing
    }
}

impl WithObjectType for Ensemble {
    const TYPE: ObjectType = ObjectType::new("ensemble");
}
