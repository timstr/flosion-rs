use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
    time::{Duration, Instant},
};

use parking_lot::RwLock;

use crate::core::{
    anydata::AnyData,
    sound::{
        context::{Context, LocalArrayList},
        soundgraphdata::SoundInputBranchId,
        soundinput::{InputTiming, SoundInputId},
        soundprocessor::{
            DynamicSoundProcessor, DynamicSoundProcessorWithId, ProcessorState, ProcessorTiming,
            SoundProcessorId, StateAndTiming, StaticSoundProcessor, StaticSoundProcessorWithId,
            StreamStatus,
        },
    },
    soundchunk::SoundChunk,
    uniqueid::UniqueId,
};

use super::{
    compiledexpression::{CompiledExpression, CompiledExpressionCollection},
    compiledsoundinput::{CompiledSoundInput, SoundProcessorInput},
    garbage::{Droppable, Garbage, GarbageChute},
    scratcharena::ScratchArena,
    soundgraphcompiler::SoundGraphCompiler,
};

/// A compiled static processor for use in the state graph. An Arc to the
/// processor is held, along with a unique copy of its compiled sound
/// inputs and expressions.
pub struct CompiledStaticProcessor<'ctx, T: StaticSoundProcessor> {
    processor: Arc<StaticSoundProcessorWithId<T>>,
    sound_input: <T::SoundInputType as SoundProcessorInput>::NodeType<'ctx>,
    expressions: T::Expressions<'ctx>,
    timing: ProcessorTiming,
}

impl<'ctx, T: StaticSoundProcessor> CompiledStaticProcessor<'ctx, T> {
    /// Compile a new static processor for the state graph
    pub(crate) fn new<'a>(
        processor: Arc<StaticSoundProcessorWithId<T>>,
        compiler: &mut SoundGraphCompiler<'a, 'ctx>,
    ) -> Self {
        let start = Instant::now();
        let sound_input = processor.get_sound_input().make_node(compiler);
        let expressions = processor.compile_expressions(compiler);
        let finish = Instant::now();
        let time_to_compile: Duration = finish - start;
        let time_to_compile_ms = time_to_compile.as_millis();
        if time_to_compile_ms > 10 {
            println!(
                "Compiling static sound processor {} took {} ms",
                processor.id().value(),
                time_to_compile_ms
            );
        }
        Self {
            processor,
            sound_input,
            expressions,
            timing: ProcessorTiming::new(),
        }
    }
}

/// A compiled dynamic processor for use in the state graph. An arc to the
/// processor is held, along with a unique copy of its compiled sound inputs
/// and expressions.
pub struct CompiledDynamicProcessor<'ctx, T: DynamicSoundProcessor> {
    id: SoundProcessorId,
    state: StateAndTiming<T::StateType>,
    sound_input: <T::SoundInputType as SoundProcessorInput>::NodeType<'ctx>,
    expressions: T::Expressions<'ctx>,
}

impl<'ctx, T: DynamicSoundProcessor> CompiledDynamicProcessor<'ctx, T> {
    /// Compile a new dynamic sound processor for the state graph
    pub(crate) fn new<'a>(
        processor: &DynamicSoundProcessorWithId<T>,
        compiler: &mut SoundGraphCompiler<'a, 'ctx>,
    ) -> Self {
        let start = Instant::now();
        let state = StateAndTiming::new(processor.make_state());
        let sound_input = processor.get_sound_input().make_node(compiler);
        let expressions = processor.compile_expressions(compiler);
        let finish = Instant::now();
        let time_to_compile: Duration = finish - start;
        let time_to_compile_ms = time_to_compile.as_millis();
        if time_to_compile_ms > 10 {
            println!(
                "Compiling dynamic sound processor {} took {} ms",
                processor.id().value(),
                time_to_compile_ms
            );
        }
        Self {
            id: processor.id(),
            state,
            sound_input,
            expressions,
        }
    }

    /// Make the compiled processor start over, that is revert its timing
    /// back to zero and cleanly reinitialize all state such that it starts
    /// processing audio from a new clean state when it is next processed.
    fn start_over(&mut self) {
        self.state.start_over();
        for t in self.sound_input.targets_mut() {
            t.timing_mut().flag_to_start_over();
        }
        self.expressions
            .visit_mut(&mut |expr: &mut CompiledExpression<'ctx>| {
                expr.start_over();
            });
    }

    /// Process the next chunk of audio. This calls into the sound processor's
    /// own `DynamicSoundProcessor::process_audio()` method and provides all the
    /// additional timing and compiled sound inputs and expressions that it needs.
    /// The sound processor's stream status is forwarded and indicates whether
    /// the processor intends to keep producing audio or is finished.
    fn process_audio(&mut self, dst: &mut SoundChunk, ctx: Context) -> StreamStatus {
        let status = T::process_audio(
            &mut self.state,
            &mut self.sound_input,
            &mut self.expressions,
            dst,
            ctx,
        );
        self.state.timing.advance_one_chunk();
        status
    }
}

