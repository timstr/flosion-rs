use std::{marker::PhantomData, sync::Arc};

use parking_lot::RwLock;

use super::{
    context::Context,
    nodeallocator::NodeAllocator,
    numbersource::{KeyedInputNumberSource, StateNumberSourceHandle},
    soundchunk::SoundChunk,
    soundinput::{step_sound_input, InputOptions, InputTiming, SoundInputId},
    soundprocessor::StreamStatus,
    soundprocessortools::SoundProcessorTools,
    statetree::{
        AnyData, ProcessorNodeWrapper, ProcessorState, SoundInputNode, SoundProcessorInput, State,
    },
};

pub struct SingleInput {
    id: SoundInputId,
}

impl SingleInput {
    pub fn new(options: InputOptions, tools: &mut SoundProcessorTools) -> SingleInput {
        SingleInput {
            id: tools.add_sound_input(options, /*num_keys=*/ 1),
        }
    }

    pub fn id(&self) -> SoundInputId {
        self.id
    }
}

impl SoundProcessorInput for SingleInput {
    type NodeType = SingleInputNode;

    fn make_node(&self, allocator: &NodeAllocator) -> Self::NodeType {
        SingleInputNode::new(self.id, allocator.make_state_tree_for(self.id))
    }
}

pub struct SingleInputNode {
    id: SoundInputId,
    timing: InputTiming,
    target: Option<Box<dyn ProcessorNodeWrapper>>,
}

impl SingleInputNode {
    pub fn new(id: SoundInputId, target: Option<Box<dyn ProcessorNodeWrapper>>) -> SingleInputNode {
        SingleInputNode {
            id,
            timing: InputTiming::default(),
            target,
        }
    }

    pub fn is_done(&self) -> bool {
        self.target.is_none() || self.timing.is_done()
    }

    pub fn request_release(&mut self, sample_offset: usize) {
        self.timing.request_release(sample_offset);
    }

    pub fn step<T: ProcessorState>(
        &mut self,
        processor_state: &T,
        dst: &mut SoundChunk,
        ctx: &Context,
    ) -> StreamStatus {
        step_sound_input(
            &mut self.timing,
            &mut self.target,
            processor_state,
            dst,
            ctx,
            AnyData::new(self.id, &()),
        )
    }

    pub fn needs_reset(&self) -> bool {
        self.timing.needs_reset()
    }

    pub fn require_reset(&mut self) {
        self.timing.require_reset();
    }

    pub fn reset(&mut self, sample_offset: usize) {
        if let Some(t) = &mut self.target {
            t.reset();
        }
        self.timing.reset(sample_offset);
    }
}

impl SoundInputNode for SingleInputNode {
    fn flag_for_reset(&mut self) {
        self.timing.require_reset();
    }
}

pub struct KeyedInput<S: State + Default> {
    id: SoundInputId,
    num_keys: usize,
    phantom_data: PhantomData<S>,
}

impl<S: State + Default> KeyedInput<S> {
    pub fn new(options: InputOptions, tools: &mut SoundProcessorTools, num_keys: usize) -> Self {
        let id = tools.add_sound_input(options, num_keys);
        Self {
            id,
            num_keys,
            phantom_data: PhantomData,
        }
    }

    pub fn id(&self) -> SoundInputId {
        self.id
    }

    pub fn add_number_source<F: Fn(&mut [f32], &S)>(
        &self,
        tools: &mut SoundProcessorTools,
        f: F,
    ) -> StateNumberSourceHandle
    where
        F: 'static + Sync + Send + Sized,
    {
        let source = Arc::new(KeyedInputNumberSource::<S, F>::new(self.id, f));
        tools.add_input_number_source(self.id, source)
    }
}

impl<S: State + Default> SoundProcessorInput for KeyedInput<S> {
    type NodeType = KeyedInputNode<S>;

    fn make_node(&self, allocator: &NodeAllocator) -> Self::NodeType {
        KeyedInputNode {
            data: (0..self.num_keys)
                .map(|_| KeyedInputData::new(self.id, allocator.make_state_tree_for(self.id)))
                .collect(),
        }
    }
}

pub struct KeyedInputData<S: State + Default> {
    id: SoundInputId,
    timing: InputTiming,
    target: Option<Box<dyn ProcessorNodeWrapper>>,
    state: S,
}

impl<S: State + Default> KeyedInputData<S> {
    fn new(id: SoundInputId, target: Option<Box<dyn ProcessorNodeWrapper>>) -> Self {
        Self {
            id,
            timing: InputTiming::default(),
            target,
            state: S::default(),
        }
    }

