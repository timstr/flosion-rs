use std::{
    cell::RefCell,
    ops::{Deref, DerefMut},
    rc::Rc,
    sync::Arc,
};

use super::{
    anydata::AnyData,
    context::Context,
    numberinputnode::{
        NumberInputNodeCollection, NumberInputNodeVisitor, NumberInputNodeVisitorMut,
    },
    scratcharena::ScratchArena,
    soundchunk::SoundChunk,
    soundgraphtopology::SoundGraphTopology,
    soundinput::{InputTiming, SoundInputId},
    soundinputnode::{
        SoundInputNode, SoundInputNodeVisitor, SoundInputNodeVisitorMut, SoundProcessorInput,
    },
    soundprocessor::{
        DynamicSoundProcessor, DynamicSoundProcessorWithId, ProcessorState, SoundProcessorId,
        StateAndTiming, StaticSoundProcessor, StaticSoundProcessorWithId, StreamStatus,
    },
};

pub struct StaticProcessorNode<'ctx, T: StaticSoundProcessor> {
    processor: Arc<StaticSoundProcessorWithId<T>>,
    sound_input: <T::SoundInputType as SoundProcessorInput>::NodeType<'ctx>,
    number_input: T::NumberInputType<'ctx>,
}

impl<'ctx, T: StaticSoundProcessor> StaticProcessorNode<'ctx, T> {
    pub(super) fn new(
        processor: Arc<StaticSoundProcessorWithId<T>>,
        context: &'ctx inkwell::context::Context,
    ) -> Self {
        let sound_input = processor.get_sound_input().make_node();
        let number_input = processor.make_number_inputs(context);
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
    pub(super) fn new(
        processor: &DynamicSoundProcessorWithId<T>,
        context: &'ctx inkwell::context::Context,
    ) -> Self {
        Self {
            id: processor.id(),
            state: StateAndTiming::new(processor.make_state()),
            sound_input: processor.get_sound_input().make_node(),
            number_input: processor.make_number_inputs(context),
        }
    }

    fn reset(&mut self) {
        self.state.reset();
        self.sound_input.flag_for_reset();
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
pub trait StateGraphNode<'ctx> {
    fn id(&self) -> SoundProcessorId;
    fn reset(&mut self);
    fn process_audio(&mut self, dst: &mut SoundChunk, ctx: Context) -> StreamStatus;

    // Used for book-keeping optimizations, e.g. to avoid visiting shared nodes twice
    // and because comparing trait objects (fat pointers) for equality is fraught
    fn address(&self) -> *const ();

    fn sound_input_node_mut(&mut self) -> &mut dyn SoundInputNode<'ctx>;
    fn visit_sound_inputs(&self, visitor: &mut dyn SoundInputNodeVisitor<'ctx>);
    fn visit_sound_inputs_mut(&mut self, visitor: &mut dyn SoundInputNodeVisitorMut<'ctx>);

    fn number_input_node_mut(&mut self) -> &mut dyn NumberInputNodeCollection<'ctx>;
    fn visit_number_inputs(&self, visitor: &mut dyn NumberInputNodeVisitor<'ctx>);
    fn visit_number_inputs_mut(&mut self, visitor: &mut dyn NumberInputNodeVisitorMut<'ctx>);
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

    fn sound_input_node_mut(&mut self) -> &mut dyn SoundInputNode<'ctx> {
        &mut self.sound_input
    }

    fn visit_sound_inputs(&self, visitor: &mut dyn SoundInputNodeVisitor<'ctx>) {
        self.sound_input.visit_inputs(visitor);
    }

    fn visit_sound_inputs_mut(&mut self, visitor: &mut dyn SoundInputNodeVisitorMut<'ctx>) {
        self.sound_input.visit_inputs_mut(visitor);
    }

    fn number_input_node_mut(&mut self) -> &mut dyn NumberInputNodeCollection<'ctx> {
        &mut self.number_input
    }

    fn visit_number_inputs(&self, visitor: &mut dyn NumberInputNodeVisitor<'ctx>) {
        self.number_input.visit_number_inputs(visitor);
    }

    fn visit_number_inputs_mut(&mut self, visitor: &mut dyn NumberInputNodeVisitorMut<'ctx>) {
        self.number_input.visit_number_inputs_mut(visitor);
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

    fn sound_input_node_mut(&mut self) -> &mut dyn SoundInputNode<'ctx> {
        &mut self.sound_input
    }

    fn visit_sound_inputs(&self, visitor: &mut dyn SoundInputNodeVisitor<'ctx>) {
        self.sound_input.visit_inputs(visitor);
    }

    fn visit_sound_inputs_mut(&mut self, visitor: &mut dyn SoundInputNodeVisitorMut<'ctx>) {
        self.sound_input.visit_inputs_mut(visitor);
    }

    fn number_input_node_mut(&mut self) -> &mut dyn NumberInputNodeCollection<'ctx> {
        &mut self.number_input
    }

    fn visit_number_inputs(&self, visitor: &mut dyn NumberInputNodeVisitor<'ctx>) {
        self.number_input.visit_number_inputs(visitor);
    }

    fn visit_number_inputs_mut(&mut self, visitor: &mut dyn NumberInputNodeVisitorMut<'ctx>) {
        self.number_input.visit_number_inputs_mut(visitor);
    }
}

pub(super) struct UniqueProcessorNode<'ctx> {
    node: Box<dyn StateGraphNode<'ctx> + 'ctx>,
}

impl<'ctx> UniqueProcessorNode<'ctx> {
    pub(super) fn new(node: Box<dyn StateGraphNode<'ctx> + 'ctx>) -> UniqueProcessorNode {
        UniqueProcessorNode { node }
    }

    pub(super) fn id(&self) -> SoundProcessorId {
        self.node.id()
    }

    pub(super) fn node(&self) -> &dyn StateGraphNode<'ctx> {
        &*self.node
    }

    pub(super) fn node_mut(&mut self) -> &mut dyn StateGraphNode<'ctx> {
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

pub(super) struct SharedProcessorNodeData<'ctx> {
    node: Box<dyn StateGraphNode<'ctx> + 'ctx>,
    cached_output: SoundChunk, // TODO: generalize to >1 output
    target_inputs: Vec<(SoundInputId, bool)>,
    stream_status: StreamStatus,
}

impl<'ctx> SharedProcessorNodeData<'ctx> {
    fn new(node: Box<dyn StateGraphNode<'ctx> + 'ctx>) -> SharedProcessorNodeData<'ctx> {
        SharedProcessorNodeData {
            node,
            cached_output: SoundChunk::new(),
            target_inputs: Vec::new(),
            stream_status: StreamStatus::Playing,
        }
    }