/// Trait for a compiled sound processor living in the state graph, intended
/// to unify both static and dynamic sound processors.
pub(crate) trait CompiledSoundProcessor<'ctx>: Sync + Send {
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
    fn process_audio(&mut self, dst: &mut SoundChunk, ctx: Context) -> StreamStatus;

    /// Used for book-keeping optimizations, e.g. to avoid visiting shared nodes twice
    /// and because comparing trait objects (fat pointers) for equality is fraught
    fn address(&self) -> *const ();

    /// Consume the compiled processor and convert it to something that can be
    /// tossed into a GarbageChute.
    fn into_droppable(self: Box<Self>) -> Box<dyn 'ctx + Droppable>;

    /// Access the processor's compiled sound input
    fn sound_input(&self) -> &dyn CompiledSoundInput<'ctx>;

    /// Mutably access the processor's compiled sound input
    fn sound_input_mut(&mut self) -> &mut dyn CompiledSoundInput<'ctx>;

    /// Access the collection of compiled expressions
    fn expressions(&self) -> &dyn CompiledExpressionCollection<'ctx>;

    /// Mutably access the collection of compiled expressions
    fn expressions_mut(&mut self) -> &mut dyn CompiledExpressionCollection<'ctx>;
}

impl<'ctx, T: StaticSoundProcessor> CompiledSoundProcessor<'ctx>
    for CompiledStaticProcessor<'ctx, T>
{
    fn id(&self) -> SoundProcessorId {
        self.processor.id()
    }

    fn start_over(&mut self) {
        self.timing.start_over();
    }

    fn process_audio(&mut self, dst: &mut SoundChunk, ctx: Context) -> StreamStatus {
        T::process_audio(
            &*self.processor,
            &self.timing,
            &mut self.sound_input,
            &mut self.expressions,
            dst,
            ctx,
        );
        self.timing.advance_one_chunk();
        StreamStatus::Playing
    }

    fn address(&self) -> *const () {
        let ptr: *const CompiledStaticProcessor<T> = self;
        ptr as *const ()
    }

    fn sound_input(&self) -> &dyn CompiledSoundInput<'ctx> {
        &self.sound_input
    }
    fn sound_input_mut(&mut self) -> &mut dyn CompiledSoundInput<'ctx> {
        &mut self.sound_input
    }

    fn expressions(&self) -> &dyn CompiledExpressionCollection<'ctx> {
        &self.expressions
    }

    fn expressions_mut(&mut self) -> &mut dyn CompiledExpressionCollection<'ctx> {
        &mut self.expressions
    }

    fn into_droppable(self: Box<Self>) -> Box<dyn 'ctx + Droppable> {
        self
    }
}

