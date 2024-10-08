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
            soundgraphdata::SoundExpressionScope,
            soundinput::InputOptions,
            soundinputtypes::{SingleInput, SingleInputNode},
            soundprocessor::{DynamicSoundProcessor, StateAndTiming, StreamStatus},
            soundprocessortools::SoundProcessorTools,
            state::State,
        },
        soundchunk::{SoundChunk, CHUNK_SIZE},
    },
    ui_core::arguments::ParsedArguments,
};

pub struct Resampler {
    pub input: SingleInput,
    pub speed_ratio: SoundExpressionHandle,
}

pub struct ResamplerExpressions<'ctx> {
    speed_ratio: CompiledExpression<'ctx>,
}

impl<'ctx> CompiledExpressionCollection<'ctx> for ResamplerExpressions<'ctx> {
    fn visit(&self, visitor: &mut dyn CompiledExpressionVisitor<'ctx>) {
        visitor.visit(&self.speed_ratio);
    }

    fn visit_mut(&mut self, visitor: &mut dyn CompiledExpressionVisitorMut<'ctx>) {
        visitor.visit(&mut self.speed_ratio);
    }
}

pub struct ResamplerState {
    init: bool,
    input_chunk: SoundChunk,
    sample_index: usize,
    sample_offset: f32,
}

impl State for ResamplerState {
    fn start_over(&mut self) {
        self.init = false;
        self.sample_index = 0;
        self.sample_offset = 0.0;
    }
}

impl DynamicSoundProcessor for Resampler {
    type StateType = ResamplerState;
    type SoundInputType = SingleInput;
    type Expressions<'ctx> = ResamplerExpressions<'ctx>;

    fn new(mut tools: SoundProcessorTools, _args: &ParsedArguments) -> Result<Self, ()> {
        Ok(Resampler {
            input: SingleInput::new(InputOptions::NonSynchronous, &mut tools),
            speed_ratio: tools.add_expression(1.0, SoundExpressionScope::with_processor_state()),
        })
    }

    fn get_sound_input(&self) -> &Self::SoundInputType {
        &self.input
    }

    fn make_state(&self) -> Self::StateType {
        ResamplerState {
            init: false,
            input_chunk: SoundChunk::new(),
            sample_index: 0,
            sample_offset: 0.0,
        }
    }

    fn compile_expressions<'a, 'ctx>(
        &self,
        compiler: &SoundGraphCompiler<'a, 'ctx>,
    ) -> Self::Expressions<'ctx> {
        ResamplerExpressions {
            speed_ratio: self.speed_ratio.compile(compiler),
        }
    }

    fn process_audio(
        state: &mut StateAndTiming<ResamplerState>,
        sound_inputs: &mut SingleInputNode,
        expressions: &mut ResamplerExpressions,
        dst: &mut SoundChunk,
        context: Context,
    ) -> StreamStatus {
        // TODO: tell context about time speed
        if !state.init {
            sound_inputs.start_over(0);
            let mut ch = SoundChunk::new();
            sound_inputs.step(state, &mut ch, &context, LocalArrayList::new());
            state.input_chunk = ch;
            state.init = true;
        }
        let mut get_next_sample = |s: &mut StateAndTiming<ResamplerState>| -> (f32, f32) {
            s.sample_index += 1;
            if s.sample_index >= CHUNK_SIZE {
                let mut ch: SoundChunk = SoundChunk::new();
                sound_inputs.step(s, &mut ch, &context, LocalArrayList::new());
                s.input_chunk = ch;
                s.sample_index = 0;
            }
            let l = s.input_chunk.l[s.sample_index];
            let r = s.input_chunk.r[s.sample_index];
            (l, r)
        };

        // TODO: linear interpolation instead of constant,
        // consider storing previous sample in state
        let mut speedratio = context.get_scratch_space(CHUNK_SIZE);
        expressions.speed_ratio.eval(
            &mut speedratio,
            Discretization::samplewise_temporal(),
            &context.push_processor_state(state, LocalArrayList::new()),
        );
        for (dst_sample, delta) in dst
            .samples_mut()
            .zip(speedratio.iter().map(|r| r.clamp(0.0, 16.0)))
        {
            debug_assert!(state.sample_index < CHUNK_SIZE);
            let curr_sample = state.input_chunk.sample(state.sample_index);
            debug_assert!(state.sample_offset < 1.0);
            let prev_offset = state.sample_offset;
            state.sample_offset += delta;
            if state.sample_offset < 1.0 {
                *dst_sample.0 = curr_sample.0;
                *dst_sample.1 = curr_sample.1;
            } else {
                let mut acc = (0.0, 0.0);
                let k_curr = 1.0 - prev_offset;
                acc.0 += k_curr * curr_sample.0;
                acc.1 += k_curr * curr_sample.1;
                for _ in 0..((state.sample_offset - 1.0).floor() as usize) {
                    let s = get_next_sample(state);
                    acc.0 += s.0;
                    acc.1 += s.1;
                }
                let s = get_next_sample(state);
                state.sample_offset = state.sample_offset.fract();
                let k_next = state.sample_offset;
                acc.0 += k_next * s.0;
                acc.1 += k_next * s.1;
                debug_assert!(delta > 0.0);
                let k_inv = 1.0 / delta;
                *dst_sample.0 = k_inv * acc.0;
                *dst_sample.1 = k_inv * acc.1;
            }
        }
        if sound_inputs.timing().is_done() {
            StreamStatus::Done
        } else {
            StreamStatus::Playing
        }
    }
}

impl WithObjectType for Resampler {
    const TYPE: ObjectType = ObjectType::new("resampler");
}
