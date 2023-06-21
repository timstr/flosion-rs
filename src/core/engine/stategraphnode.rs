use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};

use parking_lot::RwLock;

use crate::core::{
    anydata::AnyData,
    sound::{
        context::Context,
        soundinput::{InputTiming, SoundInputId},
        soundinputnode::{SoundInputNode, SoundProcessorInput},
        soundnumberinputnode::{
            SoundNumberInputNodeCollection, SoundNumberInputNodeVisitor,
            SoundNumberInputNodeVisitorMut,
        },
        soundprocessor::{
            DynamicSoundProcessor, DynamicSoundProcessorWithId, ProcessorState, SoundProcessorId,
            StateAndTiming, StaticSoundProcessor, StaticSoundProcessorWithId, StreamStatus,
        },
    },
    soundchunk::SoundChunk,
};

use super::{
    garbage::{Droppable, Garbage, GarbageChute},
    nodegen::NodeGen,
    scratcharena::ScratchArena,
};

pub struct StaticProcessorNode<'ctx, T: StaticSoundProcessor> {
    processor: Arc<StaticSoundProcessorWithId<T>>,
    sound_input: <T::SoundInputType as SoundProcessorInput>::NodeType<'ctx>,
    number_input: T::NumberInputType<'ctx>,
}

impl<'ctx, T: StaticSoundProcessor> StaticProcessorNode<'ctx, T> {
    pub(crate) fn new<'a>(
        processor: Arc<StaticSoundProcessorWithId<T>>,
        nodegen: &NodeGen<'a, 'ctx>,
    ) -> Self {
        let sound_input = processor.get_sound_input().make_node(nodegen);
        let number_input = processor.make_number_inputs(nodegen);
        Self {
            processor,
            sound_input,
            number_input,
        }
    }
}

pub struct DynamicProcessorNode<'ctx, T: DynamicSoundProcessor> {
    id: SoundProcessorId,
    state: StateAndTiming<T::StateType>,
    sound_input: <T::SoundInputType as SoundProcessorInput>::NodeType<'ctx>,
    number_input: T::NumberInputType<'ctx>,
}

impl<'ctx, T: DynamicSoundProcessor> DynamicProcessorNode<'ctx, T> {
    pub(crate) fn new<'a>(
        processor: &DynamicSoundProcessorWithId<T>,
        nodegen: &NodeGen<'a, 'ctx>,
    ) -> Self {
        Self {
            id: processor.id(),
            state: StateAndTiming::new(processor.make_state()),
            sound_input: processor.get_sound_input().make_node(nodegen),
            number_input: processor.make_number_inputs(nodegen),
        }
    }

    fn reset(&mut self) {
        self.state.reset();
        for t in self.sound_input.targets_mut() {
            t.timing_mut().require_reset();
        }
    }

    fn process_audio(&mut self, dst: &mut SoundChunk, ctx: Context) -> StreamStatus {
        let status = T::process_audio(
            &mut self.state,
            &mut self.sound_input,
            &self.number_input,
            dst,
            ctx,
        );
        self.state.timing.advance_one_chunk();
        status
    }
}

