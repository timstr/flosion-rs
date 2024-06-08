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
    compiledexpression::{
        CompiledExpression, CompiledExpressionCollection, CompiledExpressionVisitor,
        CompiledExpressionVisitorMut,
    },
    compiledsoundinput::{CompiledSoundInput, SoundProcessorInput},
    garbage::{Droppable, Garbage, GarbageChute},
    nodegen::NodeGen,
    scratcharena::ScratchArena,
};

/// A compiled state graph artefact for a static processor. An Arc to the
/// processor is held, along with a unique copy of its compiled sound
/// inputs and expressions.
pub struct CompiledStaticProcessor<'ctx, T: StaticSoundProcessor> {
    processor: Arc<StaticSoundProcessorWithId<T>>,
    sound_input: <T::SoundInputType as SoundProcessorInput>::NodeType<'ctx>,
    expressions: T::Expressions<'ctx>,
    timing: ProcessorTiming,
}

impl<'ctx, T: StaticSoundProcessor> CompiledStaticProcessor<'ctx, T> {
    pub(crate) fn new<'a>(
        processor: Arc<StaticSoundProcessorWithId<T>>,
        nodegen: &mut NodeGen<'a, 'ctx>,
    ) -> Self {
        let start = Instant::now();
        let sound_input = processor.get_sound_input().make_node(nodegen);
        let expressions = processor.compile_expressions(nodegen);
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

pub struct CompiledDynamicProcessor<'ctx, T: DynamicSoundProcessor> {
    id: SoundProcessorId,
    state: StateAndTiming<T::StateType>,
    sound_input: <T::SoundInputType as SoundProcessorInput>::NodeType<'ctx>,
    expressions: T::Expressions<'ctx>,
}

impl<'ctx, T: DynamicSoundProcessor> CompiledDynamicProcessor<'ctx, T> {
    pub(crate) fn new<'a>(
        processor: &DynamicSoundProcessorWithId<T>,
        nodegen: &mut NodeGen<'a, 'ctx>,
    ) -> Self {
        let start = Instant::now();
        let state = StateAndTiming::new(processor.make_state());
        let sound_input = processor.get_sound_input().make_node(nodegen);
        let expressions = processor.compile_expressions(nodegen);
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

    fn start_over(&mut self) {
        self.state.start_over();
        for t in self.sound_input.targets_mut() {
            t.timing_mut().flag_to_start_over();
        }
        self.expressions
            .visit_expressions_mut(&mut |expr: &mut CompiledExpression<'ctx>| {
                expr.start_over();
            });
    }

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

// TODO: make this not pub
pub trait CompiledSoundProcessor<'ctx>: Sync + Send {
    fn id(&self) -> SoundProcessorId;
    fn start_over(&mut self);
    fn process_audio(&mut self, dst: &mut SoundChunk, ctx: Context) -> StreamStatus;

    // Used for book-keeping optimizations, e.g. to avoid visiting shared nodes twice
    // and because comparing trait objects (fat pointers) for equality is fraught
    fn address(&self) -> *const ();

    fn into_droppable(self: Box<Self>) -> Box<dyn 'ctx + Droppable>;

    fn sound_input(&self) -> &dyn CompiledSoundInput<'ctx>;
    fn sound_input_mut(&mut self) -> &mut dyn CompiledSoundInput<'ctx>;

    fn expressions(&mut self) -> &mut dyn CompiledExpressionCollection<'ctx>;
    fn visit_expressions(&self, visitor: &mut dyn CompiledExpressionVisitor<'ctx>);
    fn visit_expressions_mut(&mut self, visitor: &mut dyn CompiledExpressionVisitorMut<'ctx>);
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

    fn expressions(&mut self) -> &mut dyn CompiledExpressionCollection<'ctx> {
        &mut self.expressions
    }

    fn visit_expressions(&self, visitor: &mut dyn CompiledExpressionVisitor<'ctx>) {
        self.expressions.visit_expressions(visitor);
    }

    fn visit_expressions_mut(&mut self, visitor: &mut dyn CompiledExpressionVisitorMut<'ctx>) {
        self.expressions.visit_expressions_mut(visitor);
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

    fn expressions(&mut self) -> &mut dyn CompiledExpressionCollection<'ctx> {
        &mut self.expressions
    }

    fn visit_expressions(&self, visitor: &mut dyn CompiledExpressionVisitor<'ctx>) {
        self.expressions.visit_expressions(visitor);
    }

    fn visit_expressions_mut(&mut self, visitor: &mut dyn CompiledExpressionVisitorMut<'ctx>) {
        self.expressions.visit_expressions_mut(visitor);
    }

    fn into_droppable(self: Box<Self>) -> Box<dyn 'ctx + Droppable> {
        self
    }
}

