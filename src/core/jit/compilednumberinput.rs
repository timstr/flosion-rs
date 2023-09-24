use std::sync::Arc;

use atomic_float::AtomicF32;
use send_wrapper::SendWrapper;

use crate::core::{
    engine::garbage::{Garbage, GarbageChute},
    sound::context::Context,
};

type EvalNumberInputFunc = unsafe extern "C" fn(
    *mut f32,  // pointer to destination array
    usize,     // length of destination array
    *const (), // pointer to context
);

struct CompiledNumberInputData<'ctx> {
    _execution_engine: SendWrapper<inkwell::execution_engine::ExecutionEngine<'ctx>>,
    function: SendWrapper<inkwell::execution_engine::JitFunction<'ctx, EvalNumberInputFunc>>,
    atomic_captures: Vec<Arc<AtomicF32>>,
}

impl<'ctx> Drop for CompiledNumberInputData<'ctx> {
    fn drop(&mut self) {
        // Mainly to silence a warning that atomic_captures is unused.
        // It is indeed used to guarantee that pointers to the atomics
        // it may read from stay alive.
        self.atomic_captures.clear();
    }
}

impl<'inkwell_ctx> CompiledNumberInputData<'inkwell_ctx> {
    fn new(
        execution_engine: inkwell::execution_engine::ExecutionEngine<'inkwell_ctx>,
        function: inkwell::execution_engine::JitFunction<'inkwell_ctx, EvalNumberInputFunc>,
        atomic_captures: Vec<Arc<AtomicF32>>,
    ) -> CompiledNumberInputData<'inkwell_ctx> {
        CompiledNumberInputData {
            _execution_engine: SendWrapper::new(execution_engine),
            function: SendWrapper::new(function),
            atomic_captures,
        }
    }
}

pub(crate) struct CompiledNumberInputCache<'ctx> {
    data: Arc<CompiledNumberInputData<'ctx>>,
}

impl<'ctx> CompiledNumberInputCache<'ctx> {
    pub fn new(
        execution_engine: inkwell::execution_engine::ExecutionEngine<'ctx>,
        function: inkwell::execution_engine::JitFunction<'ctx, EvalNumberInputFunc>,
        atomic_captures: Vec<Arc<AtomicF32>>,
    ) -> CompiledNumberInputCache<'ctx> {
        CompiledNumberInputCache {
            data: Arc::new(CompiledNumberInputData::new(
                execution_engine,
                function,
                atomic_captures,
            )),
        }
    }

    pub(crate) fn make_function(&self) -> CompiledNumberInputFunction<'ctx> {
        // SAFETY: the ExecutionEngine and JitFunction must outlive the
        // raw function pointer. Storing an Arc to both of those ensures
        // this. Storing that Arc further inside of a SendWrapper ensures
        // that the inkwell data can neither be accessed nor dropped on
        // the audio thread.
        CompiledNumberInputFunction {
            data: Arc::clone(&self.data),
            function: unsafe { self.data.function.as_raw() },
        }
    }
}

pub(crate) struct CompiledNumberInputFunction<'ctx> {
    // TODO: can stateful number source state be stored here???????
    data: Arc<CompiledNumberInputData<'ctx>>,
    function: EvalNumberInputFunc,
}

impl<'ctx> CompiledNumberInputFunction<'ctx> {
    pub(crate) fn eval(&self, dst: &mut [f32], context: &Context) {
        unsafe {
            let context_ptr: *const () = std::mem::transmute_copy(&context);
            (self.function)(dst.as_mut_ptr(), dst.len(), context_ptr);
        }
    }
}

impl<'ctx> Garbage<'ctx> for CompiledNumberInputFunction<'ctx> {
    fn toss(self, chute: &GarbageChute<'ctx>) {
        chute.send_arc(self.data);
    }
}