// TODO: make this not pub
pub trait StateGraphNode<'ctx>: Sync + Send {
    fn id(&self) -> SoundProcessorId;
    fn reset(&mut self);
    fn process_audio(&mut self, dst: &mut SoundChunk, ctx: Context) -> StreamStatus;

    // Used for book-keeping optimizations, e.g. to avoid visiting shared nodes twice
    // and because comparing trait objects (fat pointers) for equality is fraught
    fn address(&self) -> *const ();

    fn into_droppable(self: Box<Self>) -> Box<dyn 'ctx + Droppable>;

    fn sound_input_node(&self) -> &dyn SoundInputNode<'ctx>;
    fn sound_input_node_mut(&mut self) -> &mut dyn SoundInputNode<'ctx>;

    fn number_input_node_mut(&mut self) -> &mut dyn SoundNumberInputNodeCollection<'ctx>;
    fn visit_number_inputs(&self, visitor: &mut dyn SoundNumberInputNodeVisitor<'ctx>);
    fn visit_number_inputs_mut(&mut self, visitor: &mut dyn SoundNumberInputNodeVisitorMut<'ctx>);
}

impl<'ctx, T: StaticSoundProcessor> StateGraphNode<'ctx> for StaticProcessorNode<'ctx, T> {
    fn id(&self) -> SoundProcessorId {
        self.processor.id()
    }

    fn reset(&mut self) {
        // Nothing to do. A static processor node cannot be reset.
    }

    fn process_audio(&mut self, dst: &mut SoundChunk, ctx: Context) -> StreamStatus {
        self.processor
            .process_audio(&mut self.sound_input, &self.number_input, dst, ctx);
        StreamStatus::Playing
    }

    fn address(&self) -> *const () {
        let ptr: *const StaticProcessorNode<T> = self;
        ptr as *const ()
    }

    fn sound_input_node(&self) -> &dyn SoundInputNode<'ctx> {
        &self.sound_input
    }
    fn sound_input_node_mut(&mut self) -> &mut dyn SoundInputNode<'ctx> {
        &mut self.sound_input
    }

    fn number_input_node_mut(&mut self) -> &mut dyn SoundNumberInputNodeCollection<'ctx> {
        &mut self.number_input
    }

    fn visit_number_inputs(&self, visitor: &mut dyn SoundNumberInputNodeVisitor<'ctx>) {
        self.number_input.visit_number_inputs(visitor);
    }

    fn visit_number_inputs_mut(&mut self, visitor: &mut dyn SoundNumberInputNodeVisitorMut<'ctx>) {
        self.number_input.visit_number_inputs_mut(visitor);
    }

    fn into_droppable(self: Box<Self>) -> Box<dyn 'ctx + Droppable> {
        self
    }
}

impl<'ctx, T: DynamicSoundProcessor> StateGraphNode<'ctx> for DynamicProcessorNode<'ctx, T> {
    fn id(&self) -> SoundProcessorId {
        self.id
    }

    fn reset(&mut self) {
        (self as &mut DynamicProcessorNode<T>).reset()
    }

    fn process_audio(&mut self, dst: &mut SoundChunk, ctx: Context) -> StreamStatus {
        (self as &mut DynamicProcessorNode<T>).process_audio(dst, ctx)
    }

    fn address(&self) -> *const () {
        let ptr: *const DynamicProcessorNode<T> = self;
        ptr as *const ()
    }

    fn sound_input_node(&self) -> &dyn SoundInputNode<'ctx> {
        &self.sound_input
    }
    fn sound_input_node_mut(&mut self) -> &mut dyn SoundInputNode<'ctx> {
        &mut self.sound_input
    }

    fn number_input_node_mut(&mut self) -> &mut dyn SoundNumberInputNodeCollection<'ctx> {
        &mut self.number_input
    }

    fn visit_number_inputs(&self, visitor: &mut dyn SoundNumberInputNodeVisitor<'ctx>) {
        self.number_input.visit_number_inputs(visitor);
    }

    fn visit_number_inputs_mut(&mut self, visitor: &mut dyn SoundNumberInputNodeVisitorMut<'ctx>) {
        self.number_input.visit_number_inputs_mut(visitor);
    }

    fn into_droppable(self: Box<Self>) -> Box<dyn 'ctx + Droppable> {
        self
    }
}

pub struct UniqueProcessorNode<'ctx> {
    node: Box<dyn 'ctx + StateGraphNode<'ctx>>,
}

impl<'ctx> UniqueProcessorNode<'ctx> {
    pub(crate) fn new(node: Box<dyn 'ctx + StateGraphNode<'ctx>>) -> UniqueProcessorNode {
        UniqueProcessorNode { node }
    }

    pub(crate) fn id(&self) -> SoundProcessorId {
        self.node.id()
    }

