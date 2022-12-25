use std::{marker::PhantomData, sync::Arc};

use parking_lot::RwLock;

use super::{
    anydata::AnyData,
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
    type NodeType = SingleInputNode;

    fn make_node(&self) -> Self::NodeType {
        SingleInputNode::new(self.id)
    }
}

pub struct SingleInputNode {
    id: SoundInputId,
    timing: InputTiming,
    target: NodeTarget,
    active: bool,
}

impl SingleInputNode {
    pub fn new(id: SoundInputId) -> SingleInputNode {
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
        self.target.reset();
        self.timing.reset(sample_offset);
    }
}

impl SoundInputNode for SingleInputNode {
    fn flag_for_reset(&mut self) {
        self.timing.require_reset();
    }

    fn visit_inputs(&self, visitor: &mut dyn SoundInputNodeVisitor) {
        if self.active {
            visitor.visit_input(self.id, 0, &self.target);
        }
    }

    fn visit_inputs_mut(&mut self, visitor: &mut dyn SoundInputNodeVisitorMut) {
        if self.active {
            visitor.visit_input(self.id, 0, &mut self.target);
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

    fn make_node(&self) -> Self::NodeType {
        KeyedInputNode {
            id: self.id,
            data: (0..self.num_keys)
                .map(|_| KeyedInputData::new(self.id))
                .collect(),
            active: false,
        }
    }
}

pub struct KeyedInputData<S: State + Default> {
    id: SoundInputId,
    timing: InputTiming,
    target: NodeTarget,
    state: S,
}

impl<S: State + Default> KeyedInputData<S> {
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
        self.target.reset();
        self.timing.reset(sample_offset);
    }
}

pub struct KeyedInputNode<S: State + Default> {
    id: SoundInputId,
    data: Vec<KeyedInputData<S>>,
    active: bool,
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

    fn visit_inputs(&self, visitor: &mut dyn SoundInputNodeVisitor) {
        if self.active {
            for (i, d) in self.data.iter().enumerate() {
                visitor.visit_input(d.id, i, &d.target);
            }
        }
    }

    fn visit_inputs_mut(&mut self, visitor: &mut dyn SoundInputNodeVisitorMut) {
        if self.active {
            for (i, d) in self.data.iter_mut().enumerate() {
                visitor.visit_input(d.id, i, &mut d.target);
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
    type NodeType = ();

    fn make_node(&self) -> Self::NodeType {
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
    type NodeType = SingleInputListNode;

    fn make_node(&self) -> Self::NodeType {
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

    fn visit_inputs(&self, visitor: &mut dyn SoundInputNodeVisitor) {
        for i in &self.inputs {
            visitor.visit_input(i.id, 0, &i.target);
        }
    }

    fn visit_inputs_mut(&mut self, visitor: &mut dyn SoundInputNodeVisitorMut) {
        for i in &mut self.inputs {
            visitor.visit_input(i.id, 0, &mut i.target);
        }
    }

    fn add_input(&mut self, input_id: SoundInputId) {
        self.inputs.push(SingleInputNode::new(input_id));
    }

    fn remove_input(&mut self, input_id: SoundInputId) {
        self.inputs.retain(|i| i.id != input_id);
    }
}