pub struct UniqueCompiledSoundProcessor<'ctx> {
    processor: Box<dyn 'ctx + CompiledSoundProcessor<'ctx>>,
}

impl<'ctx> UniqueCompiledSoundProcessor<'ctx> {
    pub(crate) fn new(
        processor: Box<dyn 'ctx + CompiledSoundProcessor<'ctx>>,
    ) -> UniqueCompiledSoundProcessor {
        UniqueCompiledSoundProcessor { processor }
    }

    pub(crate) fn id(&self) -> SoundProcessorId {
        self.processor.id()
    }

    pub(crate) fn processor(&self) -> &dyn CompiledSoundProcessor<'ctx> {
        &*self.processor
    }

    pub(crate) fn processor_mut(&mut self) -> &mut dyn CompiledSoundProcessor<'ctx> {
        &mut *self.processor
    }

    fn into_box(self) -> Box<dyn 'ctx + CompiledSoundProcessor<'ctx>> {
        self.processor
    }

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

    fn start_over(&mut self) {
        self.processor.start_over();
    }

    fn visit<F: FnMut(&mut dyn CompiledSoundProcessor<'ctx>)>(&mut self, mut f: F) {
        f(&mut *self.processor);
    }
}

pub(crate) struct SharedCompiledProcessorData<'ctx> {
    processor: Box<dyn 'ctx + CompiledSoundProcessor<'ctx>>,
    cached_output: SoundChunk, // TODO: generalize to >1 output
    target_inputs: Vec<(SoundInputId, bool)>,
    stream_status: StreamStatus,
}

impl<'ctx> SharedCompiledProcessorData<'ctx> {
    fn new(
        processor: Box<dyn 'ctx + CompiledSoundProcessor<'ctx>>,
    ) -> SharedCompiledProcessorData<'ctx> {
        SharedCompiledProcessorData {
            processor,
            cached_output: SoundChunk::new(),
            target_inputs: Vec::new(),
            stream_status: StreamStatus::Playing,
        }
    }

    pub(crate) fn processor(&self) -> &dyn CompiledSoundProcessor<'ctx> {
        &*self.processor
    }

    pub(crate) fn processor_mut(&mut self) -> &mut dyn CompiledSoundProcessor<'ctx> {
        &mut *self.processor
    }

    pub(crate) fn add_target_input(&mut self, input: SoundInputId) {
        debug_assert!(self.target_inputs.iter().find(|x| x.0 == input).is_none());
        self.target_inputs.push((input, true));
    }

    pub(crate) fn remove_target_input(&mut self, input: SoundInputId) {
        debug_assert_eq!(
            self.target_inputs.iter().filter(|x| x.0 == input).count(),
            1
        );
        self.target_inputs.retain(|(siid, _)| *siid != input);
    }

    fn num_target_inputs(&self) -> usize {
        self.target_inputs.len()
    }

    fn into_unique(self) -> UniqueCompiledSoundProcessor<'ctx> {
        UniqueCompiledSoundProcessor::new(self.processor)
    }
}

pub struct SharedCompiledProcessor<'ctx> {
    processor_id: SoundProcessorId,
    data: Arc<RwLock<SharedCompiledProcessorData<'ctx>>>,
}