    pub(crate) fn node(&self) -> &dyn StateGraphNode<'ctx> {
        &*self.node
    }

    fn into_box(self) -> Box<dyn 'ctx + StateGraphNode<'ctx>> {
        self.node
    }

    pub(crate) fn node_mut(&mut self) -> &mut dyn StateGraphNode<'ctx> {
        &mut *self.node
    }

    fn step<T: ProcessorState>(
        &mut self,
        timing: &mut InputTiming,
        state: &T,
        dst: &mut SoundChunk,
        ctx: &Context,
        input_id: SoundInputId,
        input_state: AnyData,
    ) -> StreamStatus {
        let ctx = ctx.push_processor_state(state);
        let ctx = ctx.push_input(Some(self.node.id()), input_id, input_state, timing);
        let status = self.node.process_audio(dst, ctx);
        if status == StreamStatus::Done {
            debug_assert!(!timing.is_done());
            timing.mark_as_done();
        }
        status
    }

    fn reset(&mut self) {
        self.node.reset();
    }

    fn visit<F: FnMut(&mut dyn StateGraphNode<'ctx>)>(&mut self, mut f: F) {
        f(&mut *self.node);
    }
}

pub(crate) struct SharedProcessorNodeData<'ctx> {
    node: Box<dyn 'ctx + StateGraphNode<'ctx>>,
    cached_output: SoundChunk, // TODO: generalize to >1 output
    target_inputs: Vec<(SoundInputId, bool)>,
    stream_status: StreamStatus,
}

impl<'ctx> SharedProcessorNodeData<'ctx> {
    fn new(node: Box<dyn 'ctx + StateGraphNode<'ctx>>) -> SharedProcessorNodeData<'ctx> {
        SharedProcessorNodeData {
            node,
            cached_output: SoundChunk::new(),
            target_inputs: Vec::new(),
            stream_status: StreamStatus::Playing,
        }
    }

    pub(crate) fn node(&self) -> &dyn StateGraphNode<'ctx> {
        &*self.node
    }

    pub(crate) fn node_mut(&mut self) -> &mut dyn StateGraphNode<'ctx> {
        &mut *self.node
    }

    fn add_target_input(&mut self, input: SoundInputId) {
        debug_assert!(self.target_inputs.iter().find(|x| x.0 == input).is_none());
        self.target_inputs.push((input, true));
    }

    fn remove_target_input(&mut self, input: SoundInputId) {
        debug_assert_eq!(
            self.target_inputs.iter().filter(|x| x.0 == input).count(),
            1
        );
        self.target_inputs.retain(|(siid, _)| *siid != input);
    }

    fn num_target_inputs(&self) -> usize {
        self.target_inputs.len()
    }

    fn into_unique_node(self) -> UniqueProcessorNode<'ctx> {
        UniqueProcessorNode::new(self.node)
    }
}

pub struct SharedProcessorNode<'ctx> {
    processor_id: SoundProcessorId,
    data: Arc<RwLock<SharedProcessorNodeData<'ctx>>>,
}

