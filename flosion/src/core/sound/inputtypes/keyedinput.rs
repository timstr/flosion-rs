use std::marker::PhantomData;

use crate::core::{
    engine::{soundgraphcompiler::SoundGraphCompiler, stategraphnode::CompiledSoundInputBranch},
    sound::{
        soundinput::{BasicProcessorInput, InputContext, InputOptions, InputTiming},
        soundprocessor::{
            ProcessorComponent, ProcessorComponentVisitor, ProcessorComponentVisitorMut,
            SoundProcessorId, StartOver, StreamStatus,
        },
    },
    soundchunk::SoundChunk,
};

pub struct KeyedInput<S> {
    input: BasicProcessorInput,
    phantom_data: PhantomData<S>,
}

impl<S: StartOver> KeyedInput<S> {
    pub fn new(options: InputOptions, num_keys: usize) -> KeyedInput<S> {
        KeyedInput {
            input: BasicProcessorInput::new(options, num_keys),
            phantom_data: PhantomData,
        }
    }
}

impl<S: Send> ProcessorComponent for KeyedInput<S> {
    type CompiledType<'ctx> = CompiledKeyedInput<'ctx, S>;

    fn visit<'a>(&self, visitor: &'a mut dyn ProcessorComponentVisitor) {
        visitor.input(&self.input);
    }

    fn visit_mut<'a>(&mut self, visitor: &'a mut dyn ProcessorComponentVisitorMut) {
        visitor.input(&mut self.input);
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