impl<'ctx, T: DynamicSoundProcessor> CompiledSoundProcessor<'ctx>
    for CompiledDynamicProcessor<'ctx, T>
{
    fn id(&self) -> SoundProcessorId {
        self.id
    }

    fn start_over(&mut self) {
        (self as &mut CompiledDynamicProcessor<T>).start_over()
    }

    fn process_audio(&mut self, dst: &mut SoundChunk, ctx: Context) -> StreamStatus {
        (self as &mut CompiledDynamicProcessor<T>).process_audio(dst, ctx)
    }

    fn address(&self) -> *const () {
        let ptr: *const CompiledDynamicProcessor<T> = self;
        ptr as *const ()
    }

    fn sound_input(&self) -> &dyn CompiledSoundInput<'ctx> {
        &self.sound_input
    }
    fn sound_input_mut(&mut self) -> &mut dyn CompiledSoundInput<'ctx> {
        &mut self.sound_input
    }

    fn expressions(&self) -> &dyn CompiledExpressionCollection<'ctx> {
        &self.expressions
    }

    fn expressions_mut(&mut self) -> &mut dyn CompiledExpressionCollection<'ctx> {
        &mut self.expressions
    }

    fn into_droppable(self: Box<Self>) -> Box<dyn 'ctx + Droppable> {
        self
    }
}

/// A compiled sound processor (typically dynamic but could also be static)
/// that is not shared and is not cached. When the processor is called on
/// to produce audio, it does so immediately and produces audio into the
/// given buffer directly.
pub struct UniqueCompiledSoundProcessor<'ctx> {
    processor: Box<dyn 'ctx + CompiledSoundProcessor<'ctx>>,
}

impl<'ctx> UniqueCompiledSoundProcessor<'ctx> {
    /// Creates a new unique compiled sound processor.
    pub(crate) fn new(
        processor: Box<dyn 'ctx + CompiledSoundProcessor<'ctx>>,
    ) -> UniqueCompiledSoundProcessor {
        UniqueCompiledSoundProcessor { processor }
    }

    /// The sound processor's id
    pub(crate) fn id(&self) -> SoundProcessorId {
        self.processor.id()
    }

    /// Access the compiled processor
    pub(crate) fn processor(&self) -> &dyn CompiledSoundProcessor<'ctx> {
        &*self.processor
    }

    /// Converts self into merely a boxed compiled processor
    fn into_box(self) -> Box<dyn 'ctx + CompiledSoundProcessor<'ctx>> {
        self.processor
    }

    /// Process the next chunk of audio
    fn step<T: ProcessorState>(
        &mut self,
        timing: &mut InputTiming,
        state: &T,
        dst: &mut SoundChunk,
        ctx: &Context,
        input_id: SoundInputId,
        input_state: AnyData,
        local_arrays: LocalArrayList,
    ) -> StreamStatus {
        let ctx = ctx.push_processor_state(state, local_arrays);
        let ctx = ctx.push_input(Some(self.processor.id()), input_id, input_state, timing);
        let status = self.processor.process_audio(dst, ctx);
        if status == StreamStatus::Done {
            debug_assert!(!timing.is_done());
            timing.mark_as_done();
        }
        status
    }

    /// Make audio processing start over
    fn start_over(&mut self) {
        self.processor.start_over();
    }
}

/// The internal data that is shared between co-owners of a shared
/// compiled processor. It's here that the caching logic and cached
/// audio processing result lives.
pub(crate) struct SharedCompiledProcessorCache<'ctx> {
    processor: Box<dyn 'ctx + CompiledSoundProcessor<'ctx>>,
    cached_output: SoundChunk, // TODO: generalize to >1 output
    target_inputs: Vec<(SoundInputId, bool)>,
    stream_status: StreamStatus,
}