impl<'ctx> SharedProcessorNode<'ctx> {
    pub(crate) fn new(node: Box<dyn 'ctx + StateGraphNode<'ctx>>) -> SharedProcessorNode<'ctx> {
        SharedProcessorNode {
            processor_id: node.id(),
            data: Arc::new(RwLock::new(SharedProcessorNodeData::new(node))),
        }
    }

    pub(crate) fn borrow_data<'a>(
        &'a self,
    ) -> impl 'a + Deref<Target = SharedProcessorNodeData<'ctx>> {
        self.data.read()
    }

    pub(crate) fn borrow_data_mut<'a>(
        &'a mut self,
    ) -> impl 'a + DerefMut<Target = SharedProcessorNodeData<'ctx>> {
        self.data.write()
    }

    pub(crate) fn id(&self) -> SoundProcessorId {
        self.processor_id
    }

    pub(crate) fn invoke_externally(&self, scratch_space: &ScratchArena) {
        let mut data = self.data.write();
        let context = Context::new(self.processor_id, scratch_space);
        let &mut SharedProcessorNodeData {
            ref mut node,
            ref mut cached_output,
            ref target_inputs,
            stream_status: _,
        } = &mut *data;
        debug_assert!(target_inputs.len() == 0);
        node.process_audio(cached_output, context);
    }

    fn num_target_inputs(&self) -> usize {
        self.data.read().num_target_inputs()
    }

    pub(crate) fn is_entry_point(&self) -> bool {
        self.num_target_inputs() == 0
    }

    // pub(crate) fn into_unique_node(self) -> Option<UniqueProcessorNode<'ctx>> {
    //     debug_assert!(Arc::strong_count(&self.data) == self.num_target_inputs());
    //     debug_assert!(Arc::weak_count(&self.data) == 0);
    //     match Arc::try_unwrap(self.data) {
    //         Ok(inner_mutex) => Some(inner_mutex.into_inner().into_unique_node()),
    //         Err(_) => None,
    //     }
    // }

    fn step<T: ProcessorState>(
        &mut self,
        timing: &mut InputTiming,
        state: &T,
        dst: &mut SoundChunk,
        ctx: &Context,
        input_id: SoundInputId,
        input_state: AnyData,
    ) -> StreamStatus {
        let mut data = self.data.write();
        let &mut SharedProcessorNodeData {
            ref mut node,
            ref mut cached_output,
            ref mut target_inputs,
            ref mut stream_status,
        } = &mut *data;
        let all_used = target_inputs.iter().all(|(_, used)| *used);
        if all_used {
            // TODO: this processor state likely can never be read. Skip it?
            // See also note about combining processor and input frames in context.rs
            let ctx = ctx.push_processor_state(state);
            let ctx = ctx.push_input(Some(self.processor_id), input_id, input_state, timing);
            *stream_status = node.process_audio(cached_output, ctx);
            for (_target, used) in target_inputs.iter_mut() {
                *used = false;
            }
        }
        *dst = *cached_output;
        let input_used = target_inputs
            .iter_mut()
            .find_map(|(target_id, used)| {
                if *target_id == input_id {
                    Some(used)
                } else {
                    None
                }
            })
            .unwrap();
        debug_assert!(!*input_used);
        *input_used = true;
        *stream_status
    }

    fn reset(&mut self) {
        let mut data = self.data.write();
        data.node.reset();
        for (_target_id, used) in &mut data.target_inputs {
            *used = true;
        }
    }

    pub(crate) fn visit<F: FnMut(&mut dyn StateGraphNode<'ctx>)>(&mut self, mut f: F) {
        f(&mut *self.data.write().node);
    }

    fn into_arc(self) -> Arc<RwLock<SharedProcessorNodeData<'ctx>>> {
        self.data
    }
}

impl<'ctx> Garbage<'ctx> for SharedProcessorNode<'ctx> {
    fn toss(self, chute: &GarbageChute<'ctx>) {
        chute.send_arc(self.into_arc());
    }
}

impl<'ctx> Clone for SharedProcessorNode<'ctx> {
    fn clone(&self) -> Self {
        Self {
            processor_id: self.processor_id.clone(),
            data: Arc::clone(&self.data),
        }
    }
}

pub enum NodeTargetValue<'ctx> {
    Unique(UniqueProcessorNode<'ctx>),
    Shared(SharedProcessorNode<'ctx>),
    Empty,
}

pub struct NodeTarget<'ctx> {
    input_id: SoundInputId,
    key_index: usize,
    timing: InputTiming,
    target: NodeTargetValue<'ctx>,
}

