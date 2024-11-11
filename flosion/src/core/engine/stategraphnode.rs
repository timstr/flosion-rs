use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};

use parking_lot::Mutex;

use crate::core::{
    jit::argumentstack::{ArgumentStack, ArgumentStackView},
    sound::{
        context::{AudioContext, AudioStack},
        soundinput::{InputContext, InputTiming, SoundInputLocation},
        soundprocessor::{
            ProcessorTiming, SoundProcessor, SoundProcessorId, StartOver, StreamStatus,
        },
    },
    soundchunk::SoundChunk,
};

use super::{
    garbage::{Droppable, Garbage, GarbageChute},
    scratcharena::ScratchArena,
};

/// A compiled static processor for use in the state graph.
pub struct CompiledProcessorData<'ctx, T: SoundProcessor> {
    id: SoundProcessorId,
    timing: ProcessorTiming,
    processor: T::CompiledType<'ctx>,
}

impl<'ctx, T: SoundProcessor> CompiledProcessorData<'ctx, T> {
    /// Compile a new static processor for the state graph
    pub(crate) fn new<'a>(
        processor_id: SoundProcessorId,
        processor: T::CompiledType<'ctx>,
    ) -> CompiledProcessorData<'ctx, T> {
        CompiledProcessorData {
            id: processor_id,
            timing: ProcessorTiming::new(),
            processor,
        }
    }

    fn start_over(&mut self) {
        self.timing.start_over();
        self.processor.start_over();
    }

    fn process_audio(
        &mut self,
        dst: &mut SoundChunk,
        stack: AudioStack,
        scratch_arena: &ScratchArena,
        argument_stack: ArgumentStackView,
    ) -> StreamStatus {
        let mut context =
            AudioContext::new(self.id, &self.timing, scratch_arena, argument_stack, stack);
        let status = T::process_audio(&mut self.processor, dst, &mut context);
        self.timing.advance_one_chunk();
        status
    }
}

/// Trait for a compiled sound processor living in the state graph, intended
/// to unify both static and dynamic sound processors.
pub(crate) trait AnyCompiledProcessorData<'ctx>: Send {
    /// The sound processor's id
    fn id(&self) -> SoundProcessorId;

    /// Start over audio processing, that is to reset all timing and reinitialize
    /// all time-varying state. This has no effect for static sound processors,
    /// which represent external stateful resources and thus can't simply be reset
    /// on the fly. If it is possible to somehow 'start over' a given static processor,
    /// the processor itself should provide those facilities through its own API.
    fn start_over(&mut self);

    /// Process the next chunk of audio. This calls into the processor's own
    /// `process_audio()` method, either `DynamicSoundProcessor::process_audio()`
    /// or `StaticSoundProcessor::process_audio()` and provides access to all
    /// the timing and compiled sound inputs and expressions that it needs.
    /// For dynamic processors, the returned stream status is forwarded to indicate
    /// whether it is done processing audio. Static sound processors always keep
    /// producing audio.
    fn process_audio(
        &mut self,
        dst: &mut SoundChunk,
        stack: AudioStack,
        scratch_arena: &ScratchArena,
        argument_stack: ArgumentStackView,
    ) -> StreamStatus;

    /// Used for book-keeping optimizations, e.g. to avoid visiting shared nodes twice
    /// and because comparing trait objects (fat pointers) for equality is fraught
    fn address(&self) -> *const ();

    /// Consume the compiled processor and convert it to something that can be
    /// tossed into a GarbageChute.
    fn into_droppable(self: Box<Self>) -> Box<dyn 'ctx + Droppable>;
}