impl<'ctx> SharedCompiledProcessor<'ctx> {
    pub(crate) fn new(
        processor: Box<dyn 'ctx + CompiledSoundProcessor<'ctx>>,
    ) -> SharedCompiledProcessor<'ctx> {
        SharedCompiledProcessor {
            processor_id: processor.id(),
            data: Arc::new(RwLock::new(SharedCompiledProcessorData::new(processor))),
        }
    }

    pub(super) fn data(&self) -> Arc<RwLock<SharedCompiledProcessorData<'ctx>>> {
        Arc::clone(&self.data)
    }

    pub(crate) fn borrow_data<'a>(
        &'a self,
    ) -> impl 'a + Deref<Target = SharedCompiledProcessorData<'ctx>> {
        self.data.read()
    }

    pub(crate) fn borrow_data_mut<'a>(
        &'a mut self,
    ) -> impl 'a + DerefMut<Target = SharedCompiledProcessorData<'ctx>> {
        self.data.write()
    }

    pub(crate) fn id(&self) -> SoundProcessorId {
        self.processor_id
    }

    pub(crate) fn invoke_externally(&self, scratch_space: &ScratchArena) {
        let mut data = self.data.write();
        let context = Context::new(self.processor_id, scratch_space);
        let &mut SharedCompiledProcessorData {
            ref mut processor,
            ref mut cached_output,
            ref target_inputs,
            stream_status: _,
        } = &mut *data;
        debug_assert!(target_inputs.len() == 0);
        processor.process_audio(cached_output, context);
    }

    fn num_target_inputs(&self) -> usize {
        self.data.read().num_target_inputs()
    }

    pub(crate) fn is_entry_point(&self) -> bool {
        self.num_target_inputs() == 0
    }

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
        let mut data = self.data.write();
        debug_assert_eq!(
            data.target_inputs
                .iter()
                .filter(|(siid, _was_used)| { *siid == input_id })
                .count(),
            1,
            "Attempted to step a shared compiled processor for a target sound input which is not listed \
            properly in the shared processor's targets."
        );
        let &mut SharedCompiledProcessorData {
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

    fn start_over(&mut self) {
        let mut data = self.data.write();
        data.processor.start_over();
        for (_target_id, used) in &mut data.target_inputs {
            *used = true;
        }
    }

    pub(crate) fn visit<F: FnMut(&mut dyn CompiledSoundProcessor<'ctx>)>(&mut self, mut f: F) {
        f(&mut *self.data.write().processor);
    }

    fn into_arc(self) -> Arc<RwLock<SharedCompiledProcessorData<'ctx>>> {
        self.data
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
            data: Arc::clone(&self.data),
        }
    }
}

pub enum StateGraphNodeValue<'ctx> {
    Unique(UniqueCompiledSoundProcessor<'ctx>),
    Shared(SharedCompiledProcessor<'ctx>),
    Empty,
}

pub struct CompiledSoundInputBranch<'ctx> {
    input_id: SoundInputId,
    branch_id: SoundInputBranchId,
    timing: InputTiming,
    target: StateGraphNodeValue<'ctx>,
}

impl<'ctx> CompiledSoundInputBranch<'ctx> {
    pub(crate) fn new<'a>(
        input_id: SoundInputId,
        branch_id: SoundInputBranchId,
        nodegen: &mut NodeGen<'a, 'ctx>,
    ) -> CompiledSoundInputBranch<'ctx> {
        CompiledSoundInputBranch {
            input_id,
            branch_id,
            timing: InputTiming::default(),
            target: nodegen.compile_sound_input(input_id),
        }
    }

    pub(crate) fn id(&self) -> SoundInputId {
        self.input_id
    }

    pub(crate) fn branch_id(&self) -> SoundInputBranchId {
        self.branch_id
    }

    // TODO: consider hiding inputtiming and publicly re-exposing only those functions which make sense
    pub(crate) fn timing(&self) -> &InputTiming {
        &self.timing
    }
    pub(crate) fn timing_mut(&mut self) -> &mut InputTiming {
        &mut self.timing
    }

    pub(crate) fn target_id(&self) -> Option<SoundProcessorId> {
        match &self.target {
            StateGraphNodeValue::Unique(proc) => Some(proc.id()),
            StateGraphNodeValue::Shared(proc) => Some(proc.id()),
            StateGraphNodeValue::Empty => None,
        }
    }

    pub(crate) fn target(&self) -> &StateGraphNodeValue<'ctx> {
        &self.target
    }

    pub(crate) fn is_empty(&self) -> bool {
        match self.target {
            StateGraphNodeValue::Empty => true,
            _ => false,
        }
    }

    pub(crate) fn visit<F: FnMut(&mut dyn CompiledSoundProcessor<'ctx>)>(&mut self, f: F) {
        match &mut self.target {
            StateGraphNodeValue::Unique(proc) => proc.visit(f),
            StateGraphNodeValue::Shared(proc) => proc.visit(f),
            StateGraphNodeValue::Empty => (),
        }
    }

    pub(crate) fn swap_target(
        &mut self,
        mut target: StateGraphNodeValue<'ctx>,
    ) -> StateGraphNodeValue<'ctx> {
        if let StateGraphNodeValue::Shared(proc) = &mut self.target {
            proc.borrow_data_mut().remove_target_input(self.input_id);
        }
        std::mem::swap(&mut self.target, &mut target);
        if let StateGraphNodeValue::Shared(proc) = &mut self.target {
            proc.borrow_data_mut().add_target_input(self.input_id);
        }
        target
    }

    pub(crate) fn start_over(&mut self, sample_offset: usize) {
        self.timing.start_over(sample_offset);
        match &mut self.target {
            StateGraphNodeValue::Unique(proc) => proc.start_over(),
            StateGraphNodeValue::Shared(proc) => proc.start_over(),
            StateGraphNodeValue::Empty => (),
        }
    }

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