impl<'ctx> NodeTarget<'ctx> {
    pub(crate) fn new<'a>(
        input_id: SoundInputId,
        key_index: usize,
        nodegen: &NodeGen<'a, 'ctx>,
    ) -> NodeTarget<'ctx> {
        NodeTarget {
            input_id,
            key_index,
            timing: InputTiming::default(),
            target: nodegen.allocate_sound_input_node(input_id),
        }
    }

    pub(crate) fn id(&self) -> SoundInputId {
        self.input_id
    }

    pub(crate) fn key_index(&self) -> usize {
        self.key_index
    }

    // TODO: consider hiding inputtiming and publicly re-exposing only those functions which make sense
    pub fn timing(&self) -> &InputTiming {
        &self.timing
    }
    pub fn timing_mut(&mut self) -> &mut InputTiming {
        &mut self.timing
    }

    pub(crate) fn target_id(&self) -> Option<SoundProcessorId> {
        match &self.target {
            NodeTargetValue::Unique(n) => Some(n.id()),
            NodeTargetValue::Shared(n) => Some(n.id()),
            NodeTargetValue::Empty => None,
        }
    }

    pub(crate) fn target(&self) -> &NodeTargetValue<'ctx> {
        &self.target
    }

    pub(crate) fn is_empty(&self) -> bool {
        match self.target {
            NodeTargetValue::Empty => true,
            _ => false,
        }
    }

    pub(crate) fn visit<F: FnMut(&mut dyn StateGraphNode<'ctx>)>(&mut self, f: F) {
        match &mut self.target {
            NodeTargetValue::Unique(node) => node.visit(f),
            NodeTargetValue::Shared(node) => node.visit(f),
            NodeTargetValue::Empty => (),
        }
    }

    pub(crate) fn swap_target(
        &mut self,
        mut target: NodeTargetValue<'ctx>,
    ) -> NodeTargetValue<'ctx> {
        if let NodeTargetValue::Shared(node) = &mut self.target {
            node.borrow_data_mut().remove_target_input(self.input_id);
        }
        std::mem::swap(&mut self.target, &mut target);
        if let NodeTargetValue::Shared(node) = &mut self.target {
            node.borrow_data_mut().add_target_input(self.input_id);
        }
        target
    }

    pub(crate) fn reset(&mut self, sample_offset: usize) {
        self.timing.reset(sample_offset);
        match &mut self.target {
            NodeTargetValue::Unique(node) => node.reset(),
            NodeTargetValue::Shared(node) => node.reset(),
            NodeTargetValue::Empty => (),
        }
    }

    pub(crate) fn step<T: ProcessorState>(
        &mut self,
        state: &T,
        dst: &mut SoundChunk,
        ctx: &Context,
        input_state: AnyData,
    ) -> StreamStatus {
        debug_assert!(!self.timing.needs_reset());
        if self.timing.is_done() {
            dst.silence();
            return StreamStatus::Done;
        }
        let release_pending = self.timing.pending_release().is_some();

        let status = match &mut self.target {
            NodeTargetValue::Unique(node) => node.step(
                &mut self.timing,
                state,
                dst,
                ctx,
                self.input_id,
                input_state,
            ),
            NodeTargetValue::Shared(node) => node.step(
                &mut self.timing,
                state,
                dst,
                ctx,
                self.input_id,
                input_state,
            ),
            NodeTargetValue::Empty => {
                dst.silence();
                self.timing.mark_as_done();
                StreamStatus::Done
            }
        };
        let was_released = self.timing.was_released();
        if release_pending && !was_released {
            self.timing.mark_as_done();
            return StreamStatus::Done;
        }
        status
    }
}

impl<'ctx> Drop for NodeTarget<'ctx> {
    fn drop(&mut self) {
        // Remove input id from shared node target if needed
        // uhhhhhhhhh how to orchestrate this correctly with
        // state graph edits?
        self.swap_target(NodeTargetValue::Empty);
    }
}

impl<'ctx> Garbage<'ctx> for NodeTargetValue<'ctx> {
    fn toss(self, chute: &GarbageChute<'ctx>) {
        match self {
            NodeTargetValue::Unique(n) => chute.send_box(n.into_box().into_droppable()),
            NodeTargetValue::Shared(n) => chute.send_arc(n.into_arc()),
            NodeTargetValue::Empty => (),
        }
    }
}