impl<'ctx, T: 'static + SoundProcessor> AnyCompiledProcessorData<'ctx>
    for CompiledProcessorData<'ctx, T>
{
    fn id(&self) -> SoundProcessorId {
        self.id
    }

    fn start_over(&mut self) {
        CompiledProcessorData::start_over(self);
    }

    fn process_audio(
        &mut self,
        dst: &mut SoundChunk,
        stack: AudioStack,
        scratch_arena: &ScratchArena,
        argument_stack: ArgumentStackView,
    ) -> StreamStatus {
        CompiledProcessorData::process_audio(self, dst, stack, scratch_arena, argument_stack)
    }

    fn address(&self) -> *const () {
        let ptr: *const CompiledProcessorData<T> = self;
        ptr as *const ()
    }

    fn into_droppable(self: Box<Self>) -> Box<dyn 'ctx + Droppable> {
        self
    }
}

/// A compiled sound processor (typically dynamic but could also be static)
/// that is not shared and is not cached. When the processor is called on
/// to produce audio, it does so immediately and produces audio into the
/// given buffer directly.
pub struct UniqueCompiledProcessor<'ctx> {
    processor: Box<dyn 'ctx + AnyCompiledProcessorData<'ctx>>,
}

impl<'ctx> UniqueCompiledProcessor<'ctx> {
    /// Creates a new unique compiled sound processor.
    pub(crate) fn new(
        processor: Box<dyn 'ctx + AnyCompiledProcessorData<'ctx>>,
    ) -> UniqueCompiledProcessor {
        UniqueCompiledProcessor { processor }
    }

    /// The sound processor's id
    pub(crate) fn id(&self) -> SoundProcessorId {
        self.processor.id()
    }

    /// Access the compiled processor
    pub(crate) fn processor(&self) -> &dyn AnyCompiledProcessorData<'ctx> {
        &*self.processor
    }

    /// Converts self into merely a boxed compiled processor
    fn into_box(self) -> Box<dyn 'ctx + AnyCompiledProcessorData<'ctx>> {
        self.processor
    }

    /// Make audio processing start over
    fn start_over(&mut self) {
        self.processor.start_over();
    }

    fn process_audio(
        &mut self,
        dst: &mut SoundChunk,
        stack: AudioStack<'_>,
        scratch_arena: &ScratchArena,
        argument_stack: ArgumentStackView,
    ) -> StreamStatus {
        self.processor
            .process_audio(dst, stack, scratch_arena, argument_stack)
    }
}

/// The internal data that is shared between co-owners of a shared
/// compiled processor. It's here that the caching logic and cached
/// audio processing result lives.
pub(crate) struct SharedCompiledProcessorCache<'ctx> {
    processor: Box<dyn 'ctx + AnyCompiledProcessorData<'ctx>>,
    cached_output: SoundChunk, // TODO: generalize to >1 output
    target_inputs: Vec<(SoundInputLocation, bool)>,
    stream_status: StreamStatus,
}

impl<'ctx> SharedCompiledProcessorCache<'ctx> {
    /// Creates a new cache for a shared compiled processor node.
    fn new(
        processor: Box<dyn 'ctx + AnyCompiledProcessorData<'ctx>>,
    ) -> SharedCompiledProcessorCache<'ctx> {
        SharedCompiledProcessorCache {
            processor,
            cached_output: SoundChunk::new(),
            target_inputs: Vec::new(),
            stream_status: StreamStatus::Playing,
        }
    }

    /// Access the compiled processor
    pub(crate) fn processor(&self) -> &dyn AnyCompiledProcessorData<'ctx> {
        &*self.processor
    }

    /// Register a new sound input that co-owns the cache. This sound input
    /// will be expected to call on the shared node to process audio in step
    /// with the rest of the group of inputs that own it.
    pub(crate) fn add_target_input(&mut self, location: SoundInputLocation) {
        debug_assert!(self
            .target_inputs
            .iter()
            .find(|x| x.0 == location)
            .is_none());
        self.target_inputs.push((location, true));
    }

    /// Remove a previously-added sound input as a co-owner of the cache.
    pub(crate) fn remove_target_input(&mut self, location: SoundInputLocation) {
        debug_assert_eq!(
            self.target_inputs
                .iter()
                .filter(|x| x.0 == location)
                .count(),
            1
        );
        self.target_inputs.retain(|(siid, _)| *siid != location);
    }

    /// The number of sound inputs co-owning the cache
    fn num_target_inputs(&self) -> usize {
        self.target_inputs.len()
    }

    /// Consume the cache and convert it into a unique compiled processor.
    fn into_unique(self) -> UniqueCompiledProcessor<'ctx> {
        UniqueCompiledProcessor::new(self.processor)
    }
}

