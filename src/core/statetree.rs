use std::{any::Any, marker::PhantomData, slice, sync::Arc};

use eframe::egui::mutex::RwLock;

use super::{
    context::Context,
    key::Key,
    numberinput::NumberInputId,
    numbersource::{KeyedInputNumberSource, StateNumberSourceHandle},
    soundchunk::SoundChunk,
    soundgraphtopology::SoundGraphTopology,
    soundinput::{InputOptions, InputTiming, SoundInputId},
    soundprocessor::{SoundProcessor, SoundProcessorId},
    soundprocessortools::SoundProcessorTools,
    uniqueid::UniqueId,
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

impl ProcessorInput for SingleInput {
    type NodeType = SingleInputNode;

    fn make_node(&self, allocator: &NodeAllocator) -> Self::NodeType {
        SingleInputNode::new(
            self.id,
            allocator.processor_id(),
            allocator.make_state_tree_for(self.id),
        )
    }
}

pub struct SingleInputNode {
    id: SoundInputId,
    parent_processor_id: SoundProcessorId,
    timing: InputTiming,
    target: Option<Box<dyn ProcessorNodeWrapper>>,
}

impl SingleInputNode {
    pub fn new(
        id: SoundInputId,
        parent_processor_id: SoundProcessorId,
        target: Option<Box<dyn ProcessorNodeWrapper>>,
    ) -> SingleInputNode {
        SingleInputNode {
            id,
            parent_processor_id,
            timing: InputTiming::default(),
            target,
        }
    }

    pub fn step<T: State>(&mut self, processor_state: &T, dst: &mut SoundChunk, ctx: &Context) {
        if let Some(node) = &mut self.target {
            if self.timing.needs_reset() {
                node.reset();
                self.timing.reset(0); // TODO: how to adjust sample offset?
            }
            let dummy = NoState::default();
            let new_ctx = ctx.push_input(
                AnyData::new(self.parent_processor_id, processor_state),
                AnyData::new(self.id, &dummy),
                AnyData::new(self.id, &dummy),
                &self.timing,
            );
            node.process_audio(dst, new_ctx);
        } else {
            dst.silence();
        }
    }

    pub fn flag_for_reset(&mut self) {
        self.timing.require_reset();
    }
}

pub struct KeyedInput<K: Key, S: State + Default> {
    id: SoundInputId,
    keys: Vec<Arc<K>>,
    dummy_state: PhantomData<S>,
}

impl<K: Key, S: State + Default> KeyedInput<K, S> {
    pub fn new(options: InputOptions, tools: &mut SoundProcessorTools, keys: Vec<K>) -> Self {
        let id = tools.add_sound_input(options, keys.len());
        Self {
            id,
            keys: keys.into_iter().map(Arc::new).collect(),
            dummy_state: PhantomData::default(),
        }
    }

    pub fn id(&self) -> SoundInputId {
        self.id
    }

    pub fn keys(&self) -> &[Arc<K>] {
        &self.keys
    }

    pub fn add_number_source<F: Fn(&mut [f32], &K, &S)>(
        &self,
        tools: &mut SoundProcessorTools,
        f: F,
    ) -> StateNumberSourceHandle
    where
        F: 'static + Sync + Send + Sized,
    {
        let source = Arc::new(KeyedInputNumberSource::<K, S, F>::new(self.id, f));
        tools.add_input_number_source(self.id, source)
    }
}

impl<K: Key, S: State + Default> ProcessorInput for KeyedInput<K, S> {
    type NodeType = KeyedInputNode<K, S>;

    fn make_node(&self, allocator: &NodeAllocator) -> Self::NodeType {
        KeyedInputNode {
            data: self
                .keys
                .iter()
                .map(|k| {
                    KeyedInputData::new(
                        self.id,
                        allocator.processor_id(),
                        allocator.make_state_tree_for(self.id),
                        Arc::clone(k),
                    )
                })
                .collect(),
        }
    }
}

pub struct KeyedInputData<K: Key, S: State + Default> {
    id: SoundInputId,
    parent_processor_id: SoundProcessorId,
    timing: InputTiming,
    target: Option<Box<dyn ProcessorNodeWrapper>>,
    key: Arc<K>,
    state: S,
}

impl<K: Key, S: State + Default> KeyedInputData<K, S> {
    fn new(
        id: SoundInputId,
        parent_processor_id: SoundProcessorId,
        target: Option<Box<dyn ProcessorNodeWrapper>>,
        key: Arc<K>,
    ) -> Self {
        Self {
            id,
            parent_processor_id,
            timing: InputTiming::default(),
            target,
            key,
            state: S::default(),
        }
    }

    pub fn step<T: State>(&mut self, processor_state: &T, dst: &mut SoundChunk, ctx: &Context) {
        if let Some(node) = &mut self.target {
            if self.timing.needs_reset() {
                self.state.reset();
                node.reset();
                self.timing.reset(0); // TODO: how to adjust sample offset?
            }
            let new_ctx = ctx.push_input(
                AnyData::new(self.parent_processor_id, processor_state),
                AnyData::new(self.id, &*self.key),
                AnyData::new(self.id, &self.state),
                &self.timing,
            );
            node.process_audio(dst, new_ctx);
        } else {
            dst.silence();
        }
    }

    pub fn key(&self) -> &K {
        &*self.key
    }

    pub fn state(&self) -> &S {
        &self.state
    }

    pub fn state_mut(&mut self) -> &mut S {
        &mut self.state
    }
    pub fn flag_for_reset(&mut self) {
        self.timing.require_reset();
    }
}

pub struct KeyedInputNode<K: Key, S: State + Default> {
    data: Vec<KeyedInputData<K, S>>,
}

impl<K: Key, S: State + Default> KeyedInputNode<K, S> {
    pub fn data(&self) -> &[KeyedInputData<K, S>] {
        &self.data
    }

    pub fn data_mut(&mut self) -> &mut [KeyedInputData<K, S>] {
        &mut self.data
    }
}

pub trait State: Sync + Send + 'static {
    fn reset(&mut self);
}