    pub(super) fn node(&self) -> &dyn StateGraphNode<'ctx> {
        &*self.node
    }

    pub(super) fn node_mut(&mut self) -> &mut dyn StateGraphNode<'ctx> {
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

pub(super) struct SharedProcessorNode<'ctx> {
    processor_id: SoundProcessorId,
    data: Rc<RefCell<SharedProcessorNodeData<'ctx>>>,
}

impl<'ctx> SharedProcessorNode<'ctx> {
    pub(super) fn new(node: Box<dyn StateGraphNode<'ctx> + 'ctx>) -> SharedProcessorNode<'ctx> {
        SharedProcessorNode {
            processor_id: node.id(),
            data: Rc::new(RefCell::new(SharedProcessorNodeData::new(node))),
        }
    }

    pub(super) fn borrow_data<'a>(
        &'a self,
    ) -> impl 'a + Deref<Target = SharedProcessorNodeData<'ctx>> {
        self.data.borrow()
    }

    pub(super) fn borrow_data_mut<'a>(
        &'a mut self,
    ) -> impl 'a + DerefMut<Target = SharedProcessorNodeData<'ctx>> {
        self.data.borrow_mut()
    }

    pub(super) fn id(&self) -> SoundProcessorId {
        self.processor_id
    }

    pub(super) fn visit_inputs(&self, visitor: &mut dyn SoundInputNodeVisitor<'ctx>) {
        self.data.borrow().node.visit_sound_inputs(visitor);
    }

    pub(super) fn visit_inputs_mut(&self, visitor: &mut dyn SoundInputNodeVisitorMut<'ctx>) {
        self.data.borrow_mut().node.visit_sound_inputs_mut(visitor);
    }

    pub(super) fn invoke_externally(
        &self,
        topology: &SoundGraphTopology,
        scratch_space: &ScratchArena,
    ) {
        let mut data = self.data.borrow_mut();
        let context = Context::new(self.processor_id, topology, scratch_space);
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
        self.data.borrow().num_target_inputs()
    }

    pub(super) fn is_entry_point(&self) -> bool {
        self.num_target_inputs() == 0
    }

    pub(super) fn into_unique_node(self) -> Option<UniqueProcessorNode<'ctx>> {
        debug_assert!(Rc::strong_count(&self.data) == self.num_target_inputs());
        debug_assert!(Rc::weak_count(&self.data) == 0);
        match Rc::try_unwrap(self.data) {
            Ok(inner_mutex) => Some(inner_mutex.into_inner().into_unique_node()),
            Err(_) => None,
        }
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
        let mut data = self.data.borrow_mut();
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
        let mut data = self.data.borrow_mut();
        data.node.reset();
        for (_target_id, used) in &mut data.target_inputs {
            *used = true;
        }
    }

    pub(super) fn visit<F: FnMut(&mut dyn StateGraphNode<'ctx>)>(&mut self, mut f: F) {
        f(&mut *self.data.borrow_mut().node);
    }
}

