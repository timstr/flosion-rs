use std::{
    collections::HashMap,
    hash::{Hash, Hasher},
    sync::Arc,
};

use atomic_float::AtomicF32;
use inkwell::{
    builder::Builder,
    context::ContextRef,
    execution_engine::ExecutionEngine,
    intrinsics::Intrinsic,
    module::Module,
    types::FloatType,
    values::{BasicValue, FloatValue, InstructionValue, IntValue, PointerValue},
    AtomicOrdering,
};

use crate::core::{
    engine::garbage::Droppable,
    expression::{
        expressiongraph::ExpressionGraph, expressiongraphdata::ExpressionTarget,
        expressionnode::ExpressionNodeId,
    },
    sound::{
        argument::{ProcessorArgumentId, ProcessorArgumentLocation},
        expression::{ExpressionParameterMapping, ExpressionParameterTarget},
        soundgraph::SoundGraph,
        soundinput::SoundInputLocation,
        soundprocessor::SoundProcessorId,
    },
};

use super::{
    compiledexpression::CompiledExpressionArtefact, types::JitTypes, wrappers::WrapperFunctions,
};

pub(super) const FLAG_NOT_INITIALIZED: u8 = 0;
pub(super) const FLAG_INITIALIZED: u8 = 0;

pub(crate) struct InstructionLocations<'ctx> {
    pub(crate) end_of_entry: InstructionValue<'ctx>,
    pub(crate) end_of_startover: InstructionValue<'ctx>,
    pub(crate) end_of_resume: InstructionValue<'ctx>,
    pub(crate) end_of_pre_loop: InstructionValue<'ctx>,
    pub(crate) end_of_post_loop: InstructionValue<'ctx>,
    pub(crate) end_of_loop: InstructionValue<'ctx>,
}

pub struct LocalVariables<'ctx> {
    pub loop_counter: IntValue<'ctx>,
    pub(super) dst_ptr: PointerValue<'ctx>,
    pub(super) dst_len: IntValue<'ctx>,
    pub(super) context_ptr: PointerValue<'ctx>,
    pub time_step: FloatValue<'ctx>,
    pub(super) state: PointerValue<'ctx>,
}

pub struct Jit<'ctx> {
    pub(crate) instruction_locations: InstructionLocations<'ctx>,
    pub(super) local_variables: LocalVariables<'ctx>,
    pub(crate) types: JitTypes<'ctx>,
    pub(super) wrapper_functions: WrapperFunctions<'ctx>,
    pub(super) builder: Builder<'ctx>,
    pub(super) module: Module<'ctx>,
    execution_engine: ExecutionEngine<'ctx>,
    function_name: String,
    pub(super) atomic_captures: Vec<Arc<dyn Sync + Droppable>>,
    pub(super) compiled_targets: HashMap<ExpressionTarget, FloatValue<'ctx>>,
    num_state_variables: usize,
    state_array_offsets: Vec<(ExpressionNodeId, usize)>,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Interval {
    Linear { from: f32, to: f32 },
}

// Darn f32's don't want to implement Eq
impl Eq for Interval {}

// Darn f32's don't want to implement Hash
impl Hash for Interval {
    fn hash<H: Hasher>(&self, state: &mut H) {
        core::mem::discriminant(self).hash(state);
        match self {
            Interval::Linear { from, to } => {
                state.write_u32(from.to_bits());
                state.write_u32(to.to_bits());
            }
        }
    }
}

#[derive(Clone, Copy, Eq, PartialEq, Hash, Debug)]
pub enum ExpressionTestDomain {
    Temporal,
    WithRespectTo(ProcessorArgumentLocation, Interval),
}

#[derive(Clone, Copy, Eq, PartialEq, Hash, Debug)]
pub enum JitMode {
    Normal,
    Test(ExpressionTestDomain),
}

