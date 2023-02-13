use std::{marker::PhantomData, sync::Arc};

use parking_lot::RwLock;

use super::{
    anydata::AnyData,
    compilednumberinput::ArrayReadFunc,
    context::Context,
    numbersource::{KeyedInputNumberSource, StateNumberSourceHandle},
    soundchunk::SoundChunk,
    soundinput::{InputOptions, InputTiming, SoundInputId},
    soundinputnode::{
        SoundInputNode, SoundInputNodeVisitor, SoundInputNodeVisitorMut, SoundProcessorInput,
    },
    soundprocessor::{ProcessorState, StreamStatus},
    soundprocessortools::SoundProcessorTools,
    state::State,
    stategraphnode::NodeTarget,
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
    type NodeType<'ctx> = SingleInputNode<'ctx>;

    fn make_node<'ctx>(&self) -> Self::NodeType<'ctx> {
        SingleInputNode::new(self.id)
    }
}

pub struct SingleInputNode<'ctx> {
    id: SoundInputId,
    timing: InputTiming,
    target: NodeTarget<'ctx>,
    active: bool,
}

impl<'ctx> SingleInputNode<'ctx> {
    pub fn new(id: SoundInputId) -> SingleInputNode<'ctx> {
        SingleInputNode {
            id,
            timing: InputTiming::default(),
            target: NodeTarget::new(),
            active: false,
        }
    }

    pub fn is_done(&self) -> bool {
        self.target.is_empty() || self.timing.is_done()
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
        self.target.step(
            &mut self.timing,
            processor_state,
            dst,
            ctx,
            self.id,
            AnyData::new(&()),
        )
    }

    pub fn needs_reset(&self) -> bool {
        self.timing.needs_reset()
    }

    pub fn require_reset(&mut self) {
        self.timing.require_reset();
    }

    pub fn reset(&mut self, sample_offset: usize) {
        self.target.reset();
        self.timing.reset(sample_offset);
    }
}

impl<'ctx> SoundInputNode<'ctx> for SingleInputNode<'ctx> {
    fn flag_for_reset(&mut self) {
        self.timing.require_reset();
    }

    fn visit_inputs(&self, visitor: &mut dyn SoundInputNodeVisitor<'ctx>) {
        if self.active {
            visitor.visit_input(self.id, 0, &self.target);
        }
    }

    fn visit_inputs_mut(&mut self, visitor: &mut dyn SoundInputNodeVisitorMut<'ctx>) {
        if self.active {
            visitor.visit_input(self.id, 0, &mut self.target, &mut self.timing);
        }
    }

    fn add_input(&mut self, input_id: SoundInputId) {
        debug_assert_eq!(input_id, self.id);
        debug_assert!(!self.active);
        self.active = true;
    }

    fn remove_input(&mut self, input_id: SoundInputId) {
        debug_assert_eq!(input_id, self.id);
        debug_assert!(self.active);
        self.active = false;
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

    // TODO: add/remove keys with SoundProcessorTools
}

impl<S: State + Default> SoundProcessorInput for KeyedInput<S> {
    type NodeType<'ctx> = KeyedInputNode<'ctx, S>;

    fn make_node<'ctx>(&self) -> Self::NodeType<'ctx> {
        KeyedInputNode {
            id: self.id,
            data: (0..self.num_keys)
                .map(|_| KeyedInputData::new(self.id))
                .collect(),
            active: false,
        }
    }
}

pub struct KeyedInputData<'ctx, S: State + Default> {
    id: SoundInputId,
    timing: InputTiming,
    target: NodeTarget<'ctx>,
    state: S,
}

impl<'ctx, S: State + Default> KeyedInputData<'ctx, S> {
    fn new(id: SoundInputId) -> Self {
        Self {
            id,
            timing: InputTiming::default(),
            target: NodeTarget::new(),
            state: S::default(),
        }
    }

