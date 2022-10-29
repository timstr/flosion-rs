use std::{any::Any, collections::HashMap};

use super::{
    context::Context,
    nodeallocator::NodeAllocator,
    soundchunk::SoundChunk,
    soundinput::SoundInputId,
    soundprocessor::{DynamicSoundProcessor, SoundProcessorId, StateAndTiming, StreamStatus},
    state::State,
    uniqueid::UniqueId,
};

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

// TODO: make this not pub, wrap it in a Option<Box<...>> or Option<Arc<...>> for state
// graph allocation. An enum might be suitable for the different ownership types there.
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

// TODO
// - a struct to represent a group of mutually-synchronous sound processors
// - an algorithm for finding all synchronous groups
// - a struct for allocating nodes within a specific group that stores
//   cached processor nodes (this is context sensitive and cannot apply to the entire topology at once)
// - a processor node implementation for cached processors
// - oh right, multiple processor outputs too (this can wait until after caching, esp. since it requires caching)
// - (from below) in-place modification of sound nodes
//
// How to keep track of potentially multiple copies of the same synchronous group when allocating?
// - in order to allocate shared nodes for cached processors, some reference to already-allocated
//   nodes and/or synchronous groups is needed
// - maybe allocate every node in a synchronous groups at once before visiting any processors of any other groups?
//     - yeeeaaaah then allocate all processor dependencies not already included in the group
//     - a quick proof that this works correctly by treating dependencies in isolation: Assume that processors
//       nodes for multiple dependencious outside a given synchronous group cannot be allocated in isolation.
//       This implies that for a given synchronous group A which depends upon two processors in other groups,
//       those two other processors might be part of the same other group B. But, because both processors
//       are not part of A, they must be depended upon via two separate non-synchronous inputs. Thus, for those
//       two inputs, their dependencies (the two other processors) are not synchronized. Thus, they cannot be
//       in the same (context-dependent) synchronous group. Thus, they can be allocated in complete isolation
//       of one another, and so by contradiction, other processors depended upon by a synchronous group can
//       always be allocated in isolation.
// - This would require me to support modifying the state tree/graph in-place to add pointers to nodes in a specific
//   non-depth-first order, but I want that anyway for preserving all possible audio states when changing connections
//   in the sound graph.
//     - on a slight side note, in-place modification of the state graph will also prove useful for drop-in
//       addition of synchronous sound processors to an existing stream without disrupting playback
//
// - note that there can exist synchronous groups connected to multiple static processors, all of which should
//   be allocated together. In other words, starting at a single static processor and going depth-first is not
//   enough, some kind of queue is needed to track visited nodes (a second queue would also be useful for tracking
//   which nodes from other synchronous groups to visit next as described above)
#[derive(Eq, PartialEq)]
pub(super) struct SynchronousGroupId(usize);

#[derive(Hash, Eq, PartialEq)]
pub(super) struct ProcessorRoute {
    processor: SoundProcessorId,
    via_input: SoundInputId,
}

pub(super) struct SynchronousPartition {
    processors: HashMap<ProcessorRoute, SynchronousGroupId>,
}
