use std::{
    any::Any,
    ops::{Deref, DerefMut},
};

use super::{
    context::Context,
    nodeallocator::NodeAllocator,
    soundchunk::SoundChunk,
    soundinput::SoundInputId,
    soundprocessor::{DynamicSoundProcessor, SoundProcessorId, StaticSoundProcessor, StreamStatus},
    uniqueid::UniqueId,
};

pub trait ProcessorState: 'static + Sync + Send {
    fn state(&self) -> &dyn Any;

    fn is_static(&self) -> bool;

    fn timing(&self) -> Option<&ProcessorTiming>;
}

impl<T: StaticSoundProcessor> ProcessorState for T {
    fn state(&self) -> &dyn Any {
        self
    }

    fn is_static(&self) -> bool {
        true
    }

    fn timing(&self) -> Option<&ProcessorTiming> {
        None
    }
}

impl<T: State> ProcessorState for StateAndTiming<T> {
    fn state(&self) -> &dyn Any {
        (self as &StateAndTiming<T>).state()
    }

    fn is_static(&self) -> bool {
        false
    }

    fn timing(&self) -> Option<&ProcessorTiming> {
        Some((self as &StateAndTiming<T>).timing())
    }
}

impl<T: State> StateAndTiming<T> {
    pub(super) fn new(state: T) -> StateAndTiming<T> {
        StateAndTiming {
            state,
            timing: ProcessorTiming::new(),
        }
    }

    fn reset(&mut self) {
        self.state.reset();
        self.timing.reset();
    }

    pub fn state(&self) -> &T {
        &self.state
    }

    pub fn state_mut(&mut self) -> &mut T {
        &mut self.state
    }

    pub fn timing(&self) -> &ProcessorTiming {
        &self.timing
    }
}

impl<T: State> Deref for StateAndTiming<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.state
    }
}

impl<T: State> DerefMut for StateAndTiming<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.state
    }
}

#[derive(Clone, Copy)]
pub struct AnyData<'a, I: UniqueId> {
    owner_id: I,
    data: &'a dyn Any,
}

impl<'a, I: UniqueId> AnyData<'a, I> {
    pub fn new(owner_id: I, data: &'a dyn Any) -> Self {
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

impl State for () {
    fn reset(&mut self) {}
}

impl SoundInputNode for () {
    fn flag_for_reset(&mut self) {}
}

pub trait SoundProcessorInput {
    type NodeType: SoundInputNode;

    fn make_node(&self, allocator: &NodeAllocator) -> Self::NodeType;
}

// Trait used for automating allocation and reallocation of node inputs
// Not concerned with actual audio processing or providing access to
// said inputs - concrete types will provide those.
pub trait SoundInputNode: Sync + Send {
    fn flag_for_reset(&mut self);

    // TODO: get rid of public node allocator interface
    // TODO:
    // - how to deal with varying numbers of inputs, e.g. singleinput vs. singleinputlist?
    //     - some kind of iterator interface?
    //       e.g.
    //           fn get_inputs(&self) -> impl Iterator<Type=SomethingSomethingGraphNode>;
    //       where node implementations iterate over their allocated inputs.
    //       seems a bit awkward, although an iterator over mutable references could allow
    //       in-place modification somewhat straightforwardly. But this approach also seems
    //       like it will lead to difficulties getting lifetimes right.
    //     - some kind of visitor pattern?
    //       e.g.
    //           fn visit_inputs<F: Fn(&SomethingSomethingGraphNode)>(&self, f: F);
    //       where node implementations call `f` on each of their allocated inputs.
    //       I like this approach and I think I will choose it. The visitor function
    //       can inspect and modify (if a _mut version is provided) allocated notes
    //       at the same time, and lifetimes shouldn't be an issue. Might need to use
    //       a trait object (&dyn Fn ...) for the visitor if this trait needs to be
    //       object safe.
    //     -
}

pub struct ProcessorTiming {
    elapsed_chunks: usize,
}

impl ProcessorTiming {
    fn new() -> ProcessorTiming {
        ProcessorTiming { elapsed_chunks: 0 }
    }

    fn reset(&mut self) {
        self.elapsed_chunks = 0;
    }

    fn advance_one_chunk(&mut self) {
        self.elapsed_chunks += 1;
    }

    pub fn elapsed_chunks(&self) -> usize {
        self.elapsed_chunks
    }
}

pub struct DynamicProcessorNode<T: DynamicSoundProcessor> {
    id: SoundProcessorId,
    state: StateAndTiming<T::StateType>,
    input: <T::InputType as SoundProcessorInput>::NodeType,
}

impl<T: DynamicSoundProcessor> DynamicProcessorNode<T> {
    pub fn new(
        id: SoundProcessorId,
        state: T::StateType,
        inputs: <T::InputType as SoundProcessorInput>::NodeType,
    ) -> Self {
        Self {
            id,
            state: StateAndTiming::new(state),
            input: inputs,
        }
    }

    fn reset(&mut self) {
        self.state.reset();
        self.input.flag_for_reset();
    }

    fn process_audio(&mut self, dst: &mut SoundChunk, ctx: Context) -> StreamStatus {
        let status = T::process_audio(&mut self.state, &mut self.input, dst, ctx);
        self.state.timing.advance_one_chunk();
        status
    }
}

// TODO: implement StaticProcessorNode here to replace existing functionality in
// SoundGraphTopology's static processor cache (at least w.r.t. storing the cached
// audio in the static processor node)

pub trait ProcessorNodeWrapper: Sync + Send {
    fn id(&self) -> SoundProcessorId;
    fn reset(&mut self);
    fn process_audio(&mut self, dst: &mut SoundChunk, ctx: Context) -> StreamStatus;
    fn input_node(&self) -> &dyn SoundInputNode;
    // TODO: input_node_mut when needed for modifying state graph
}

impl<T: DynamicSoundProcessor> ProcessorNodeWrapper for DynamicProcessorNode<T> {
    fn id(&self) -> SoundProcessorId {
        self.id
    }

    fn reset(&mut self) {
        (self as &mut DynamicProcessorNode<T>).reset()
    }

    fn process_audio(&mut self, dst: &mut SoundChunk, ctx: Context) -> StreamStatus {
        (self as &mut DynamicProcessorNode<T>).process_audio(dst, ctx)
    }

    fn input_node(&self) -> &dyn SoundInputNode {
        &self.input
    }
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum StateOwner {
    SoundInput(SoundInputId),
    SoundProcessor(SoundProcessorId),
}

pub trait State: Sync + Send + 'static {
    fn reset(&mut self);
}

pub struct StateAndTiming<T: State> {
    state: T,
    timing: ProcessorTiming,
}