// impl<T> ProcessorState for T
// where
//     T: Default + Copy + Sync + Send + 'static,
// {
//     fn reset(&mut self) {
//         *self = Self::default();
//     }
// }

#[derive(Clone, Copy)]
pub struct AnyData<'a, I: UniqueId> {
    owner_id: I,
    data: &'a dyn Any,
}

impl<'a, I: UniqueId> AnyData<'a, I> {
    pub fn new<T: 'static>(owner_id: I, data: &'a T) -> Self {
        Self { owner_id, data }
    }

    pub fn owner_id(&self) -> I {
        self.owner_id
    }

    pub fn downcast_if<T: 'static>(&self, owner_id: I) -> Option<&T> {
        if owner_id != self.owner_id {
            return None;
        }
        // TODO: perform an unchecked cast in release mode
        let r = self.data.downcast_ref::<T>();
        debug_assert!(r.is_some());
        Some(r.unwrap())
    }
}

#[derive(Default)]
pub struct NoState {} // TODO: is this needed? Could probably replace with unit type ()
impl State for NoState {
    fn reset(&mut self) {}
}

pub trait ProcessorInput {
    type NodeType: InputNode;

    fn make_node(&self, allocator: &NodeAllocator) -> Self::NodeType;
}

// Trait used for automating allocation and reallocation of node inputs
// Not concerned with actual audio processing or providing access to
// said inputs - concrete types will provide those.
pub trait InputNode: Sync + Send {
    fn flag_for_reset(&mut self);
}

impl InputNode for SingleInputNode {
    fn flag_for_reset(&mut self) {
        (self as &mut SingleInputNode).flag_for_reset()
    }
}

impl<K: Key, S: State + Default> InputNode for KeyedInputNode<K, S> {
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

impl ProcessorInput for NoInputs {
    type NodeType = NoInputs;

    fn make_node(&self, _allocator: &NodeAllocator) -> Self::NodeType {
        NoInputs {}
    }
}

impl InputNode for NoInputs {
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
}

impl ProcessorInput for SingleInputList {
    type NodeType = SingleInputListNode;

    fn make_node(&self, allocator: &NodeAllocator) -> Self::NodeType {
        SingleInputListNode {
            inputs: self
                .input_ids
                .read()
                .iter()
                .map(|id| {
                    SingleInputNode::new(
                        *id,
                        allocator.processor_id(),
                        allocator.make_state_tree_for(*id),
                    )
                })
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

impl InputNode for SingleInputListNode {
    fn flag_for_reset(&mut self) {
        for i in &mut self.inputs {
            i.flag_for_reset();
        }
    }
}

pub struct NumberInputNode {
    id: NumberInputId,
}

impl NumberInputNode {
    pub(super) fn new(id: NumberInputId) -> Self {
        Self { id }
    }

    pub fn eval(&self, dst: &mut [f32], context: &Context) {
        context.evaluate_number_input(self.id, dst);
    }

    pub fn eval_scalar(&self, dst: &mut f32, context: &Context) {
        let s = slice::from_mut(dst);
        context.evaluate_number_input(self.id, s);
    }
}

pub struct ProcessorNode<T: SoundProcessor> {
    state: T::StateType,
    inputs: <T::InputType as ProcessorInput>::NodeType,
}

impl<T: SoundProcessor> ProcessorNode<T> {
    pub fn new(state: T::StateType, inputs: <T::InputType as ProcessorInput>::NodeType) -> Self {
        Self { state, inputs }
    }

    fn reset(&mut self) {
        self.state.reset();
        self.inputs.flag_for_reset();
    }

    fn process_audio(&mut self, dst: &mut SoundChunk, ctx: Context) {
        T::process_audio(&mut self.state, &mut self.inputs, dst, ctx);
    }
}

pub trait ProcessorNodeWrapper: Sync + Send {
    fn reset(&mut self);
    fn process_audio(&mut self, dst: &mut SoundChunk, ctx: Context);
}

impl<T: SoundProcessor> ProcessorNodeWrapper for ProcessorNode<T> {
    fn reset(&mut self) {
        (self as &mut ProcessorNode<T>).reset()
    }

    fn process_audio(&mut self, dst: &mut SoundChunk, ctx: Context) {
        (self as &mut ProcessorNode<T>).process_audio(dst, ctx)
    }
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum StateOwner {
    SoundInput(SoundInputId),
    SoundProcessor(SoundProcessorId),
}

pub struct NodeAllocator<'a> {
    processor_id: SoundProcessorId,
    topology: &'a SoundGraphTopology,
}

impl<'a> NodeAllocator<'a> {
    pub fn new(
        processor_id: SoundProcessorId,
        topology: &'a SoundGraphTopology,
    ) -> NodeAllocator<'a> {
        NodeAllocator {
            processor_id,
            topology,
        }
    }

    pub fn processor_id(&self) -> SoundProcessorId {
        self.processor_id
    }

    pub fn make_state_tree_for(
        &self,
        input_id: SoundInputId,
    ) -> Option<Box<dyn ProcessorNodeWrapper>> {
        self.topology.make_state_tree_for(input_id)
    }
}