impl<'ctx> SharedCompiledProcessorCache<'ctx> {
    /// Creates a new cache for a shared compiled processor node.
    fn new(
        processor: Box<dyn 'ctx + CompiledSoundProcessor<'ctx>>,
    ) -> SharedCompiledProcessorCache<'ctx> {
        SharedCompiledProcessorCache {
            processor,
            cached_output: SoundChunk::new(),
            target_inputs: Vec::new(),
            stream_status: StreamStatus::Playing,
        }
    }

    /// Access the compiled processor
    pub(crate) fn processor(&self) -> &dyn CompiledSoundProcessor<'ctx> {
        &*self.processor
    }

    /// Register a new sound input that co-owns the cache. This sound input
    /// will be expected to call on the shared node to process audio in step
    /// with the rest of the group of inputs that own it.
    pub(crate) fn add_target_input(&mut self, input: SoundInputId) {
        debug_assert!(self.target_inputs.iter().find(|x| x.0 == input).is_none());
        self.target_inputs.push((input, true));
    }

    /// Remove a previously-added sound input as a co-owner of the cache.
    pub(crate) fn remove_target_input(&mut self, input: SoundInputId) {
        debug_assert_eq!(
            self.target_inputs.iter().filter(|x| x.0 == input).count(),
            1
        );
        self.target_inputs.retain(|(siid, _)| *siid != input);
    }

    /// The number of sound inputs co-owning the cache
    fn num_target_inputs(&self) -> usize {
        self.target_inputs.len()
    }

    /// Consume the cache and convert it into a unique compiled processor.
    fn into_unique(self) -> UniqueCompiledSoundProcessor<'ctx> {
        UniqueCompiledSoundProcessor::new(self.processor)
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
    cache: Arc<RwLock<SharedCompiledProcessorCache<'ctx>>>,
}