impl<'ctx> Jit<'ctx> {
    pub(crate) fn new(inkwell_context: &'ctx inkwell::context::Context) -> Jit<'ctx> {
        Self::new_inner(inkwell_context).unwrap()
    }

    fn new_inner(
        inkwell_context: &'ctx inkwell::context::Context,
    ) -> Result<Jit<'ctx>, inkwell::builder::BuilderError> {
        let module_name = "flosion_llvm_module";
        let function_name = "flosion_llvm_function".to_string();

        let module = inkwell_context.create_module(module_name);

        // TODO: change optimization level here in release builds?
        let execution_engine = module
            .create_jit_execution_engine(inkwell::OptimizationLevel::None)
            .unwrap();

        let address_space = inkwell::AddressSpace::default();

        let types = JitTypes::new(address_space, &execution_engine, inkwell_context);

        let wrapper_functions = WrapperFunctions::new(&types, &module, &execution_engine);

        let builder = inkwell_context.create_builder();

        let fn_eval_expression_type = types.void_type.fn_type(
            &[
                // *mut f32 : pointer to destination array
                types.pointer_type.into(),
                // usize : length of destination array
                types.usize_type.into(),
                // f32 : time step
                types.f32_type.into(),
                // *mut () : context ptr
                types.pointer_type.into(),
                // *mut u8 : pointer to init flag
                types.pointer_type.into(),
                // *mut f32 : pointer to state
                types.pointer_type.into(),
            ],
            false, // is_var_args
        );

        let fn_eval_expression = module.add_function(&function_name, fn_eval_expression_type, None);

        let bb_entry = inkwell_context.append_basic_block(fn_eval_expression, "entry");
        let bb_check_startover =
            inkwell_context.append_basic_block(fn_eval_expression, "check_startover");
        let bb_startover = inkwell_context.append_basic_block(fn_eval_expression, "startover");
        let bb_resume = inkwell_context.append_basic_block(fn_eval_expression, "resume");
        let bb_pre_loop = inkwell_context.append_basic_block(fn_eval_expression, "pre_loop");
        let bb_loop = inkwell_context.append_basic_block(fn_eval_expression, "loop");
        let bb_post_loop = inkwell_context.append_basic_block(fn_eval_expression, "post_loop");
        let bb_exit = inkwell_context.append_basic_block(fn_eval_expression, "exit");

        // read arguments
        let arg_f32_dst_ptr = fn_eval_expression
            .get_nth_param(0)
            .unwrap()
            .into_pointer_value();
        let arg_dst_len = fn_eval_expression
            .get_nth_param(1)
            .unwrap()
            .into_int_value();
        let arg_time_step = fn_eval_expression
            .get_nth_param(2)
            .unwrap()
            .into_float_value();
        let arg_ctx_ptr = fn_eval_expression
            .get_nth_param(3)
            .unwrap()
            .into_pointer_value();
        let arg_ptr_init_flag = fn_eval_expression
            .get_nth_param(4)
            .unwrap()
            .into_pointer_value();
        let arg_ptr_state = fn_eval_expression
            .get_nth_param(5)
            .unwrap()
            .into_pointer_value();

        arg_f32_dst_ptr.set_name("dst_ptr");
        arg_dst_len.set_name("dst_len");
        arg_time_step.set_name("time_step");
        arg_ctx_ptr.set_name("context_ptr");

        let inst_end_of_entry;
        let inst_end_of_startover;
        let inst_end_of_resume;
        let inst_end_of_pre_loop;
        let inst_end_of_loop;
        let inst_end_of_post_loop;

        let v_loop_counter;

        // entry
        builder.position_at_end(bb_entry);
        {
            // len_is_zero = dst_len == 0
            let len_is_zero = builder.build_int_compare(
                inkwell::IntPredicate::EQ,
                arg_dst_len,
                types.usize_type.const_zero(),
                "len_is_zero",
            )?;

            // array read functions and state pointer offsets will be inserted here later

            // if len == 0 { goto exit } else { goto check_startover }
            inst_end_of_entry =
                builder.build_conditional_branch(len_is_zero, bb_exit, bb_check_startover)?;
        }

        // check_startover
        builder.position_at_end(bb_check_startover);
        {
            // init_flag = *ptr_init_flag
            let init_flag = builder
                .build_load(types.u8_type, arg_ptr_init_flag, "init_flag")?
                .into_int_value();

            // was_init = init_flag == FLAG_INITIALIZED
            let was_init = builder.build_int_compare(
                inkwell::IntPredicate::EQ,
                init_flag,
                types.u8_type.const_int(FLAG_INITIALIZED as u64, false),
                "was_init",
            )?;

            // if was_init { goto resume } else { goto startover }
            builder.build_conditional_branch(was_init, bb_resume, bb_startover)?;
        }

        // startover
        builder.position_at_end(bb_startover);
        {
            // *ptr_init_flag = 1
            builder.build_store(
                arg_ptr_init_flag,
                types.u8_type.const_int(FLAG_INITIALIZED as u64, false),
            )?;

            // stateful expression node init code will be inserted here

            // goto pre_loop
            inst_end_of_startover = builder.build_unconditional_branch(bb_pre_loop)?;
        }

        // resume
        builder.position_at_end(bb_resume);
        {
            // stateful expression node load code will be inserted here

            // goto pre_loop
            inst_end_of_resume = builder.build_unconditional_branch(bb_pre_loop)?;
        }

        // pre_loop
        builder.position_at_end(bb_pre_loop);
        {
            // stateful expression node pre-loop code will be inserted here

            // goto loop
            inst_end_of_pre_loop = builder.build_unconditional_branch(bb_loop)?;
        }

        // loop
        builder.position_at_end(bb_loop);
        {
            let phi = builder.build_phi(types.usize_type, "loop_counter")?;
            v_loop_counter = phi.as_basic_value().into_int_value();

            let v_loop_counter_inc = builder.build_int_add(
                v_loop_counter,
                types.usize_type.const_int(1, false),
                "loop_counter_inc",
            )?;

            phi.add_incoming(&[
                (&types.usize_type.const_zero(), bb_pre_loop),
                (&v_loop_counter_inc, bb_loop),
            ]);

            // check that _next_ loop iteration is in bounds, since
            // loop body is about to be executed any way, and size
            // zero has already been prevented
            let v_loop_counter_ge_len = builder.build_int_compare(
                inkwell::IntPredicate::EQ,
                v_loop_counter_inc,
                arg_dst_len,
                "loop_counter_ge_len",
            )?;

            // loop body will be inserted here

            // if loop_counter >= dst_len { goto post_loop } else { goto loop_body }
            inst_end_of_loop =
                builder.build_conditional_branch(v_loop_counter_ge_len, bb_post_loop, bb_loop)?;
        }

        // post_loop
        builder.position_at_end(bb_post_loop);
        {
            // stateful expression node store and post-loop code will be inserted here

            // goto exit
            inst_end_of_post_loop = builder.build_unconditional_branch(bb_exit)?;
        }

        // exit
        builder.position_at_end(bb_exit);
        {
            builder.build_return(None)?;
        }

        let instruction_locations = InstructionLocations {
            end_of_entry: inst_end_of_entry,
            end_of_startover: inst_end_of_startover,
            end_of_resume: inst_end_of_resume,
            end_of_pre_loop: inst_end_of_pre_loop,
            end_of_post_loop: inst_end_of_post_loop,
            end_of_loop: inst_end_of_loop,
        };

        let local_variables = LocalVariables {
            loop_counter: v_loop_counter,
            dst_ptr: arg_f32_dst_ptr,
            dst_len: arg_dst_len,
            context_ptr: arg_ctx_ptr,
            time_step: arg_time_step,
            state: arg_ptr_state,
        };

        Ok(Jit {
            instruction_locations,
            local_variables,
            types,
            function_name,
            wrapper_functions,
            builder,
            module,
            execution_engine,
            atomic_captures: Vec::new(),
            compiled_targets: HashMap::new(),
            num_state_variables: 0,
            state_array_offsets: Vec::new(),
        })
    }

