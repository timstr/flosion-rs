use std::{fs, path::Path, process::Command, sync::Arc};

use atomic_float::AtomicF32;
use inkwell::{
    builder::Builder,
    intrinsics::Intrinsic,
    module::Module,
    types::{FloatType, IntType, PointerType},
    values::{BasicValue, FloatValue, FunctionValue, InstructionValue, IntValue, PointerValue},
    AddressSpace, AtomicOrdering,
};

use crate::core::uniqueid::UniqueId;

use super::{
    anydata::AnyData, context::Context, numberinput::NumberInputId, numbersource::NumberSourceId,
    samplefrequency::SAMPLE_FREQUENCY, soundgraphtopology::SoundGraphTopology,
    soundinput::SoundInputId, soundprocessor::SoundProcessorId,
};

struct InstructionLocations<'ctx> {
    end_of_bb_entry: InstructionValue<'ctx>,
    end_of_bb_loop: InstructionValue<'ctx>,
}

struct LocalVariables<'ctx> {
    loop_counter: IntValue<'ctx>,
    dst_ptr: PointerValue<'ctx>,
    dst_len: IntValue<'ctx>,
    context_ptr: PointerValue<'ctx>,
}

struct Types<'ctx> {
    pointer_type: PointerType<'ctx>,
    float_type: FloatType<'ctx>,
    float_pointer_type: PointerType<'ctx>,
    usize_type: IntType<'ctx>,
}

struct WrapperFunctions<'ctx> {
    processor_scalar_read_wrapper: FunctionValue<'ctx>,
    input_scalar_read_wrapper: FunctionValue<'ctx>,
    processor_array_read_wrapper: FunctionValue<'ctx>,
    input_array_read_wrapper: FunctionValue<'ctx>,
    processor_time_wrapper: FunctionValue<'ctx>,
}

pub struct CodeGen<'ctx> {
    instruction_locations: InstructionLocations<'ctx>,
    local_variables: LocalVariables<'ctx>,
    types: Types<'ctx>,
    wrapper_functions: WrapperFunctions<'ctx>,
    builder: Builder<'ctx>,
    module: Module<'ctx>,
    atomic_captures: Vec<Arc<AtomicF32>>,
}