impl<'ctx> Clone for SharedProcessorNode<'ctx> {
    fn clone(&self) -> Self {
        Self {
            processor_id: self.processor_id.clone(),
            data: Rc::clone(&self.data),
        }
    }
}

pub(super) enum NodeTargetValue<'ctx> {
    Unique(UniqueProcessorNode<'ctx>),
    Shared(SharedProcessorNode<'ctx>),
    Empty,
}

pub struct NodeTarget<'ctx> {
    input_id: SoundInputId,
    target: NodeTargetValue<'ctx>,
}

impl<'ctx> NodeTarget<'ctx> {
    pub(super) fn new(input_id: SoundInputId) -> Self {
        Self {
            input_id,
            target: NodeTargetValue::Empty,
        }
    }

    pub(super) fn processor_id(&self) -> Option<SoundProcessorId> {
        match &self.target {
            NodeTargetValue::Unique(n) => Some(n.id()),
            NodeTargetValue::Shared(n) => Some(n.id()),
            NodeTargetValue::Empty => None,
        }
    }

    pub(super) fn target(&self) -> &NodeTargetValue<'ctx> {
        &self.target
    }

    pub(super) fn is_empty(&self) -> bool {
        match self.target {
            NodeTargetValue::Empty => true,
            _ => false,
        }
    }

    pub(super) fn visit<F: FnMut(&mut dyn StateGraphNode<'ctx>)>(&mut self, f: F) {
        match &mut self.target {
            NodeTargetValue::Unique(node) => node.visit(f),
            NodeTargetValue::Shared(node) => node.visit(f),
            NodeTargetValue::Empty => (),
        }
    }

    pub(super) fn set_target(&mut self, target: NodeTargetValue<'ctx>) {
        if let NodeTargetValue::Shared(node) = &mut self.target {
            node.borrow_data_mut().remove_target_input(self.input_id);
        }
        self.target = target;
        if let NodeTargetValue::Shared(node) = &mut self.target {
            node.borrow_data_mut().add_target_input(self.input_id);
        }
    }

    pub(super) fn reset(&mut self) {
        match &mut self.target {
            NodeTargetValue::Unique(node) => node.reset(),
            NodeTargetValue::Shared(node) => node.reset(),
            NodeTargetValue::Empty => (),
        }
    }

    pub(super) fn step<T: ProcessorState>(
        &mut self,
        timing: &mut InputTiming,
        state: &T,
        dst: &mut SoundChunk,
        ctx: &Context,
        input_id: SoundInputId,
        input_state: AnyData,
    ) -> StreamStatus {
        debug_assert!(!timing.needs_reset());
        if timing.is_done() {
            dst.silence();
            return StreamStatus::Done;
        }
        let release_pending = timing.pending_release().is_some();

        let status = match &mut self.target {
            NodeTargetValue::Unique(node) => {
                node.step(timing, state, dst, ctx, input_id, input_state)
            }
            NodeTargetValue::Shared(node) => {
                node.step(timing, state, dst, ctx, input_id, input_state)
            }
            NodeTargetValue::Empty => {
                dst.silence();
                timing.mark_as_done();
                StreamStatus::Done
            }
        };
        let was_released = timing.was_released();
        if release_pending && !was_released {
            timing.mark_as_done();
            return StreamStatus::Done;
        }
        status
    }
}

impl<'ctx> Drop for NodeTarget<'ctx> {
    fn drop(&mut self) {
        // Remove input id from shared node target if needed
        self.set_target(NodeTargetValue::Empty);
    }
}
