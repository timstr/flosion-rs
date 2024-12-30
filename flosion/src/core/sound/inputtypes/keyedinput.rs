use std::marker::PhantomData;

use hashstash::{InplaceUnstasher, Stashable, Stasher, UnstashError, UnstashableInplace};

use crate::core::{
    engine::{soundgraphcompiler::SoundGraphCompiler, stategraphnode::CompiledSoundInputBranch},
    sound::{
        argument::ArgumentScope,
        soundinput::{
            AnyProcessorInput, BasicProcessorInput, Chronicity, InputContext, InputTiming,
            ProcessorInputId,
        },
        soundprocessor::{
            ProcessorComponent, ProcessorComponentVisitor, ProcessorComponentVisitorMut,
            SoundProcessorId, StartOver, StreamStatus,
        },
    },
    soundchunk::SoundChunk,
    stashing::{StashingContext, UnstashingContext},
};

pub struct KeyedInput<S> {
    input: BasicProcessorInput,
    phantom_data: PhantomData<S>,
}

impl<S> KeyedInput<S> {
    pub fn new(chronicity: Chronicity, num_keys: usize, scope: ArgumentScope) -> KeyedInput<S> {
        KeyedInput {
            input: BasicProcessorInput::new(chronicity, num_keys, scope),
            phantom_data: PhantomData,
        }
    }

    pub fn id(&self) -> ProcessorInputId {
        self.input.id()
    }

    pub fn num_keys(&self) -> usize {
        self.input.branches()
    }

    pub fn set_num_keys(&mut self, num_keys: usize) {
        self.input.set_branches(num_keys);
    }
}

impl<S: Send> ProcessorComponent for KeyedInput<S> {
    type CompiledType<'ctx> = CompiledKeyedInput<'ctx, S>;

    fn visit<'a>(&self, visitor: &'a mut dyn ProcessorComponentVisitor) {
        visitor.input(self);
    }

    fn visit_mut<'a>(&mut self, visitor: &'a mut dyn ProcessorComponentVisitorMut) {
        visitor.input(self);
    }

    fn compile<'ctx>(
        &self,
        processor_id: SoundProcessorId,
        compiler: &mut SoundGraphCompiler<'_, 'ctx>,
    ) -> Self::CompiledType<'ctx> {
        let mut items = Vec::new();
        for _ in 0..self.input.branches() {
            items.push(CompiledKeyedInputItem {
                input: self.input.compile_branch(processor_id, compiler),
                state: None,
            });
        }
        CompiledKeyedInput { items }
    }
}

impl<S> Stashable<StashingContext> for KeyedInput<S> {
    fn stash(&self, stasher: &mut Stasher<StashingContext>) {
        self.input.stash(stasher);
    }
}

impl<S> UnstashableInplace<UnstashingContext<'_>> for KeyedInput<S> {
    fn unstash_inplace(
        &mut self,
        unstasher: &mut InplaceUnstasher<UnstashingContext<'_>>,
    ) -> Result<(), UnstashError> {
        self.input.unstash_inplace(unstasher)
    }
}

pub struct CompiledKeyedInputItem<'ctx, S> {
    input: CompiledSoundInputBranch<'ctx>,
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

impl<S> AnyProcessorInput for KeyedInput<S> {
    fn id(&self) -> ProcessorInputId {
        self.input.id()
    }
}
