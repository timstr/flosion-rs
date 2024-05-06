use std::{collections::HashMap, sync::Arc};

use atomic_float::AtomicF32;
use inkwell::{
    builder::Builder,
    execution_engine::ExecutionEngine,
    intrinsics::Intrinsic,
    module::Module,
    types::FloatType,
    values::{BasicValue, FloatValue, InstructionValue, IntValue, PointerValue},
    AtomicOrdering,
};

use crate::core::{
    engine::garbage::Droppable,
    number::{
        numbergraphdata::NumberTarget, numbergraphtopology::NumberGraphTopology,
        numberinput::NumberInputId, numbersource::NumberSourceId,
    },
    sound::{
        soundgraphtopology::SoundGraphTopology, soundinput::SoundInputId,
        soundnumberinput::SoundNumberInputId, soundnumbersource::SoundNumberSourceId,
        soundprocessor::SoundProcessorId,
    },
    uniqueid::UniqueId,
};

use super::{
    compilednumberinput::CompiledNumberInput,
    types::JitTypes,
    wrappers::{ArrayReadFunc, ScalarReadFunc, WrapperFunctions},
};

pub(super) const FLAG_NOT_INITIALIZED: u8 = 0;
pub(super) const FLAG_INITIALIZED: u8 = 0;

pub(crate) struct InstructionLocations<'ctx> {
    pub(crate) end_of_entry: InstructionValue<'ctx>,
    pub(crate) end_of_reset: InstructionValue<'ctx>,
    pub(crate) end_of_resume: InstructionValue<'ctx>,
    pub(crate) end_of_pre_loop: InstructionValue<'ctx>,
    pub(crate) end_of_post_loop: InstructionValue<'ctx>,
    pub(crate) end_of_loop: InstructionValue<'ctx>,
}

pub(super) struct LocalVariables<'ctx> {
    pub(super) loop_counter: IntValue<'ctx>,
    pub(super) dst_ptrs: Vec<PointerValue<'ctx>>,
    pub(super) dst_len: IntValue<'ctx>,
    pub(super) context_1: IntValue<'ctx>,
    pub(super) context_2: IntValue<'ctx>,
    pub(super) time_step: FloatValue<'ctx>,
    pub(super) state: PointerValue<'ctx>,
}

pub struct CodeGen<'ctx> {
    pub(crate) instruction_locations: InstructionLocations<'ctx>,
    pub(super) local_variables: LocalVariables<'ctx>,
    pub(crate) types: JitTypes<'ctx>,
    pub(super) wrapper_functions: WrapperFunctions<'ctx>,
    pub(super) builder: Builder<'ctx>,
    pub(super) module: Module<'ctx>,
    execution_engine: ExecutionEngine<'ctx>,
    function_name: String,
    pub(super) atomic_captures: Vec<Arc<dyn Droppable>>,
    pub(super) compiled_targets: HashMap<NumberTarget, FloatValue<'ctx>>,
    num_state_variables: usize,
    state_array_offsets: Vec<(NumberSourceId, usize)>,
}