impl<'ctx> CodeGen<'ctx> {
    fn new(
        basic_blocks: InstructionLocations<'ctx>,
        local_variables: LocalVariables<'ctx>,
        types: Types<'ctx>,
        wrapper_functions: WrapperFunctions<'ctx>,
        builder: Builder<'ctx>,
        module: Module<'ctx>,
    ) -> CodeGen<'ctx> {
        CodeGen {
            instruction_locations: basic_blocks,
            local_variables,
            types,
            wrapper_functions,
            builder,
            module,
            atomic_captures: Vec::new(),
        }
    }

    fn visit_input(
        &mut self,
        number_input_id: NumberInputId,
        topology: &SoundGraphTopology,
    ) -> FloatValue<'ctx> {
        let input_data = topology.number_input(number_input_id).unwrap();
        match input_data.target() {
            Some(nsid) => self.visit_source(nsid, topology),
            None => self
                .types
                .float_type
                .const_float(input_data.default_value().into()),
        }
    }

    fn visit_source(
        &mut self,
        number_source_id: NumberSourceId,
        topology: &SoundGraphTopology,
    ) -> FloatValue<'ctx> {
        let source_data = topology.number_source(number_source_id).unwrap();
        // TODO: consider caching number inputs to avoid generating any
        // a second time
        let input_values: Vec<_> = source_data
            .inputs()
            .iter()
            .map(|niid| self.visit_input(*niid, topology))
            .collect();
        source_data.instance().compile(self, &input_values)
    }

    pub fn module(&self) -> &Module<'ctx> {
        &self.module
    }

    pub fn builder(&self) -> &Builder<'ctx> {
        &self.builder
    }

    pub fn float_type(&self) -> FloatType<'ctx> {
        self.types.float_type
    }

    pub fn build_input_scalar_read(
        &mut self,
        input_id: SoundInputId,
        function: ScalarReadFunc,
    ) -> FloatValue<'ctx> {
        self.builder
            .position_before(&self.instruction_locations.end_of_bb_entry);
        let function_addr = self.types.usize_type.const_int(function as u64, false);
        let siid = self
            .types
            .usize_type
            .const_int(input_id.value() as u64, false);
        let call_site_value = self.builder.build_call(
            self.wrapper_functions.input_scalar_read_wrapper,
            &[
                function_addr.into(),
                self.local_variables.context_ptr.into(),
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
            .position_before(&self.instruction_locations.end_of_bb_loop);

        scalar_read_retv
    }

    pub fn build_processor_scalar_read(
        &mut self,
        processor_id: SoundProcessorId,
        function: ScalarReadFunc,
    ) -> FloatValue<'ctx> {
        self.builder
            .position_before(&self.instruction_locations.end_of_bb_entry);
        let function_addr = self.types.usize_type.const_int(function as u64, false);
        let spid = self
            .types
            .usize_type
            .const_int(processor_id.value() as u64, false);
        let call_site_value = self.builder.build_call(
            self.wrapper_functions.processor_scalar_read_wrapper,
            &[
                function_addr.into(),
                self.local_variables.context_ptr.into(),
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
            .position_before(&self.instruction_locations.end_of_bb_loop);

        scalar_read_retv
    }

    pub fn build_input_array_read(
        &mut self,
        input_id: SoundInputId,
        function: ArrayReadFunc,
    ) -> FloatValue<'ctx> {
        self.builder
            .position_before(&self.instruction_locations.end_of_bb_entry);
        let function_addr = self.types.usize_type.const_int(function as u64, false);
        let siid = self
            .types
            .usize_type
            .const_int(input_id.value() as u64, false);
        let call_site_value = self.builder.build_call(
            self.wrapper_functions.input_array_read_wrapper,
            &[
                function_addr.into(),
                self.local_variables.context_ptr.into(),
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
            .position_before(&self.instruction_locations.end_of_bb_loop);
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
            .position_before(&self.instruction_locations.end_of_bb_entry);
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
                self.local_variables.context_ptr.into(),
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
            .position_before(&self.instruction_locations.end_of_bb_loop);
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

    pub fn build_processor_time(&mut self, processor_id: SoundProcessorId) -> FloatValue<'ctx> {
        self.builder
            .position_before(&self.instruction_locations.end_of_bb_entry);
        let spid = self
            .types
            .usize_type
            .const_int(processor_id.value() as u64, false);
        let ptr_time = self.builder.build_alloca(self.types.float_type, "time");
        let ptr_speed = self.builder.build_alloca(self.types.float_type, "speed");
        self.builder.build_call(
            self.wrapper_functions.processor_time_wrapper,
            &[
                self.local_variables.context_ptr.into(),
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

        self.builder
            .position_before(&self.instruction_locations.end_of_bb_loop);

        let index_float = self.builder.build_unsigned_int_to_float(
            self.local_variables.loop_counter,
            self.types.float_type,
            "index_f",
        );

        let time_offset = self
            .builder
            .build_float_mul(index_float, speed, "time_offset");
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

    pub fn build_atomicf32_load(&mut self, value: Arc<AtomicF32>) -> FloatValue<'ctx> {
        let ptr: *const AtomicF32 = &*value;
        let addr_val = self.types.usize_type.const_int(ptr as u64, false);

        // Read the atomic only once before the loop, since it's not
        // expected to change during the loop execution and repeated
        // atomic reads would be wasteful
        self.builder
            .position_before(&self.instruction_locations.end_of_bb_entry);

        let ptr_val =
            self.builder
                .build_int_to_ptr(addr_val, self.types.float_pointer_type, "p_atomicf32");
        let load = self.builder.build_load(ptr_val, "atomic32_val");
        let load_inst = load.as_instruction_value().unwrap();
        load_inst
            .set_atomic_ordering(AtomicOrdering::SequentiallyConsistent)
            .unwrap();

        // Store an Arc to the value to ensure it stays alive
        self.atomic_captures.push(value);

        self.builder
            .position_before(&self.instruction_locations.end_of_bb_loop);

        load.into_float_value()
    }

    fn run(&mut self, number_input_id: NumberInputId, topology: &SoundGraphTopology) {
        self.builder
            .position_before(&self.instruction_locations.end_of_bb_loop);
        let final_value = self.visit_input(number_input_id, topology);
        let dst_elem_ptr = unsafe {
            self.builder.build_gep(
                self.local_variables.dst_ptr,
                &[self.local_variables.loop_counter],
                "dst_elem_ptr",
            )
        };
        self.builder.build_store(dst_elem_ptr, final_value);
    }
}

pub type ScalarReadFunc = fn(&AnyData) -> f32;
pub type ArrayReadFunc = for<'a> fn(&'a AnyData<'a>) -> &'a [f32];

unsafe extern "C" fn input_scalar_read_wrapper(
    array_read_fn: *const (),
    context_ptr: *const (),
    sound_input_id: usize,
) -> f32 {
    assert_eq!(
        std::mem::size_of::<ScalarReadFunc>(),
        std::mem::size_of::<*const ()>()
    );
    let f: ScalarReadFunc = std::mem::transmute_copy(&array_read_fn);
    let ctx: *const Context = std::mem::transmute_copy(&context_ptr);
    let ctx: &Context = unsafe { &*ctx };
    let siid = SoundInputId(sound_input_id);
    let frame = ctx.find_input_frame(siid);
    f(&frame.state())
}

unsafe extern "C" fn processor_scalar_read_wrapper(
    array_read_fn: *const (),
    context_ptr: *const (),
    sound_processor_id: usize,
) -> f32 {
    assert_eq!(
        std::mem::size_of::<ScalarReadFunc>(),
        std::mem::size_of::<*const ()>()
    );
    let f: ScalarReadFunc = std::mem::transmute_copy(&array_read_fn);
    let ctx: *const Context = std::mem::transmute_copy(&context_ptr);
    let ctx: &Context = unsafe { &*ctx };
    let spid = SoundProcessorId(sound_processor_id);
    let frame = ctx.find_processor_state(spid);
    f(&frame)
}

unsafe extern "C" fn input_array_read_wrapper(
    array_read_fn: *const (),
    context_ptr: *const (),
    sound_input_id: usize,
    expected_len: usize,
) -> *const f32 {
    assert_eq!(
        std::mem::size_of::<ArrayReadFunc>(),
        std::mem::size_of::<*const ()>()
    );
    let f: ArrayReadFunc = std::mem::transmute_copy(&array_read_fn);
    let ctx: *const Context = std::mem::transmute_copy(&context_ptr);
    let ctx: &Context = unsafe { &*ctx };
    let siid = SoundInputId(sound_input_id);
    let frame = ctx.find_input_frame(siid);
    let s = f(&frame.state());
    if s.len() != expected_len {
        panic!("input_array_read_wrapper received a slice of incorrect length");
    }
    s.as_ptr()
}

unsafe extern "C" fn processor_array_read_wrapper(
    array_read_fn: *const (),
    context_ptr: *const (),
    sound_processor_id: usize,
    expected_len: usize,
) -> *const f32 {
    assert_eq!(
        std::mem::size_of::<ArrayReadFunc>(),
        std::mem::size_of::<*const ()>()
    );
    let f: ArrayReadFunc = std::mem::transmute_copy(&array_read_fn);
    let ctx: *const Context = std::mem::transmute_copy(&context_ptr);
    let ctx: &Context = unsafe { &*ctx };
    let spid = SoundProcessorId(sound_processor_id);
    let frame = ctx.find_processor_state(spid);
    let s = f(&frame);
    if s.len() != expected_len {
        panic!("processor_array_read_wrapper received a slice of incorrect length");
    }
    s.as_ptr()
}

unsafe extern "C" fn processor_time_wrapper(
    context_ptr: *const (),
    sound_processor_id: usize,
    ptr_time: *mut f32,
    ptr_speed: *mut f32,
) {
    let ctx: *const Context = std::mem::transmute_copy(&context_ptr);
    let ctx: &Context = unsafe { &*ctx };
    let spid = SoundProcessorId(sound_processor_id);
    let (time, speed) = ctx.time_offset_and_speed_at_processor(spid);
    *ptr_time = time;
    *ptr_speed = speed / SAMPLE_FREQUENCY as f32;
}

// NOTE: could use va_args for external sources, maybe worth testing since
// that would mean less indirection
type EvalNumberInputFunc = unsafe extern "C" fn(
    *mut f32,  // pointer to destination array
    usize,     // length of destination array
    *const (), // pointer to context
);

// NOTE: Compiled number input node stores everything directly for now
// Caching and reuse among other similar/identical number nodes coming later maybe
pub(crate) struct CompiledNumberInputNode<'ctx> {
    // TODO: can stateful number source state be stored here???????

    // inkwell stuff, unsure if needed, probably useful for debugging.
    // also unsure if removing these is memory safe
    // context: &'inkwell_ctx inkwell::context::Context,
    // module: inkwell::module::Module<'ctx>,
    execution_engine: inkwell::execution_engine::ExecutionEngine<'ctx>,

    // The function compiled by LLVM
    function: inkwell::execution_engine::JitFunction<'ctx, EvalNumberInputFunc>,

    atomic_captures: Vec<Arc<AtomicF32>>,
}

impl<'ctx> Drop for CompiledNumberInputNode<'ctx> {
    fn drop(&mut self) {
        // Mainly to silence a warning that atomic_captures is unused.
        // It is indeed used to guarantee that pointers to the atomics
        // it may read from stay alive.
        self.atomic_captures.clear();
    }
}

impl<'inkwell_ctx, 'audio_ctx> CompiledNumberInputNode<'inkwell_ctx> {
    pub(crate) fn compile(
        number_input_id: NumberInputId,
        topology: &SoundGraphTopology,
        inkwell_context: &'inkwell_ctx inkwell::context::Context,
    ) -> CompiledNumberInputNode<'inkwell_ctx> {
        let module_name = format!("node_id{}", number_input_id.0);
        let module = inkwell_context.create_module(&module_name);

        let builder = inkwell_context.create_builder();
        let execution_engine = module
            .create_jit_execution_engine(inkwell::OptimizationLevel::None)
            .unwrap();

        let address_space = AddressSpace::default();
        let target_data = execution_engine.get_target_data();
        let void_type = inkwell_context.void_type();
        let u8_type = inkwell_context.i8_type();
        let ptr_type = u8_type.ptr_type(address_space);
        let f32_type = inkwell_context.f32_type();
        let f32ptr_type = f32_type.ptr_type(address_space);
        let usize_type = inkwell_context.ptr_sized_int_type(target_data, Some(address_space));

        let fn_scalar_read_wrapper_type = f32_type.fn_type(
            &[
                // array_read_fn
                usize_type.into(),
                // context_ptr
                ptr_type.into(),
                // sound_input_id/sound_processor_id
                usize_type.into(),
            ],
            false,
        );

        let fn_array_read_wrapper_type = f32ptr_type.fn_type(
            &[
                // array_read_fn
                ptr_type.into(),
                // context_ptr
                ptr_type.into(),
                // sound_input_id/sound_processor_id
                usize_type.into(),
                // expected_len
                usize_type.into(),
            ],
            false,
        );

        let fn_time_wrapper_type = void_type.fn_type(
            &[
                // context_ptr
                ptr_type.into(),
                // sound_input_id/sound_processor_id
                usize_type.into(),
                // ptr_time
                f32ptr_type.into(),
                // ptr_speed
                f32ptr_type.into(),
            ],
            false,
        );

        let fn_input_scalar_read_wrapper = module.add_function(
            "input_scalar_read_wrapper",
            fn_scalar_read_wrapper_type,
            None,
        );

        let fn_proc_scalar_read_wrapper = module.add_function(
            "processor_scalar_read_wrapper",
            fn_scalar_read_wrapper_type,
            None,
        );

        let fn_proc_array_read_wrapper = module.add_function(
            "processor_array_read_wrapper",
            fn_array_read_wrapper_type,
            None,
        );

        let fn_input_array_read_wrapper =
            module.add_function("input_array_read_wrapper", fn_array_read_wrapper_type, None);

        let fn_processor_time_wrapper =
            module.add_function("processor_time_wrapper", fn_time_wrapper_type, None);

        execution_engine.add_global_mapping(
            &fn_input_scalar_read_wrapper,
            input_scalar_read_wrapper as usize,
        );
        execution_engine.add_global_mapping(
            &fn_proc_scalar_read_wrapper,
            processor_scalar_read_wrapper as usize,
        );
        execution_engine.add_global_mapping(
            &fn_proc_array_read_wrapper,
            processor_array_read_wrapper as usize,
        );
        execution_engine.add_global_mapping(
            &fn_input_array_read_wrapper,
            input_array_read_wrapper as usize,
        );
        execution_engine
            .add_global_mapping(&fn_processor_time_wrapper, processor_time_wrapper as usize);

        let fn_eval_number_input_type = void_type.fn_type(
            &[
                // *mut f32 : pointer to destination array
                f32ptr_type.into(),
                // usize : length of destination array
                usize_type.into(),
                // *const () : pointer to context
                ptr_type.into(),
            ],
            false, // is_var_args
        );

        let function_name = format!("compiled_node_id{}", number_input_id.0);
        let fn_eval_number_input =
            module.add_function(&function_name, fn_eval_number_input_type, None);

        let bb_entry = inkwell_context.append_basic_block(fn_eval_number_input, "entry");
        let bb_loop = inkwell_context.append_basic_block(fn_eval_number_input, "loop");
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
        let arg_actx_ptr = fn_eval_number_input
            .get_nth_param(2)
            .unwrap()
            .into_pointer_value();

        arg_f32_dst_ptr.set_name("dst_ptr");
        arg_dst_len.set_name("dst_len");
        arg_actx_ptr.set_name("audio_ctx");

        let inst_end_of_entry;
        let inst_end_of_loop;
        let v_loop_counter;

        builder.position_at_end(bb_entry);
        {
            let len_is_zero = builder.build_int_compare(
                inkwell::IntPredicate::EQ,
                arg_dst_len,
                usize_type.const_zero(),
                "len_is_zero",
            );

            // array read functions will be inserted here later

            inst_end_of_entry = builder.build_conditional_branch(len_is_zero, bb_exit, bb_loop);
        }

        builder.position_at_end(bb_loop);
        {
            // if loop_counter >= dst_len { goto exit } else { goto loop_body }
            let phi = builder.build_phi(usize_type, "loop_counter");
            v_loop_counter = phi.as_basic_value().into_int_value();

            let v_loop_counter_inc = builder.build_int_add(
                v_loop_counter,
                usize_type.const_int(1, false),
                "loop_counter_inc",
            );

            phi.add_incoming(&[
                (&usize_type.const_zero(), bb_entry),
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

            inst_end_of_loop =
                builder.build_conditional_branch(v_loop_counter_ge_len, bb_exit, bb_loop);
        }

        builder.position_at_end(bb_exit);
        {
            builder.build_return(None);
        }

        let mut codegen = CodeGen::new(
            InstructionLocations {
                end_of_bb_entry: inst_end_of_entry,
                end_of_bb_loop: inst_end_of_loop,
            },
            LocalVariables {
                loop_counter: v_loop_counter,
                dst_ptr: arg_f32_dst_ptr,
                dst_len: arg_dst_len,
                context_ptr: arg_actx_ptr,
            },
            Types {
                pointer_type: ptr_type,
                float_type: f32_type,
                float_pointer_type: f32ptr_type,
                usize_type: usize_type,
            },
            WrapperFunctions {
                processor_scalar_read_wrapper: fn_proc_scalar_read_wrapper,
                input_scalar_read_wrapper: fn_input_scalar_read_wrapper,
                processor_array_read_wrapper: fn_proc_array_read_wrapper,
                input_array_read_wrapper: fn_input_array_read_wrapper,
                processor_time_wrapper: fn_processor_time_wrapper,
            },
            builder,
            module,
        );

        codegen.run(number_input_id, topology);

        if let Err(s) = codegen.module().verify() {
            let s = s.to_string();
            println!(
                "LLVM failed to verify IR for number input node {}:",
                number_input_id.value()
            );
            for line in s.lines() {
                println!("    {}", line);
            }
            panic!();
        }

        // print out the IR if testing
        // {
        //     let bc_path = Path::new("module.bc");
        //     let ll_path = Path::new("module.ll");
        //     codegen.module().write_bitcode_to_path(&bc_path);

        //     let llvm_dis_output = Command::new("llvm-dis-14")
        //         .arg(&bc_path)
        //         .arg("-o")
        //         .arg(&ll_path)
        //         .output()
        //         .expect("Failed to call llvm-dis");

        //     if !llvm_dis_output.status.success() {
        //         println!(
        //             "llvm-dis returned {}",
        //             llvm_dis_output.status.code().unwrap()
        //         );
        //         let stdout = String::from_utf8(llvm_dis_output.stdout).unwrap();
        //         let stderr = String::from_utf8(llvm_dis_output.stderr).unwrap();
        //         for l in stdout.lines() {
        //             println!("stdout | {}", l);
        //         }
        //         for l in stderr.lines() {
        //             println!("stderr | {}", l);
        //         }
        //         panic!("llvm-dis is unhappy");
        //     }

        //     let ll_contents = fs::read_to_string(ll_path).expect("Failed to open ll file");
        //     println!("LLVM IR for number input node {}", number_input_id.value());
        //     for l in ll_contents.lines() {
        //         println!("    {}", l);
        //     }

        //     std::fs::remove_file(bc_path).unwrap();
        //     std::fs::remove_file(ll_path).unwrap();
        // }

        let compiled_fn = match unsafe { execution_engine.get_function(&function_name) } {
            Ok(f) => f,
            Err(e) => {
                panic!(
                    "Unable to JIT compile number input node {}:\n    {:?}",
                    number_input_id.value(),
                    e
                );
            }
        };

        CompiledNumberInputNode {
            execution_engine,
            function: compiled_fn,
            atomic_captures: codegen.atomic_captures,
        }
    }

    pub(crate) fn eval(&self, dst: &mut [f32], context: &Context) {
        unsafe {
            let context_ptr: *const () = std::mem::transmute_copy(&context);
            self.function.call(dst.as_mut_ptr(), dst.len(), context_ptr);
        }
    }
}
