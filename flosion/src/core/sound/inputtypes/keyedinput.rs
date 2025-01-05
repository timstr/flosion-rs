use std::marker::PhantomData;

use hashstash::{InplaceUnstasher, Stashable, Stasher, UnstashError, UnstashableInplace};

use crate::core::{
    engine::{soundgraphcompiler::SoundGraphCompiler, compiledprocessor::CompiledSoundInputNode},
    sound::{
        argument::ArgumentScope,
        soundinput::{
            InputContext, InputTiming, ProcessorInput, SoundInputBackend, SoundInputCategory,
            SoundInputLocation,
        },
        soundprocessor::{SoundProcessorId, StartOver, StreamStatus},
    },
    soundchunk::SoundChunk,
    stashing::{StashingContext, UnstashingContext},
};

pub struct KeyedInputBackend<S> {
    num_keys: usize,
    phantom_data: PhantomData<S>,
}

impl<S> KeyedInputBackend<S> {
    pub fn new(num_keys: usize) -> KeyedInputBackend<S> {
        KeyedInputBackend {
            num_keys,
            phantom_data: PhantomData,
        }
    }

    pub fn num_keys(&self) -> usize {
        self.num_keys
    }

    pub fn set_num_keys(&mut self, num_keys: usize) {
        self.num_keys = num_keys;
    }
}

impl<S: Send> SoundInputBackend for KeyedInputBackend<S> {
    type CompiledType<'ctx> = CompiledKeyedInput<'ctx, S>;

    fn category(&self) -> SoundInputCategory {
        SoundInputCategory::Branched(self.num_keys)
    }

    fn compile<'ctx>(
        &self,
        location: SoundInputLocation,
        target: Option<SoundProcessorId>,
        compiler: &mut SoundGraphCompiler<'_, 'ctx>,
    ) -> Self::CompiledType<'ctx> {
        CompiledKeyedInput {
            items: (0..self.num_keys)
                .map(|_| CompiledKeyedInputItem {
                    input: CompiledSoundInputNode::new(
                        location,
                        compiler.compile_sound_processor(target),
                    ),
                    state: None,
                })
                .collect(),
        }
    }
}

impl<S> Stashable<StashingContext> for KeyedInputBackend<S> {
    fn stash(&self, stasher: &mut Stasher<StashingContext>) {
        stasher.u64(self.num_keys as _);
    }
}

impl<S> UnstashableInplace<UnstashingContext<'_>> for KeyedInputBackend<S> {
    fn unstash_inplace(
        &mut self,
        unstasher: &mut InplaceUnstasher<UnstashingContext<'_>>,
    ) -> Result<(), UnstashError> {
        let n = unstasher.u64_always()?;
        if unstasher.time_to_write() {
            self.num_keys = n as _;
        }
        Ok(())
    }
}

pub struct CompiledKeyedInputItem<'ctx, S> {
    input: CompiledSoundInputNode<'ctx>,
    state: Option<S>,
}

pub struct CompiledKeyedInput<'ctx, S> {
    items: Vec<CompiledKeyedInputItem<'ctx, S>>,
}

impl<'ctx, S> CompiledKeyedInput<'ctx, S> {
    pub fn items(&self) -> &[CompiledKeyedInputItem<'ctx, S>] {
        &self.items
    }

    pub fn items_mut(&mut self) -> &mut [CompiledKeyedInputItem<'ctx, S>] {
        &mut self.items
    }
}

impl<'ctx, S: 'static> CompiledKeyedInputItem<'ctx, S> {
    pub fn state(&self) -> Option<&S> {
        self.state.as_ref()
    }

    pub fn state_mut(&mut self) -> Option<&mut S> {
        self.state.as_mut()
    }

    pub fn set_state(&mut self, state: S) {
        self.state = Some(state);
    }

    pub fn timing(&self) -> &InputTiming {
        self.input.timing()
    }

    pub fn step(&mut self, dst: &mut SoundChunk, ctx: InputContext) -> StreamStatus {
        self.input.step(dst, ctx)
    }

    pub fn start_over_at(&mut self, sample_offset: usize) {
        self.input.start_over_at(sample_offset);
    }
}

impl<'ctx, S> StartOver for CompiledKeyedInput<'ctx, S> {
    fn start_over(&mut self) {
        for item in &mut self.items {
            item.input.start_over_at(0);
            item.state = None;
        }
    }
}

pub type KeyedInput<S> = ProcessorInput<KeyedInputBackend<S>>;

impl<S> KeyedInput<S> {
    pub fn new(num_keys: usize, argument_scope: ArgumentScope) -> KeyedInput<S> {
        ProcessorInput::new_from_parts(
            argument_scope,
            KeyedInputBackend {
                num_keys,
                phantom_data: PhantomData,
            },
        )
    }
}