    pub fn is_done(&self) -> bool {
        self.target.is_empty() || self.timing.is_done()
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
        self.target.step(
            &mut self.timing,
            processor_state,
            dst,
            ctx,
            self.id,
            AnyData::new(&self.state),
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
        self.target.reset();
        self.timing.reset(sample_offset);
    }
}

pub struct KeyedInputNode<'ctx, S: State + Default> {
    id: SoundInputId,
    data: Vec<KeyedInputData<'ctx, S>>,
    active: bool,
}

impl<'ctx, S: State + Default> KeyedInputNode<'ctx, S> {
    pub fn data(&self) -> &[KeyedInputData<'ctx, S>] {
        &self.data
    }

    pub fn data_mut(&mut self) -> &mut [KeyedInputData<'ctx, S>] {
        &mut self.data
    }
}

impl<'ctx, S: State + Default> SoundInputNode<'ctx> for KeyedInputNode<'ctx, S> {
    fn flag_for_reset(&mut self) {
        for d in &mut self.data {
            d.timing.require_reset();
        }
    }

    fn visit_inputs(&self, visitor: &mut dyn SoundInputNodeVisitor<'ctx>) {
        if self.active {
            for (i, d) in self.data.iter().enumerate() {
                visitor.visit_input(d.id, i, &d.target);
            }
        }
    }

    fn visit_inputs_mut(&mut self, visitor: &mut dyn SoundInputNodeVisitorMut<'ctx>) {
        if self.active {
            for (i, d) in self.data.iter_mut().enumerate() {
                visitor.visit_input(d.id, i, &mut d.target, &mut d.timing);
            }
        }
    }

    fn add_input(&mut self, input_id: SoundInputId) {
        debug_assert_eq!(input_id, self.id);
        debug_assert!(!self.active);
        self.active = true;
    }

    fn remove_input(&mut self, input_id: SoundInputId) {
        debug_assert_eq!(input_id, self.id);
        debug_assert!(self.active);
        self.active = true;
    }

    fn add_key(&mut self, input_id: SoundInputId, index: usize) {
        debug_assert!(input_id == self.id);
        self.data.insert(index, KeyedInputData::new(self.id));
    }

    fn remove_key(&mut self, input_id: SoundInputId, index: usize) {
        debug_assert!(input_id == self.id);
        self.data.remove(index);
    }
}

impl SoundProcessorInput for () {
    type NodeType<'ctx> = ();

    fn make_node<'ctx>(&self) -> Self::NodeType<'ctx> {
        ()
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
        tools.remove_sound_input(id, tools.processor_id());
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
    type NodeType<'ctx> = SingleInputListNode<'ctx>;

    fn make_node<'ctx>(&self) -> Self::NodeType<'ctx> {
        SingleInputListNode {
            inputs: self
                .input_ids
                .read()
                .iter()
                .map(|id| SingleInputNode::new(*id))
                .collect(),
        }
    }
}

pub struct SingleInputListNode<'ctx> {
    inputs: Vec<SingleInputNode<'ctx>>,
}

impl<'ctx> SingleInputListNode<'ctx> {
    pub fn get(&self) -> &[SingleInputNode<'ctx>] {
        &self.inputs
    }
    pub fn get_mut(&mut self) -> &mut [SingleInputNode<'ctx>] {
        &mut self.inputs
    }
}

impl<'ctx> SoundInputNode<'ctx> for SingleInputListNode<'ctx> {
    fn flag_for_reset(&mut self) {
        for i in &mut self.inputs {
            i.flag_for_reset();
        }
    }

    fn visit_inputs(&self, visitor: &mut dyn SoundInputNodeVisitor<'ctx>) {
        for i in &self.inputs {
            visitor.visit_input(i.id, 0, &i.target);
        }
    }

    fn visit_inputs_mut(&mut self, visitor: &mut dyn SoundInputNodeVisitorMut<'ctx>) {
        for i in &mut self.inputs {
            visitor.visit_input(i.id, 0, &mut i.target, &mut i.timing);
        }
    }

    fn add_input(&mut self, input_id: SoundInputId) {
        self.inputs.push(SingleInputNode::new(input_id));
    }

    fn remove_input(&mut self, input_id: SoundInputId) {
        self.inputs.retain(|i| i.id != input_id);
    }
}