/// A compiled sound processor (static or dynamic) that can be shared between
/// multiple compiled sound inputs and the state graph's top-level nodes as well.
/// Each separate co-owner of the shared processor is expected to call on it
/// to process audio exactly once as a group, in no particular order. The compiled
/// processor internally processes audio only once and caches the result for the
/// entire group. Once every co-owner has invoked the shared node, its cached
/// result is discarded and it will eagerly process the next chunk of audio the
/// next time it is invoked by any co-owner.
pub struct SharedCompiledProcessor<'ctx> {
    processor_id: SoundProcessorId,
    // NOTE that this may introduce blocking on the audio thread if multiple audio
    // threads are used in the future. Avoiding that will likely require careful
    // scheduling and organization beyond what is possible within the cache itself.
    // Note: using Mutex instead of RwLock because RwLock requires Sync
    // TODO: consider the processor and its compiled artefacts in one Mutex-guarded
    // struct, and the actual cache that may be read in parallel in a different
    // RwLock-guarded struct
    cache: Arc<Mutex<SharedCompiledProcessorCache<'ctx>>>,
}

impl<'ctx> SharedCompiledProcessor<'ctx> {
    /// Creates a new shared compiled processor.
    pub(crate) fn new(
        processor: Box<dyn 'ctx + AnyCompiledProcessorData<'ctx>>,
    ) -> SharedCompiledProcessor<'ctx> {
        SharedCompiledProcessor {
            processor_id: processor.id(),
            cache: Arc::new(Mutex::new(SharedCompiledProcessorCache::new(processor))),
        }
    }

    /// The sound processor's id
    pub(crate) fn id(&self) -> SoundProcessorId {
        self.processor_id
    }

    /// Access the cache. This obtains a lock on the cache and thus
    /// may block or cause blocking elsewhere.
    pub(crate) fn borrow_cache<'a>(
        &'a self,
    ) -> impl 'a + Deref<Target = SharedCompiledProcessorCache<'ctx>> {
        self.cache.lock()
    }

    /// Mutably access the cache. This obtains a lock on the cache
    /// and thus may block or cause blocking elsewhere.
    pub(crate) fn borrow_cache_mut<'a>(
        &'a mut self,
    ) -> impl 'a + DerefMut<Target = SharedCompiledProcessorCache<'ctx>> {
        self.cache.lock()
    }

    pub(crate) fn invoke_externally(
        &self,
        scratch_arena: &ScratchArena,
        argument_stack: &ArgumentStack,
    ) {
        let mut data = self.cache.lock();
        let &mut SharedCompiledProcessorCache {
            ref mut processor,
            ref mut cached_output,
            ref target_inputs,
            stream_status: _,
        } = &mut *data;
        debug_assert!(target_inputs.len() == 0);
        processor.process_audio(
            cached_output,
            AudioStack::Root,
            scratch_arena,
            argument_stack.view_at_bottom(),
        );
    }

    /// The number of sound inputs co-owning this shared processor.
    /// If there are no sound inputs, the shared processor is consired
    /// to be an entry point.
    fn num_target_inputs(&self) -> usize {
        self.cache.lock().num_target_inputs()
    }

    /// Returns whether the shared processor is not co-owned by any
    /// sound inputs and thus is a top-level node into the state graph
    /// through which recursive audio processing can begin.
    pub(crate) fn is_entry_point(&self) -> bool {
        self.num_target_inputs() == 0
    }

    /// Access the cache and retreive the next chunk of processed audio,
    /// which is either cached if other inputs have already called on it
    /// and this one hasn't, or is freshly generated if this is the first
    /// input in the group to call on the shared processor this turn.
    /// All inputs in the group must collectively call on the shared
    /// processor in order to advance it correctly.
    fn process_audio(
        &mut self,
        dst: &mut SoundChunk,
        stack: AudioStack,
        scratch_arena: &ScratchArena,
        argument_stack: ArgumentStackView,
    ) -> StreamStatus {
        let top_frame = stack.top_frame().unwrap();
        let input_location = top_frame.input_location();

        let mut data = self.cache.lock();
        debug_assert_eq!(
            data.target_inputs
                .iter()
                .filter(|(loc, _was_used)| { *loc == input_location })
                .count(),
            1,
            "Attempted to step a shared compiled processor for a target sound input which is not listed \
            properly in the shared processor's targets."
        );
        let &mut SharedCompiledProcessorCache {
            ref mut processor,
            ref mut cached_output,
            ref mut target_inputs,
            ref mut stream_status,
        } = &mut *data;
        let all_used = target_inputs.iter().all(|(_, used)| *used);
        if all_used {
            *stream_status =
                processor.process_audio(cached_output, stack, scratch_arena, argument_stack);
            for (_target, used) in target_inputs.iter_mut() {
                *used = false;
            }
        }
        *dst = *cached_output;
        let input_used = target_inputs
            .iter_mut()
            .find_map(|(target_loc, used)| {
                if *target_loc == input_location {
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

    /// Make the processor start over
    fn start_over(&mut self) {
        let mut data = self.cache.lock();
        data.processor.start_over();
        for (_target_id, used) in &mut data.target_inputs {
            *used = true;
        }
    }

    /// Consume self and convert into an Arc to the inner shared cached
    fn into_arc(self) -> Arc<Mutex<SharedCompiledProcessorCache<'ctx>>> {
        self.cache
    }
}

impl<'ctx> Garbage<'ctx> for SharedCompiledProcessor<'ctx> {
    fn toss(self, chute: &GarbageChute<'ctx>) {
        chute.send_arc(self.into_arc());
    }
}

impl<'ctx> Clone for SharedCompiledProcessor<'ctx> {
    fn clone(&self) -> Self {
        Self {
            processor_id: self.processor_id.clone(),
            cache: Arc::clone(&self.cache),
        }
    }
}

/// The contents of a compiled sound input branch.
pub enum StateGraphNodeValue<'ctx> {
    /// A uniquely owned and directly invocable compiled sound processor
    Unique(UniqueCompiledProcessor<'ctx>),

    /// A co-owned and cached compiled sound processor
    Shared(SharedCompiledProcessor<'ctx>),

    /// No compiled sound processor at all. The input is empty and will
    /// produce silence if invoked.
    Empty,
}

/// CompiledSoundInputBranch combines the possible compiled nodes,
/// timing information, and sound input and branch tracking needed
/// for both invoking a sound input to produce audio within the
/// state graph as well as communicate changes to a concrete sound
/// input type, in terms of adding and removing compiled inputs and branches.
pub struct CompiledSoundInputBranch<'ctx> {
    location: SoundInputLocation,
    timing: InputTiming,
    target: StateGraphNodeValue<'ctx>,
}

impl<'ctx> CompiledSoundInputBranch<'ctx> {
    /// Compile a new CompiledSoundInputBranch.
    pub(crate) fn new<'a>(
        location: SoundInputLocation,
        target: StateGraphNodeValue<'ctx>,
    ) -> CompiledSoundInputBranch<'ctx> {
        // Create empty target first and then swap in the given
        // target, in order to reuse shared caching logic
        let mut compiled_input = CompiledSoundInputBranch {
            location,
            timing: InputTiming::default(),
            target: StateGraphNodeValue::Empty,
        };

        compiled_input.swap_target(target);

        compiled_input
    }

    /// The sound input's location
    pub(crate) fn location(&self) -> SoundInputLocation {
        self.location
    }

    /// Access the input timing
    // TODO: consider hiding inputtiming and publicly re-exposing only those functions which make sense
    pub(crate) fn timing(&self) -> &InputTiming {
        &self.timing
    }
    /// Mutably access the input timing
    pub(crate) fn timing_mut(&mut self) -> &mut InputTiming {
        &mut self.timing
    }

    /// Get the id of the sound processor which the compiled input
    /// is effectively connected to, if any.
    pub(crate) fn target_id(&self) -> Option<SoundProcessorId> {
        match &self.target {
            StateGraphNodeValue::Unique(proc) => Some(proc.id()),
            StateGraphNodeValue::Shared(proc) => Some(proc.id()),
            StateGraphNodeValue::Empty => None,
        }
    }

    /// Access the inner compiled state graph node
    pub(crate) fn target(&self) -> &StateGraphNodeValue<'ctx> {
        &self.target
    }

    /// Replace the inner compiled state graph node with
    /// the given one, and return the old the one. If the
    /// new node is a shared compiled processor, this input
    /// will be added as a co-owner. Symmetrically, if the
    /// node being removed is shared, this input will also
    /// be removed from it.
    pub(crate) fn swap_target(
        &mut self,
        mut target: StateGraphNodeValue<'ctx>,
    ) -> StateGraphNodeValue<'ctx> {
        if let StateGraphNodeValue::Shared(proc) = &mut self.target {
            proc.borrow_cache_mut().remove_target_input(self.location);
        }
        std::mem::swap(&mut self.target, &mut target);
        if let StateGraphNodeValue::Shared(proc) = &mut self.target {
            proc.borrow_cache_mut().add_target_input(self.location);
        }
        target
    }

    /// Make audio processing start over. Resets the timing and
    /// regenerates any time-varying state of the inner compiled
    /// processor.
    pub(crate) fn start_over_at(&mut self, sample_offset: usize) {
        self.timing.start_over(sample_offset);
        match &mut self.target {
            StateGraphNodeValue::Unique(proc) => proc.start_over(),
            StateGraphNodeValue::Shared(proc) => proc.start_over(),
            StateGraphNodeValue::Empty => (),
        }
    }

    /// Process the next chunk of audio
    pub(crate) fn step(&mut self, dst: &mut SoundChunk, ctx: InputContext) -> StreamStatus {
        if self.timing.need_to_start_over() {
            // NOTE: implicitly starting over doesn't use any fine timing
            self.start_over_at(0);
        }
        if self.timing.is_done() {
            dst.silence();
            return StreamStatus::Done;
        }
        let release_pending = self.timing.pending_release().is_some();

        let stack = ctx
            .audio_context()
            .push_frame(self.location.input(), &mut self.timing);

        let status = match &mut self.target {
            StateGraphNodeValue::Unique(proc) => proc.process_audio(
                dst,
                stack,
                ctx.audio_context().scratch_arena(),
                ctx.argument_stack(),
            ),
            StateGraphNodeValue::Shared(proc) => proc.process_audio(
                dst,
                stack,
                ctx.audio_context().scratch_arena(),
                ctx.argument_stack(),
            ),
            StateGraphNodeValue::Empty => {
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

impl<'ctx> Drop for CompiledSoundInputBranch<'ctx> {
    fn drop(&mut self) {
        // Remove input id from shared node target if needed
        // uhhhhhhhhh how to orchestrate this correctly with
        // state graph edits?
        self.swap_target(StateGraphNodeValue::Empty);
    }
}

impl<'ctx> Garbage<'ctx> for StateGraphNodeValue<'ctx> {
    fn toss(self, chute: &GarbageChute<'ctx>) {
        match self {
            StateGraphNodeValue::Unique(proc) => chute.send_box(proc.into_box().into_droppable()),
            StateGraphNodeValue::Shared(proc) => chute.send_arc(proc.into_arc()),
            StateGraphNodeValue::Empty => (),
        }
    }
}