impl<'ctx> SharedCompiledProcessor<'ctx> {
    /// Creates a new shared compiled processor.
    pub(crate) fn new(
        processor: Box<dyn 'ctx + CompiledSoundProcessor<'ctx>>,
    ) -> SharedCompiledProcessor<'ctx> {
        SharedCompiledProcessor {
            processor_id: processor.id(),
            cache: Arc::new(RwLock::new(SharedCompiledProcessorCache::new(processor))),
        }
    }

    /// The sound processor's id
    pub(crate) fn id(&self) -> SoundProcessorId {
        self.processor_id
    }

    /// Access the cache. This obtains a read lock on the cache and thus
    /// may block or cause blocking elsewhere.
    pub(crate) fn borrow_cache<'a>(
        &'a self,
    ) -> impl 'a + Deref<Target = SharedCompiledProcessorCache<'ctx>> {
        self.cache.read()
    }

    /// Mutably access the cache. This obtains a write lock on the cache
    /// and thus may block or cause blocking elsewhere.
    pub(crate) fn borrow_cache_mut<'a>(
        &'a mut self,
    ) -> impl 'a + DerefMut<Target = SharedCompiledProcessorCache<'ctx>> {
        self.cache.write()
    }

    /// Call on the inner sound processor to produce the next chunk of
    /// audio without reference to any sound input. This requires that
    /// `is_entry_point()` returns true, i.e. that there are no sound
    /// inputs co-owning this shared node.
    pub(crate) fn invoke_externally(&self, scratch_space: &ScratchArena) {
        let mut data = self.cache.write();
        let context = Context::new(self.processor_id, scratch_space);
        let &mut SharedCompiledProcessorCache {
            ref mut processor,
            ref mut cached_output,
            ref target_inputs,
            stream_status: _,
        } = &mut *data;
        debug_assert!(target_inputs.len() == 0);
        processor.process_audio(cached_output, context);
    }

    /// The number of sound inputs co-owning this shared processor.
    /// If there are no sound inputs, the shared processor is consired
    /// to be an entry point.
    fn num_target_inputs(&self) -> usize {
        self.cache.read().num_target_inputs()
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
    fn step<T: ProcessorState>(
        &mut self,
        timing: &mut InputTiming,
        state: &T,
        dst: &mut SoundChunk,
        ctx: &Context,
        input_id: SoundInputId,
        input_state: AnyData,
        local_arrays: LocalArrayList,
    ) -> StreamStatus {
        let mut data = self.cache.write();
        debug_assert_eq!(
            data.target_inputs
                .iter()
                .filter(|(siid, _was_used)| { *siid == input_id })
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
            // TODO: this processor state likely can never be read. Skip it?
            // See also note about combining processor and input frames in context.rs
            let ctx = ctx.push_processor_state(state, local_arrays);
            let ctx = ctx.push_input(Some(self.processor_id), input_id, input_state, timing);
            *stream_status = processor.process_audio(cached_output, ctx);
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

    /// Make the processor start over
    fn start_over(&mut self) {
        let mut data = self.cache.write();
        data.processor.start_over();
        for (_target_id, used) in &mut data.target_inputs {
            *used = true;
        }
    }

    /// Consume self and convert into an Arc to the inner shared cached
    fn into_arc(self) -> Arc<RwLock<SharedCompiledProcessorCache<'ctx>>> {
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
    Unique(UniqueCompiledSoundProcessor<'ctx>),

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
    input_id: SoundInputId,
    branch_id: SoundInputBranchId,
    timing: InputTiming,
    target: StateGraphNodeValue<'ctx>,
}

impl<'ctx> CompiledSoundInputBranch<'ctx> {
    /// Compile a new CompiledSoundInputBranch.
    pub(crate) fn new<'a>(
        input_id: SoundInputId,
        branch_id: SoundInputBranchId,
        compiler: &mut SoundGraphCompiler<'a, 'ctx>,
    ) -> CompiledSoundInputBranch<'ctx> {
        CompiledSoundInputBranch {
            input_id,
            branch_id,
            timing: InputTiming::default(),
            target: compiler.compile_sound_input(input_id),
        }
    }

    /// The sound input's id
    pub(crate) fn id(&self) -> SoundInputId {
        self.input_id
    }

    /// The branch id within the sound input
    pub(crate) fn branch_id(&self) -> SoundInputBranchId {
        self.branch_id
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
            proc.borrow_cache_mut().remove_target_input(self.input_id);
        }
        std::mem::swap(&mut self.target, &mut target);
        if let StateGraphNodeValue::Shared(proc) = &mut self.target {
            proc.borrow_cache_mut().add_target_input(self.input_id);
        }
        target
    }

    /// Make audio processing start over. Resets the timing and
    /// regenerates any time-varying state of the inner compiled
    /// processor.
    pub(crate) fn start_over(&mut self, sample_offset: usize) {
        self.timing.start_over(sample_offset);
        match &mut self.target {
            StateGraphNodeValue::Unique(proc) => proc.start_over(),
            StateGraphNodeValue::Shared(proc) => proc.start_over(),
            StateGraphNodeValue::Empty => (),
        }
    }

    /// Process the next chunk of audio
    // TODO: this is a bit of a mess. It should be possible to
    // fold 'state', 'input_state', and  'local_arrays' into one
    // or two calls to push onto the Context stack, and thus
    // removed here. This method really doesn't need to be generic.
    //
    // The nice thing about taking all these arguments is that it
    // ultimately forces sound processors' `process_audio` methods
    // to provide the necessary information to correctly push the
    // audio call stack. Perhaps that can still be achieved with
    // something a bit tidier? Maybe compiled sound inputs can be
    // wrapped in something that demands the same context info
    // before the sound input's 'step' method is available? Hmmmm.
    pub(crate) fn step<T: ProcessorState>(
        &mut self,
        state: &T,
        dst: &mut SoundChunk,
        ctx: &Context,
        input_state: AnyData,
        local_arrays: LocalArrayList,
    ) -> StreamStatus {
        if self.timing.need_to_start_over() {
            // NOTE: implicitly starting over doesn't use any fine timing
            self.start_over(0);
        }
        if self.timing.is_done() {
            dst.silence();
            return StreamStatus::Done;
        }
        let release_pending = self.timing.pending_release().is_some();

        let status = match &mut self.target {
            StateGraphNodeValue::Unique(proc) => proc.step(
                &mut self.timing,
                state,
                dst,
                ctx,
                self.input_id,
                input_state,
                local_arrays,
            ),
            StateGraphNodeValue::Shared(proc) => proc.step(
                &mut self.timing,
                state,
                dst,
                ctx,
                self.input_id,
                input_state,
                local_arrays,
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
