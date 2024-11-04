use std::{ptr::null, sync::Arc};

use send_wrapper::SendWrapper;

use crate::core::{
    engine::garbage::{Droppable, Garbage, GarbageChute},
    expression::context::ExpressionContext,
    jit::jit::FLAG_INITIALIZED,
    samplefrequency::SAMPLE_TIME_STEP,
    soundchunk::CHUNK_SIZE,
};

use super::jit::FLAG_NOT_INITIALIZED;

type EvalExpressionFunc = unsafe extern "C" fn(
    *mut f32,  // pointer to destination array
    usize,     // length of destination array
    f32,       // time step
    *const (), // context
    *mut u8,   // init flag
    *mut f32,  // state variables
);

struct CompiledExpressionData<'ctx> {
    _execution_engine: SendWrapper<inkwell::execution_engine::ExecutionEngine<'ctx>>,
    _function: SendWrapper<inkwell::execution_engine::JitFunction<'ctx, EvalExpressionFunc>>,
    _atomic_captures: Vec<Arc<dyn Sync + Droppable>>,
    num_state_variables: usize,
    raw_function: EvalExpressionFunc,
}

impl<'inkwell_ctx> CompiledExpressionData<'inkwell_ctx> {
    fn new(
        execution_engine: inkwell::execution_engine::ExecutionEngine<'inkwell_ctx>,
        function: inkwell::execution_engine::JitFunction<'inkwell_ctx, EvalExpressionFunc>,
        num_state_variables: usize,
        atomic_captures: Vec<Arc<dyn Sync + Droppable>>,
    ) -> CompiledExpressionData<'inkwell_ctx> {
        // SAFETY: the ExecutionEngine and JitFunction must outlive the
        // raw function pointer. Storing an Arc to both of those ensures
        // this. Storing that Arc further inside of a SendWrapper ensures
        // that the inkwell data can neither be accessed nor dropped on
        // the audio thread.
        let raw_function = unsafe { function.as_raw() };
        CompiledExpressionData {
            _execution_engine: SendWrapper::new(execution_engine),
            _function: SendWrapper::new(function),
            _atomic_captures: atomic_captures,
            num_state_variables,
            raw_function,
        }
    }
}

// Stores the compiled artefact of an expression. Intended to be
// used to create copies of callable functions, not intended to be
// invoked directly. See make_function below.
pub(crate) struct CompiledExpressionArtefact<'ctx> {
    data: Arc<CompiledExpressionData<'ctx>>,
}

impl<'ctx> CompiledExpressionArtefact<'ctx> {
    pub fn new(
        execution_engine: inkwell::execution_engine::ExecutionEngine<'ctx>,
        function: inkwell::execution_engine::JitFunction<'ctx, EvalExpressionFunc>,
        num_state_variables: usize,
        atomic_captures: Vec<Arc<dyn Sync + Droppable>>,
    ) -> CompiledExpressionArtefact<'ctx> {
        CompiledExpressionArtefact {
            data: Arc::new(CompiledExpressionData::new(
                execution_engine,
                function,
                num_state_variables,
                atomic_captures,
            )),
        }
    }

    pub(crate) fn make_function(&self) -> CompiledExpressionFunction<'ctx> {
        let mut state_variables = Vec::new();
        state_variables.resize(self.data.num_state_variables, 0.0);
        CompiledExpressionFunction {
            data: Arc::clone(&self.data),
            function: self.data.raw_function,
            init_flag: FLAG_NOT_INITIALIZED,
            state_variables,
        }
    }
}

pub(crate) struct CompiledExpressionFunction<'ctx> {
    data: Arc<CompiledExpressionData<'ctx>>,
    function: EvalExpressionFunc,
    init_flag: u8,
    state_variables: Vec<f32>,
}

pub enum Discretization {
    None,
    Temporal(f32 /* time step */),
}

impl Discretization {
    pub fn samplewise_temporal() -> Discretization {
        Discretization::Temporal(SAMPLE_TIME_STEP)
    }

    pub fn chunkwise_temporal() -> Discretization {
        Discretization::Temporal(SAMPLE_TIME_STEP * CHUNK_SIZE as f32)
    }

    pub(crate) fn time_step(&self) -> f32 {
        match self {
            Discretization::None => 0.0,
            Discretization::Temporal(dt) => *dt,
        }
    }
}

impl<'ctx> CompiledExpressionFunction<'ctx> {
    pub(crate) fn start_over(&mut self) {
        self.init_flag = FLAG_NOT_INITIALIZED;
    }

    pub(crate) fn eval(
        &mut self,
        dst: &mut [f32],
        context: ExpressionContext,
        discretization: Discretization,
    ) {
        self.eval_impl(dst, Some(context), discretization);
    }

    pub(crate) fn eval_in_test_mode(&mut self, dst: &mut [f32], discretization: Discretization) {
        self.eval_impl(dst, None, discretization);
    }

    fn eval_impl(
        &mut self,
        dst: &mut [f32],
        context: Option<ExpressionContext>,
        discretization: Discretization,
    ) {
        debug_assert!(self.init_flag == FLAG_INITIALIZED || self.init_flag == FLAG_NOT_INITIALIZED);
        let ptr_context: *const ExpressionContext = match context.as_ref() {
            Some(c) => c,
            None => null(),
        };
        let time_step = discretization.time_step();
        let CompiledExpressionFunction {
            data: _,
            function,
            init_flag,
            state_variables,
        } = self;
        let ptr_init_flag: *mut u8 = init_flag;
        let ptr_state_variables: *mut f32 = state_variables.as_mut_ptr();
        unsafe {
            function(
                dst.as_mut_ptr(),
                dst.len(),
                time_step,
                ptr_context as _,
                ptr_init_flag,
                ptr_state_variables,
            );
        }
        debug_assert_eq!(self.init_flag, FLAG_INITIALIZED);
    }
}

impl<'ctx> Garbage<'ctx> for CompiledExpressionFunction<'ctx> {
    fn toss(self, chute: &GarbageChute<'ctx>) {
        chute.send_arc(self.data);
    }
}