    pub(super) fn finish(self) -> CompiledExpressionArtefact<'ctx> {
        // let s = self.module().print_to_string();
        // println!("===================== start of module =====================");
        // println!("{}", s.to_str().unwrap());
        // println!("===================== end of module =====================");

        if let Err(s) = self.module().verify() {
            let s = s.to_string();
            println!("LLVM failed to verify IR module");
            for line in s.lines() {
                println!("    {}", line);
            }
            panic!();
        }

        // Apply optimizations in release mode
        #[cfg(not(debug_assertions))]
        {
            use inkwell::{
                passes::{PassManager, PassManagerBuilder},
                OptimizationLevel,
            };

            let pass_manager_builder = PassManagerBuilder::create();

            pass_manager_builder.set_optimization_level(OptimizationLevel::Aggressive);
            // TODO: other optimization options?

            let pass_manager = PassManager::create(());

            pass_manager_builder.populate_module_pass_manager(&pass_manager);
            pass_manager.run_on(&self.module);
        }

        let compiled_fn = match unsafe { self.execution_engine.get_function(&self.function_name) } {
            Ok(f) => f,
            Err(e) => {
                panic!("Unable to run JIT compiler:\n    {:?}", e);
            }
        };

        CompiledExpressionArtefact::new(
            self.execution_engine,
            compiled_fn,
            self.num_state_variables,
            self.atomic_captures,
        )
    }

    fn visit_target(
        &mut self,
        target: ExpressionTarget,
        graph: &ExpressionGraph,
    ) -> FloatValue<'ctx> {
        if let Some(v) = self.compiled_targets.get(&target) {
            return *v;
        }
        match target {
            ExpressionTarget::Node(expr_node_id) => {
                let expr_node_data = graph.node(expr_node_id).unwrap();

                let mut input_values = Vec::new();
                expr_node_data.foreach_input(|input, _| {
                    let input_value = match input.target() {
                        Some(target) => self.visit_target(target, graph),
                        None => self
                            .types
                            .f32_type
                            .const_float(input.default_value().into()),
                    };
                    input_values.push(input_value);
                });

                let num_variables = expr_node_data.num_variables();

                let base_state_index = self.num_state_variables;

                self.state_array_offsets
                    .push((expr_node_id, self.num_state_variables));
                self.num_state_variables += num_variables;

                // Get pointers to state variables in shared state array
                self.builder
                    .position_before(&self.instruction_locations.end_of_entry);
                let state_ptrs: Vec<PointerValue<'ctx>> = (0..num_variables)
                    .map(|i| {
                        let ptr_all_states = self.local_variables.state;
                        let offset = self
                            .types
                            .usize_type
                            .const_int((base_state_index + i) as u64, false);

                        // ptr_state = ptr_all_states + offset
                        let ptr_state = unsafe {
                            self.builder
                                .build_gep(
                                    self.types.f32_type,
                                    ptr_all_states,
                                    &[offset],
                                    "ptr_state",
                                )
                                .unwrap()
                        };

                        ptr_state
                    })
                    .collect();

                let v = expr_node_data.compile(self, &input_values, &state_ptrs);

                self.compiled_targets
                    .insert(ExpressionTarget::Node(expr_node_id), v);
                v
            }
            ExpressionTarget::Parameter(_) => {
                panic!("Missing pre-compiled value for an expression graph parameter")
            }
        }
    }

    pub fn module(&self) -> &Module<'ctx> {
        &self.module
    }

    pub fn builder(&self) -> &Builder<'ctx> {
        &self.builder
    }

    pub fn local_variables(&self) -> &LocalVariables<'ctx> {
        &self.local_variables
    }

    pub fn context(&self) -> ContextRef<'ctx> {
        self.module.get_context()
    }

    // TODO: remove
    pub fn float_type(&self) -> FloatType<'ctx> {
        self.types.f32_type
    }

    pub(crate) fn build_argument_pointer(
        &mut self,
        argument_id: ProcessorArgumentId,
    ) -> PointerValue<'ctx> {
        self.builder
            .position_before(&self.instruction_locations.end_of_entry);
        let arg_id = self
            .types
            .usize_type
            .const_int(argument_id.value() as u64, false);
        let callsiteval = self
            .builder
            .build_call(
                self.wrapper_functions.argument_pointer_wrapper,
                &[self.local_variables.context_ptr.into(), arg_id.into()],
                "arg_ptr_retv",
            )
            .unwrap();
        callsiteval
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_pointer_value()
    }

    fn build_processor_time(&mut self, processor_id: SoundProcessorId) -> FloatValue<'ctx> {
        self.builder
            .position_before(&self.instruction_locations.end_of_entry);
        let spid = self
            .types
            .usize_type
            .const_int(processor_id.value() as u64, false);
        let ptr_time = self
            .builder
            .build_alloca(self.types.f32_type, "time")
            .unwrap();
        let ptr_speed = self
            .builder
            .build_alloca(self.types.f32_type, "speed")
            .unwrap();
        self.builder
            .build_call(
                self.wrapper_functions.processor_time_wrapper,
                &[
                    self.local_variables.context_ptr.into(),
                    spid.into(),
                    ptr_time.into(),
                    ptr_speed.into(),
                ],
                "sp_time_retv",
            )
            .unwrap();
        let time = self
            .builder
            .build_load(self.types.f32_type, ptr_time, "time")
            .unwrap()
            .into_float_value();
        let speed = self
            .builder
            .build_load(self.types.f32_type, ptr_speed, "speed")
            .unwrap()
            .into_float_value();
        let adjusted_time_step = self
            .builder
            .build_float_mul(speed, self.local_variables.time_step, "adjusted_time_step")
            .unwrap();

        self.builder
            .position_before(&self.instruction_locations.end_of_loop);

        let index_float = self
            .builder
            .build_unsigned_int_to_float(
                self.local_variables.loop_counter,
                self.types.f32_type,
                "index_f",
            )
            .unwrap();

        let time_offset = self
            .builder
            .build_float_mul(index_float, adjusted_time_step, "time_offset")
            .unwrap();
        let curr_time = self
            .builder
            .build_float_add(time, time_offset, "curr_time")
            .unwrap();

        curr_time
    }

    fn build_input_time(&mut self, input_location: SoundInputLocation) -> FloatValue<'ctx> {
        self.builder
            .position_before(&self.instruction_locations.end_of_entry);
        let proc_id = self
            .types
            .usize_type
            .const_int(input_location.processor().value() as u64, false);
        let input_id = self
            .types
            .usize_type
            .const_int(input_location.input().value() as u64, false);
        let ptr_time = self
            .builder
            .build_alloca(self.types.f32_type, "time")
            .unwrap();
        let ptr_speed = self
            .builder
            .build_alloca(self.types.f32_type, "speed")
            .unwrap();
        self.builder
            .build_call(
                self.wrapper_functions.input_time_wrapper,
                &[
                    self.local_variables.context_ptr.into(),
                    proc_id.into(),
                    input_id.into(),
                    ptr_time.into(),
                    ptr_speed.into(),
                ],
                "si_time_retv",
            )
            .unwrap();
        let time = self
            .builder
            .build_load(self.types.f32_type, ptr_time, "time")
            .unwrap()
            .into_float_value();
        let speed = self
            .builder
            .build_load(self.types.f32_type, ptr_speed, "speed")
            .unwrap()
            .into_float_value();
        let adjusted_time_step = self
            .builder
            .build_float_mul(speed, self.local_variables.time_step, "adjusted_time_step")
            .unwrap();

        self.builder
            .position_before(&self.instruction_locations.end_of_loop);

        let index_float = self
            .builder
            .build_unsigned_int_to_float(
                self.local_variables.loop_counter,
                self.types.f32_type,
                "index_f",
            )
            .unwrap();

        let time_offset = self
            .builder
            .build_float_mul(index_float, adjusted_time_step, "time_offset")
            .unwrap();
        let curr_time = self
            .builder
            .build_float_add(time, time_offset, "curr_time")
            .unwrap();

        curr_time
    }

    pub fn build_print_str(&mut self, s: &'static str) {
        let str_bytes = s.as_bytes();

        let ptr_char = self
            .types
            .usize_type
            .const_int(str_bytes.as_ptr() as u64, false);
        let ptr_char = self
            .builder
            .build_int_to_ptr(ptr_char, self.types.pointer_type, "ptr_char")
            .unwrap();

        let len = self
            .types
            .usize_type
            .const_int(str_bytes.len() as u64, false);

        self.builder
            .build_call(
                self.wrapper_functions.print_str_wrapper,
                &[ptr_char.into(), len.into()],
                "print_str_call",
            )
            .unwrap();
    }

    pub fn build_print_usize_dec(&mut self, value: IntValue<'ctx>) {
        self.builder
            .build_call(
                self.wrapper_functions.print_usize_dec_wrapper,
                &[value.into()],
                "print_usize_dec_call",
            )
            .unwrap();
    }

    pub fn build_print_usize_hex(&mut self, value: IntValue<'ctx>) {
        self.builder
            .build_call(
                self.wrapper_functions.print_usize_hex_wrapper,
                &[value.into()],
                "print_usize_hex_call",
            )
            .unwrap();
    }

    pub fn build_print_f32(&mut self, value: FloatValue<'ctx>) {
        self.builder
            .build_call(
                self.wrapper_functions.print_f32_wrapper,
                &[value.into()],
                "print_f32_call",
            )
            .unwrap();
    }

    pub fn build_print_ptr(&mut self, value: PointerValue<'ctx>) {
        self.builder
            .build_call(
                self.wrapper_functions.print_ptr_wrapper,
                &[value.into()],
                "print_ptr_call",
            )
            .unwrap();
    }

    pub fn build_unary_intrinsic_call(
        &mut self,
        name: &str,
        input: FloatValue<'ctx>,
    ) -> FloatValue<'ctx> {
        // TODO: error handling
        let intrinsic = Intrinsic::find(name).unwrap();

        let decl = intrinsic.get_declaration(&self.module, &[self.float_type().into()]);

        // TODO: error handling
        let decl = decl.unwrap();

        let callsiteval = self
            .builder
            .build_call(decl, &[input.into()], &format!("{}_call", name))
            .unwrap();

        // TODO: error handling
        callsiteval
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_float_value()
    }

    pub fn build_binary_intrinsic_call(
        &mut self,
        name: &str,
        input1: FloatValue<'ctx>,
        input2: FloatValue<'ctx>,
    ) -> FloatValue<'ctx> {
        // TODO: error handling
        let intrinsic = Intrinsic::find(name).unwrap();

        let decl = intrinsic.get_declaration(&self.module, &[self.float_type().into()]);

        // TODO: error handling
        let decl = decl.unwrap();

        let callsiteval = self
            .builder
            .build_call(
                decl,
                &[input1.into(), input2.into()],
                &format!("{}_call", name),
            )
            .unwrap();

        // TODO: error handling
        callsiteval
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_float_value()
    }

    pub fn build_atomicf32_load(&mut self, value: Arc<AtomicF32>) -> FloatValue<'ctx> {
        let ptr: *const AtomicF32 = &*value;
        let addr_val = self.types.usize_type.const_int(ptr as u64, false);

        // Read the atomic only once before the loop, since it's not
        // expected to change during the loop execution and repeated
        // atomic reads would be wasteful
        self.builder
            .position_before(&self.instruction_locations.end_of_entry);

        let ptr_val = self
            .builder
            .build_int_to_ptr(addr_val, self.types.pointer_type, "p_atomicf32")
            .unwrap();
        let load = self
            .builder
            .build_load(self.types.f32_type, ptr_val, "atomic32_val")
            .unwrap();
        let load_inst = load.as_instruction_value().unwrap();
        load_inst
            .set_atomic_ordering(AtomicOrdering::SequentiallyConsistent)
            .unwrap();

        // Store an Arc to the value to ensure it stays alive
        self.atomic_captures.push(value);

        self.builder
            .position_before(&self.instruction_locations.end_of_loop);

        load.into_float_value()
    }

    pub fn time_step(&self) -> FloatValue<'ctx> {
        self.local_variables.time_step
    }

    fn compile_interval(&mut self, interval: Interval) -> FloatValue<'ctx> {
        match interval {
            Interval::Linear {
                from: interval_begin,
                to: interval_end,
            } => {
                let arr_size_f32 = self
                    .builder()
                    .build_signed_int_to_float(
                        self.local_variables.dst_len,
                        self.types.f32_type,
                        "dst_len_f32",
                    )
                    .unwrap();
                let interval_len_over_arr_size = self
                    .builder()
                    .build_float_div(
                        self.types
                            .f32_type
                            .const_float((interval_end - interval_begin) as _),
                        arr_size_f32,
                        "internal_len_over_arr_size",
                    )
                    .unwrap();
                let loop_counter_f32 = self
                    .builder()
                    .build_signed_int_to_float(
                        self.local_variables.loop_counter,
                        self.types.f32_type,
                        "loop_counter_f32",
                    )
                    .unwrap();
                let interval_val_from_zero = self
                    .builder()
                    .build_float_mul(
                        loop_counter_f32,
                        interval_len_over_arr_size,
                        "interval_val_from_zero",
                    )
                    .unwrap();
                let interval_val = self
                    .builder()
                    .build_float_add(
                        interval_val_from_zero,
                        self.types.f32_type.const_float(interval_begin as _),
                        "interval_val",
                    )
                    .unwrap();
                interval_val
            }
        }
    }

    fn compile_all_parameters(
        &mut self,
        graph: &SoundGraph,
        parameter_mapping: &ExpressionParameterMapping,
        mode: JitMode,
    ) {
        for (param_id, target) in parameter_mapping.items() {
            self.builder()
                .position_before(&self.instruction_locations.end_of_loop);

            let param_value = match mode {
                JitMode::Normal => match target {
                    ExpressionParameterTarget::Argument(arg_location) => {
                        let proc = graph.sound_processor(arg_location.processor()).unwrap();
                        proc.with_processor_argument(arg_location.argument(), |arg| {
                            arg.compile_evaluation(self)
                        })
                        .unwrap()
                    }
                    ExpressionParameterTarget::ProcessorTime(spid) => {
                        self.build_processor_time(*spid)
                    }
                    ExpressionParameterTarget::InputTime(input_loc) => {
                        self.build_input_time(*input_loc)
                    }
                },
                JitMode::Test(test_domain) => {
                    match test_domain {
                        ExpressionTestDomain::Temporal => {
                            // If testing against the time domain,
                            // don't evaluate arguments (they won't
                            // have been pushed) but do no other
                            // work, since the discretization
                            // already handles this.

                            // Return a zero value, since
                            // we have presumably have no access
                            // to the argument values on the
                            // audio thread.
                            self.types.f32_type.const_zero()
                        }
                        ExpressionTestDomain::WithRespectTo(wrt_arg, interval) => {
                            // If testing against a specific parameter,
                            // generate code to produce instrumented values
                            // within the requested range. For the time being,
                            // this assumes that the expression is being
                            // evaluated once over a single array containing
                            // the entire extent being tested over i.e., the
                            // output array for one invocation exactly lines
                            // up with the requested interval.
                            if *target == ExpressionParameterTarget::Argument(wrt_arg) {
                                self.compile_interval(interval)
                            } else {
                                self.types.f32_type.const_zero()
                            }
                        }
                    }
                }
            };

            self.compiled_targets
                .insert(ExpressionTarget::Parameter(*param_id), param_value);
        }
    }

    pub(crate) fn compile_expression(
        mut self,
        expression_graph: &ExpressionGraph,
        parameter_mapping: &ExpressionParameterMapping,
        graph: &SoundGraph,
        mode: JitMode,
    ) -> CompiledExpressionArtefact<'ctx> {
        // pre-compile all expression graph arguments
        self.compile_all_parameters(graph, parameter_mapping, mode);

        // TODO: add support for multiple results
        assert_eq!(expression_graph.results().len(), 1);
        let output_id = expression_graph.results()[0].id();

        let output_data = expression_graph.result(output_id).unwrap();
        let final_value = match output_data.target() {
            Some(target) => self.visit_target(target, expression_graph),
            None => self
                .types
                .f32_type
                .const_float(output_data.default_value() as f64)
                .into(),
        };

        self.builder
            .position_before(&self.instruction_locations.end_of_loop);

        let dst_elem_ptr = unsafe {
            self.builder.build_gep(
                self.types.f32_type,
                self.local_variables.dst_ptr,
                &[self.local_variables.loop_counter],
                "dst_elem_ptr",
            )
        }
        .unwrap();
        self.builder.build_store(dst_elem_ptr, final_value).unwrap();

        self.finish()
    }
}