impl<'ctx> CodeGen<'ctx> {
    pub(crate) fn new(inkwell_context: &'ctx inkwell::context::Context) -> CodeGen<'ctx> {
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

        let fn_eval_number_input_type = types.void_type.fn_type(
            &[
                // *mut f32 : pointer to destination array
                types.f32_pointer_type.into(),
                // usize : length of destination array
                types.usize_type.into(),
                // f32 : time step
                types.f32_type.into(),
                // usize : context 1
                types.usize_type.into(),
                // usize : context 2
                types.usize_type.into(),
                // *mut u8 : pointer to init flag
                types.u8_pointer_type.into(),
                // *mut f32 : pointer to state
                types.f32_pointer_type.into(),
            ],
            false, // is_var_args
        );

        let fn_eval_number_input =
            module.add_function(&function_name, fn_eval_number_input_type, None);

        let bb_entry = inkwell_context.append_basic_block(fn_eval_number_input, "entry");
        let bb_check_reset =
            inkwell_context.append_basic_block(fn_eval_number_input, "check_reset");
        let bb_reset = inkwell_context.append_basic_block(fn_eval_number_input, "reset");
        let bb_resume = inkwell_context.append_basic_block(fn_eval_number_input, "resume");
        let bb_pre_loop = inkwell_context.append_basic_block(fn_eval_number_input, "pre_loop");
        let bb_loop = inkwell_context.append_basic_block(fn_eval_number_input, "loop");
        let bb_post_loop = inkwell_context.append_basic_block(fn_eval_number_input, "post_loop");
        let bb_exit = inkwell_context.append_basic_block(fn_eval_number_input, "exit");

        // read arguments
        let arg_f32_dst_ptr = fn_eval_number_input
            .get_nth_param(0)
            .unwrap()
            .into_pointer_value();
        let arg_dst_len = fn_eval_number_input
            .get_nth_param(1)
            .unwrap()
            .into_int_value();
        let arg_time_step = fn_eval_number_input
            .get_nth_param(2)
            .unwrap()
            .into_float_value();
        let arg_ctx_1 = fn_eval_number_input
            .get_nth_param(3)
            .unwrap()
            .into_int_value();
        let arg_ctx_2 = fn_eval_number_input
            .get_nth_param(4)
            .unwrap()
            .into_int_value();
        let arg_ptr_init_flag = fn_eval_number_input
            .get_nth_param(5)
            .unwrap()
            .into_pointer_value();
        let arg_ptr_state = fn_eval_number_input
            .get_nth_param(6)
            .unwrap()
            .into_pointer_value();

        arg_f32_dst_ptr.set_name("dst_ptr");
        arg_dst_len.set_name("dst_len");
        arg_time_step.set_name("time_step");
        arg_ctx_1.set_name("context_1");
        arg_ctx_2.set_name("context_2");

        let inst_end_of_entry;
        let inst_end_of_reset;
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
            );

            // array read functions and state pointer offsets will be inserted here later

            // if len == 0 { goto exit } else { goto check_reset }
            inst_end_of_entry =
                builder.build_conditional_branch(len_is_zero, bb_exit, bb_check_reset);
        }

        // check_reset
        builder.position_at_end(bb_check_reset);
        {
            // init_flag = *ptr_init_flag
            let init_flag = builder
                .build_load(arg_ptr_init_flag, "init_flag")
                .into_int_value();

            // was_init = init_flag == FLAG_INITIALIZED
            let was_init = builder.build_int_compare(
                inkwell::IntPredicate::EQ,
                init_flag,
                types.u8_type.const_int(FLAG_INITIALIZED as u64, false),
                "was_init",
            );

            // if was_init { goto resume } else { goto reset }
            builder.build_conditional_branch(was_init, bb_resume, bb_reset);
        }

        // reset
        builder.position_at_end(bb_reset);
        {
            // *ptr_init_flag = 1
            builder.build_store(
                arg_ptr_init_flag,
                types.u8_type.const_int(FLAG_INITIALIZED as u64, false),
            );

            // stateful number source init code will be inserted here

            // goto pre_loop
            inst_end_of_reset = builder.build_unconditional_branch(bb_pre_loop);
        }

        // resume
        builder.position_at_end(bb_resume);
        {
            // stateful number source load code will be inserted here

            // goto pre_loop
            inst_end_of_resume = builder.build_unconditional_branch(bb_pre_loop);
        }

        // pre_loop
        builder.position_at_end(bb_pre_loop);
        {
            // stateful number source pre-loop code will be inserted here

            // goto loop
            inst_end_of_pre_loop = builder.build_unconditional_branch(bb_loop);
        }

        // loop
        builder.position_at_end(bb_loop);
        {
            let phi = builder.build_phi(types.usize_type, "loop_counter");
            v_loop_counter = phi.as_basic_value().into_int_value();

            let v_loop_counter_inc = builder.build_int_add(
                v_loop_counter,
                types.usize_type.const_int(1, false),
                "loop_counter_inc",
            );

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
            );

            // loop body will be inserted here