    pub fn is_done(&self) -> bool {
        self.target.is_none() || self.timing.is_done()
    }

    pub fn request_release(&mut self, sample_offset: usize) {
        self.timing.request_release(sample_offset);
    }

    pub fn was_released(&self) -> bool {
        self.timing.was_released()
    }

    pub fn step<T: ProcessorState>(
        &mut self,
        processor_state: &T,
        dst: &mut SoundChunk,
        ctx: &Context,
    ) -> StreamStatus {
        step_sound_input(
            &mut self.timing,
            &mut self.target,
            processor_state,
            dst,
            ctx,
            AnyData::new(self.id, &self.state),
        )
    }

    pub fn state(&self) -> &S {
        &self.state
    }

    pub fn state_mut(&mut self) -> &mut S {
        &mut self.state
    }

    pub fn needs_reset(&self) -> bool {
        self.timing.needs_reset()
    }

    pub fn require_reset(&mut self) {
        self.timing.require_reset();
    }

    pub fn reset(&mut self, sample_offset: usize) {
        if let Some(t) = &mut self.target {
            t.reset();
        }
        self.timing.reset(sample_offset);
    }
}

pub struct KeyedInputNode<S: State + Default> {
    data: Vec<KeyedInputData<S>>,
}

impl<S: State + Default> KeyedInputNode<S> {
    pub fn data(&self) -> &[KeyedInputData<S>] {
        &self.data
    }

    pub fn data_mut(&mut self) -> &mut [KeyedInputData<S>] {
        &mut self.data
    }
}

impl<S: State + Default> SoundInputNode for KeyedInputNode<S> {
    fn flag_for_reset(&mut self) {
        for d in &mut self.data {
            d.timing.require_reset();
        }
    }
}

#[derive(Default)]
pub struct NoInputs {}

impl NoInputs {
    pub fn new() -> NoInputs {
        NoInputs {}
    }
}

impl SoundProcessorInput for NoInputs {
    type NodeType = NoInputs;

    fn make_node(&self, _allocator: &NodeAllocator) -> Self::NodeType {
        NoInputs {}
    }
}

impl SoundInputNode for NoInputs {
    fn flag_for_reset(&mut self) {
        // Nothing to do
    }
}

pub struct SingleInputList {
    // NOTE: this RwLock is mostly a formality, since
    // SoundProcessorTools is required to change the input
    // anyway and therefore mutable access to the topology
    // is already held
    input_ids: RwLock<Vec<SoundInputId>>,
    options: InputOptions,
}

impl SingleInputList {
    pub fn new(
        count: usize,
        options: InputOptions,
        tools: &mut SoundProcessorTools,
    ) -> SingleInputList {
        SingleInputList {
            input_ids: RwLock::new(
                (0..count)
                    .map(|_| tools.add_sound_input(options, /*num_keys=*/ 1))
                    .collect(),
            ),
            options,
        }
    }

    pub fn add_input(&self, tools: &mut SoundProcessorTools) {
        self.input_ids
            .write()
            .push(tools.add_sound_input(self.options, /*num_keys=*/ 1));
    }

    pub fn remove_input(&self, id: SoundInputId, tools: &mut SoundProcessorTools) {
        let mut input_ids = self.input_ids.write();
        assert!(input_ids.iter().filter(|i| **i == id).count() == 1);
        tools.remove_sound_input(id);
        input_ids.retain(|i| *i != id);
    }

    pub fn get_input_ids(&self) -> Vec<SoundInputId> {
        self.input_ids.read().clone()
    }

    pub fn length(&self) -> usize {
        self.input_ids.read().len()
    }
}

impl SoundProcessorInput for SingleInputList {
    type NodeType = SingleInputListNode;

    fn make_node(&self, allocator: &NodeAllocator) -> Self::NodeType {
        SingleInputListNode {
            inputs: self
                .input_ids
                .read()
                .iter()
                .map(|id| SingleInputNode::new(*id, allocator.make_state_tree_for(*id)))
                .collect(),
        }
    }
}

pub struct SingleInputListNode {
    inputs: Vec<SingleInputNode>,
}

impl SingleInputListNode {
    pub fn get(&self) -> &[SingleInputNode] {
        &self.inputs
    }
    pub fn get_mut(&mut self) -> &mut [SingleInputNode] {
        &mut self.inputs
    }
}

impl SoundInputNode for SingleInputListNode {
    fn flag_for_reset(&mut self) {
        for i in &mut self.inputs {
            i.flag_for_reset();
        }
    }
}