            // if loop_counter >= dst_len { goto post_loop } else { goto loop_body }
            inst_end_of_loop =
                builder.build_conditional_branch(v_loop_counter_ge_len, bb_post_loop, bb_loop);
        }

        // post_loop
        builder.position_at_end(bb_post_loop);
        {
            // stateful number source store and post-loop code will be inserted here

            // goto exit
            inst_end_of_post_loop = builder.build_unconditional_branch(bb_exit);
        }

        // exit
        builder.position_at_end(bb_exit);
        {
            builder.build_return(None);
        }

        let instruction_locations = InstructionLocations {
            end_of_entry: inst_end_of_entry,
            end_of_reset: inst_end_of_reset,
            end_of_resume: inst_end_of_resume,
            end_of_pre_loop: inst_end_of_pre_loop,
            end_of_post_loop: inst_end_of_post_loop,
            end_of_loop: inst_end_of_loop,
        };

        let local_variables = LocalVariables {
            loop_counter: v_loop_counter,
            dst_ptr: arg_f32_dst_ptr,
            dst_len: arg_dst_len,
            context_1: arg_ctx_1,
            context_2: arg_ctx_2,
            time_step: arg_time_step,
            state: arg_ptr_state,
        };

        CodeGen {
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
        }
    }

    pub(super) fn finish(self) -> CompiledNumberInput<'ctx> {
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

            pass_manager_builder.populate_lto_pass_manager(&pass_manager, false, false);
            pass_manager.run_on(&self.module);
        }

        let compiled_fn = match unsafe { self.execution_engine.get_function(&self.function_name) } {
            Ok(f) => f,
            Err(e) => {
                panic!("Unable to run JIT compiler:\n    {:?}", e);
            }
        };

        CompiledNumberInput::new(
            self.execution_engine,
            compiled_fn,
            self.num_state_variables,
            self.atomic_captures,
        )
    }

    fn visit_input(
        &mut self,
        number_input_id: NumberInputId,
        topology: &NumberGraphTopology,
    ) -> FloatValue<'ctx> {
        let input_data = topology.number_input(number_input_id).unwrap();
        match input_data.target() {
            Some(target) => self.visit_target(target, topology),
            None => self
                .types
                .f32_type
                .const_float(input_data.default_value().into()),
        }
    }

    pub(super) fn assign_target(&mut self, target: NumberTarget, value: FloatValue<'ctx>) {
        self.compiled_targets.insert(target, value);
    }

    fn visit_target(
        &mut self,
        target: NumberTarget,
        topology: &NumberGraphTopology,
    ) -> FloatValue<'ctx> {
        if let Some(v) = self.compiled_targets.get(&target) {
            return *v;
        }
        match target {
            NumberTarget::Source(number_source_id) => {
                let source_data = topology.number_source(number_source_id).unwrap();
                let number_source = source_data.instance();

                let input_values: Vec<_> = source_data
                    .number_inputs()
                    .iter()
                    .map(|niid| self.visit_input(*niid, topology))
                    .collect();

                let num_variables = number_source.num_variables();

                let base_state_index = self.num_state_variables;

                self.state_array_offsets
                    .push((number_source_id, self.num_state_variables));
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
                                .build_gep(ptr_all_states, &[offset], "ptr_state")
                        };

                        ptr_state
                    })
                    .collect();

                let v = number_source.compile(self, &input_values, &state_ptrs);

                // self.builder
                //     .position_before(&self.instruction_locations.end_of_bb_loop);
                // let v = number_source.compile_loop(self, &input_values, &state_variables);

                self.compiled_targets
                    .insert(NumberTarget::Source(number_source_id), v);
                v
            }
            NumberTarget::GraphInput(_) => {
                panic!("Missing pre-compiled value for a number graph input")
            }
        }
    }

    pub fn module(&self) -> &Module<'ctx> {
        &self.module
    }

    pub fn builder(&self) -> &Builder<'ctx> {
        &self.builder
    }

    pub fn float_type(&self) -> FloatType<'ctx> {
        self.types.f32_type
    }

    pub fn build_input_scalar_read(
        &mut self,
        input_id: SoundInputId,
        function: ScalarReadFunc,
    ) -> FloatValue<'ctx> {
        self.builder
            .position_before(&self.instruction_locations.end_of_entry);
        let function_addr = self.types.usize_type.const_int(function as u64, false);
        let siid = self
            .types
            .usize_type
            .const_int(input_id.value() as u64, false);
        let call_site_value = self.builder.build_call(
            self.wrapper_functions.input_scalar_read_wrapper,
            &[
                function_addr.into(),
                self.local_variables.context_1.into(),
                self.local_variables.context_2.into(),
                siid.into(),
            ],
            "si_scalar_fn_retv",
        );
        let scalar_read_retv = call_site_value
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_float_value();

        self.builder
            .position_before(&self.instruction_locations.end_of_loop);

        scalar_read_retv
    }

    pub fn build_processor_scalar_read(
        &mut self,
        processor_id: SoundProcessorId,
        function: ScalarReadFunc,
    ) -> FloatValue<'ctx> {
        self.builder
            .position_before(&self.instruction_locations.end_of_entry);
        let function_addr = self.types.usize_type.const_int(function as u64, false);
        let spid = self
            .types
            .usize_type
            .const_int(processor_id.value() as u64, false);
        let call_site_value = self.builder.build_call(
            self.wrapper_functions.processor_scalar_read_wrapper,
            &[
                function_addr.into(),
                self.local_variables.context_1.into(),
                self.local_variables.context_2.into(),
                spid.into(),
            ],
            "sp_scalar_fn_retv",
        );
        let scalar_read_retv = call_site_value
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_float_value();

        self.builder
            .position_before(&self.instruction_locations.end_of_loop);

        scalar_read_retv
    }

    pub fn build_input_array_read(
        &mut self,
        input_id: SoundInputId,
        function: ArrayReadFunc,
    ) -> FloatValue<'ctx> {
        self.builder
            .position_before(&self.instruction_locations.end_of_entry);
        let function_addr = self.types.usize_type.const_int(function as u64, false);
        let siid = self
            .types
            .usize_type
            .const_int(input_id.value() as u64, false);
        let call_site_value = self.builder.build_call(
            self.wrapper_functions.input_array_read_wrapper,
            &[
                function_addr.into(),
                self.local_variables.context_1.into(),
                self.local_variables.context_2.into(),
                siid.into(),
                self.local_variables.dst_len.into(),
            ],
            "si_arr_fn_retv",
        );
        let array_read_retv = call_site_value
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_pointer_value();

        self.builder
            .position_before(&self.instruction_locations.end_of_loop);

        let array_elem_ptr = unsafe {
            self.builder.build_gep(
                array_read_retv,
                &[self.local_variables.loop_counter],
                "array_elem_ptr",
            )
        };
        let array_elem = self.builder.build_load(array_elem_ptr, "array_elem");
        array_elem.into_float_value()
    }

    pub fn build_processor_array_read(
        &mut self,
        processor_id: SoundProcessorId,
        function: ArrayReadFunc,
    ) -> FloatValue<'ctx> {
        self.builder
            .position_before(&self.instruction_locations.end_of_entry);
        let function_addr = self.types.usize_type.const_int(function as u64, false);
        let function_addr =
            self.builder
                .build_int_to_ptr(function_addr, self.types.pointer_type, "function_addr");
        let spid = self
            .types
            .usize_type
            .const_int(processor_id.value() as u64, false);
        let call_site_value = self.builder.build_call(
            self.wrapper_functions.processor_array_read_wrapper,
            &[
                function_addr.into(),
                self.local_variables.context_1.into(),
                self.local_variables.context_2.into(),
                spid.into(),
                self.local_variables.dst_len.into(),
            ],
            "sp_arr_fn_retv",
        );
        let array_read_retv = call_site_value
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_pointer_value();

        self.builder
            .position_before(&self.instruction_locations.end_of_loop);

        let array_elem_ptr = unsafe {
            self.builder.build_gep(
                array_read_retv,
                &[self.local_variables.loop_counter],
                "array_elem_ptr",
            )
        };
        let array_elem = self.builder.build_load(array_elem_ptr, "array_elem");
        array_elem.into_float_value()
    }

    pub fn build_processor_local_array_read(
        &mut self,
        processor_id: SoundProcessorId,
        source_id: SoundNumberSourceId,
    ) -> FloatValue<'ctx> {
        self.builder
            .position_before(&self.instruction_locations.end_of_entry);
        let spid = self
            .types
            .usize_type
            .const_int(processor_id.value() as u64, false);
        let nsid = self
            .types
            .usize_type
            .const_int(source_id.value() as u64, false);
        let call_site_value = self.builder.build_call(
            self.wrapper_functions.processor_local_array_read_wrapper,
            &[
                self.local_variables.context_1.into(),
                self.local_variables.context_2.into(),
                spid.into(),
                nsid.into(),
                self.local_variables.dst_len.into(),
            ],
            "sp_local_arr_fn_retv",
        );
        let local_array_read_retv = call_site_value
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_pointer_value();

        self.builder
            .position_before(&self.instruction_locations.end_of_loop);

        let array_elem_ptr = unsafe {
            self.builder.build_gep(
                local_array_read_retv,
                &[self.local_variables.loop_counter],
                "array_elem_ptr",
            )
        };
        let array_elem = self.builder.build_load(array_elem_ptr, "array_elem");
        array_elem.into_float_value()
    }

    pub fn build_processor_time(&mut self, processor_id: SoundProcessorId) -> FloatValue<'ctx> {
        self.builder
            .position_before(&self.instruction_locations.end_of_entry);
        let spid = self
            .types
            .usize_type
            .const_int(processor_id.value() as u64, false);
        let ptr_time = self.builder.build_alloca(self.types.f32_type, "time");
        let ptr_speed = self.builder.build_alloca(self.types.f32_type, "speed");
        self.builder.build_call(
            self.wrapper_functions.processor_time_wrapper,
            &[
                self.local_variables.context_1.into(),
                self.local_variables.context_2.into(),
                spid.into(),
                ptr_time.into(),
                ptr_speed.into(),
            ],
            "sp_time_retv",
        );
        let time = self.builder.build_load(ptr_time, "time").into_float_value();
        let speed = self
            .builder
            .build_load(ptr_speed, "speed")
            .into_float_value();
        let adjusted_time_step = self.builder.build_float_mul(
            speed,
            self.local_variables.time_step,
            "adjusted_time_step",
        );

        self.builder
            .position_before(&self.instruction_locations.end_of_loop);

        let index_float = self.builder.build_unsigned_int_to_float(
            self.local_variables.loop_counter,
            self.types.f32_type,
            "index_f",
        );

        let time_offset =
            self.builder
                .build_float_mul(index_float, adjusted_time_step, "time_offset");
        let curr_time = self.builder.build_float_add(time, time_offset, "curr_time");

        curr_time
    }

    pub fn build_input_time(&mut self, input_id: SoundInputId) -> FloatValue<'ctx> {
        self.builder
            .position_before(&self.instruction_locations.end_of_entry);
        let siid = self
            .types
            .usize_type
            .const_int(input_id.value() as u64, false);
        let ptr_time = self.builder.build_alloca(self.types.f32_type, "time");
        let ptr_speed = self.builder.build_alloca(self.types.f32_type, "speed");
        self.builder.build_call(
            self.wrapper_functions.input_time_wrapper,
            &[
                self.local_variables.context_1.into(),
                self.local_variables.context_2.into(),
                siid.into(),
                ptr_time.into(),
                ptr_speed.into(),
            ],
            "si_time_retv",
        );
        let time = self.builder.build_load(ptr_time, "time").into_float_value();
        let speed = self
            .builder
            .build_load(ptr_speed, "speed")
            .into_float_value();
        let adjusted_time_step = self.builder.build_float_mul(
            speed,
            self.local_variables.time_step,
            "adjusted_time_step",
        );

        self.builder
            .position_before(&self.instruction_locations.end_of_loop);

        let index_float = self.builder.build_unsigned_int_to_float(
            self.local_variables.loop_counter,
            self.types.f32_type,
            "index_f",
        );

        let time_offset =
            self.builder
                .build_float_mul(index_float, adjusted_time_step, "time_offset");
        let curr_time = self.builder.build_float_add(time, time_offset, "curr_time");

        curr_time
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
            .build_call(decl, &[input.into()], &format!("{}_call", name));

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

        let callsiteval = self.builder.build_call(
            decl,
            &[input1.into(), input2.into()],
            &format!("{}_call", name),
        );

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

        let ptr_val =
            self.builder
                .build_int_to_ptr(addr_val, self.types.f32_pointer_type, "p_atomicf32");
        let load = self.builder.build_load(ptr_val, "atomic32_val");
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

    pub(crate) fn compile_number_input(
        mut self,
        number_input_id: SoundNumberInputId,
        topology: &SoundGraphTopology,
    ) -> CompiledNumberInput<'ctx> {
        let sg_number_input_data = topology.number_input(number_input_id).unwrap();

        let number_topo = sg_number_input_data.number_graph().topology();

        // pre-compile all number graph inputs
        for (giid, snsid) in sg_number_input_data.target_mapping().items() {
            let value = topology
                .number_source(*snsid)
                .unwrap()
                .instance()
                .compile(&mut self);
            self.assign_target(NumberTarget::GraphInput(*giid), value);
        }

        assert!(number_topo.graph_outputs().len() >= 1);

        for output in number_topo.graph_outputs() {
            let output_id = output.id();
            let output_data = number_topo.graph_output(output_id).unwrap();
            let final_value = match output_data.target() {
                Some(target) => self.visit_target(target, number_topo),
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
                    self.local_variables.dst_ptr,
                    &[self.local_variables.loop_counter],
                    "dst_elem_ptr",
                )
            };
            self.builder.build_store(dst_elem_ptr, final_value);
        }

        self.finish()
    }
}
