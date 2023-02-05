use std::{fs, path::Path, process::Command};

use inkwell::{
    builder::Builder,
    types::{FloatType, IntType},
    values::{FloatValue, FunctionValue, InstructionValue, IntValue, PointerValue},
};

use crate::core::uniqueid::UniqueId;

use super::{
    context::Context, numberinput::NumberInputId, numbersource::NumberSourceId,
    soundgraphtopology::SoundGraphTopology, soundprocessor::SoundProcessorId,
};

pub struct CodeGen<'ctx> {
    end_of_bb_entry: InstructionValue<'ctx>,
    end_of_bb_loop: InstructionValue<'ctx>,
    loop_counter: IntValue<'ctx>,
    dst_ptr: PointerValue<'ctx>,
    context_ptr: PointerValue<'ctx>,
    float_type: FloatType<'ctx>,
    usize_type: IntType<'ctx>,
    array_read_wrapper: FunctionValue<'ctx>,
    builder: Builder<'ctx>,
}

impl<'ctx> CodeGen<'ctx> {
    fn new(
        end_of_bb_entry: InstructionValue<'ctx>,
        end_of_bb_loop: InstructionValue<'ctx>,
        loop_counter: IntValue<'ctx>,
        dst_ptr: PointerValue<'ctx>,
        context_ptr: PointerValue<'ctx>,
        float_type: FloatType<'ctx>,
        usize_type: IntType<'ctx>,
        array_read_wrapper: FunctionValue<'ctx>,
        builder: Builder<'ctx>,
    ) -> CodeGen<'ctx> {
        CodeGen {
            end_of_bb_entry,
            end_of_bb_loop,
            loop_counter,
            dst_ptr,
            context_ptr,
            float_type,
            usize_type,
            array_read_wrapper,
            builder,
        }
    }

    fn visit_input(
        &self,
        number_input_id: NumberInputId,
        topology: &SoundGraphTopology,
    ) -> FloatValue<'ctx> {
        let input_data = topology.number_input(number_input_id).unwrap();
        match input_data.target() {
            Some(nsid) => self.visit_source(nsid, topology),
            None => self
                .float_type
                .const_float(input_data.default_value().into()),
        }
    }

    fn visit_source(
        &self,
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

    pub fn builder(&self) -> &Builder<'ctx> {
        &self.builder
    }

    pub fn float_type(&self) -> &FloatType<'ctx> {
        &self.float_type
    }

    pub fn build_array_read(
        &self,
        processor_id: SoundProcessorId,
        function: ArrayReadFunc,
    ) -> FloatValue<'ctx> {
        self.builder.position_before(&self.end_of_bb_entry);
        let function_addr = self.usize_type.const_int(function as u64, false);
        let spid = self.usize_type.const_int(processor_id.0 as u64, false);
        let call_site_value = self.builder.build_call(
            self.array_read_wrapper,
            &[function_addr.into(), self.context_ptr.into(), spid.into()],
            "array_read_fn_retv",
        );
        let array_read_retv = call_site_value
            .try_as_basic_value()
            .left()
            .unwrap()
            .into_pointer_value();

        self.builder.position_before(&self.end_of_bb_loop);
        let array_elem_ptr = unsafe {
            self.builder
                .build_gep(array_read_retv, &[self.loop_counter], "array_elem_ptr")
        };
        let array_elem = self.builder.build_load(array_elem_ptr, "array_elem");
        array_elem.into_float_value()
    }

    fn run(self, number_input_id: NumberInputId, topology: &SoundGraphTopology) {
        self.builder.position_before(&self.end_of_bb_loop);
        let final_value = self.visit_input(number_input_id, topology);
        let dst_elem_ptr = unsafe {
            self.builder
                .build_gep(self.dst_ptr, &[self.loop_counter], "dst_elem_ptr")
        };
        self.builder.build_store(dst_elem_ptr, final_value);
    }
}

type ArrayReadFunc<'a> = fn(&'a Context, SoundProcessorId) -> &'a [f32];

unsafe extern "C" fn array_read_wrapper(
    array_read_fn: *const (),
    context_ptr: *const (),
    sound_processor_id: usize,
) -> *const f32 {
    assert_eq!(
        std::mem::size_of::<ArrayReadFunc>(),
        std::mem::size_of::<*const ()>()
    );
    let f: ArrayReadFunc = std::mem::transmute_copy(&array_read_fn);
    let ctx: *const Context = std::mem::transmute_copy(&context_ptr);
    let spid = SoundProcessorId(sound_processor_id);
    let s = f(&*ctx, spid);
    // TODO: check length of slice
    s.as_ptr()
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
pub(super) struct CompiledNumberInputNode<'ctx> {
    // inkwell stuff, unsure if needed, probably useful for debugging.
    // also unsure if removing these is memory safe
    // context: &'inkwell_ctx inkwell::context::Context,
    module: inkwell::module::Module<'ctx>,
    execution_engine: inkwell::execution_engine::ExecutionEngine<'ctx>,

    // The function compiled by LLVM
    function: inkwell::execution_engine::JitFunction<'ctx, EvalNumberInputFunc>,
}

impl<'inkwell_ctx, 'audio_ctx> CompiledNumberInputNode<'inkwell_ctx> {
    pub(super) fn compile(
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

        let address_space = inkwell::AddressSpace::default();
        let target_data = execution_engine.get_target_data();
        let void_type = inkwell_context.void_type();
        let u8_type = inkwell_context.i8_type();
        let ptr_type = u8_type.ptr_type(address_space);
        let f32_type = inkwell_context.f32_type();
        let f32ptr_type = f32_type.ptr_type(address_space);
        let usize_type = inkwell_context.ptr_sized_int_type(target_data, Some(address_space));

        let fn_array_read_wrapper_type = ptr_type.fn_type(
            &[
                ptr_type.into(),
                ptr_type.into(),
                usize_type.into(),
                usize_type.into(),
            ],
            false,
        );

        let fn_array_read_wrapper =
            module.add_function("array_read_wrapper", fn_array_read_wrapper_type, None);

        execution_engine.add_global_mapping(&fn_array_read_wrapper, array_read_wrapper as usize);

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
            builder.build_return(Some(&u8_type.const_zero()));
        }

        let codegen = CodeGen::new(
            inst_end_of_entry,
            inst_end_of_loop,
            v_loop_counter,
            arg_f32_dst_ptr,
            arg_actx_ptr,
            f32_type,
            usize_type,
            fn_array_read_wrapper,
            builder,
        );

        codegen.run(number_input_id, topology);

        // print out the IR if testing
        #[cfg(debug_assertions)]
        {
            let bc_path = Path::new("module.bc");
            let ll_path = Path::new("module.ll");
            module.write_bitcode_to_path(&bc_path);

            let llvm_dis_output = Command::new("llvm-dis-14")
                .arg(&bc_path)
                .arg("-o")
                .arg(&ll_path)
                .output()
                .expect("Failed to call llvm-dis");

            if !llvm_dis_output.status.success() {
                println!(
                    "llvm-dis returned {}",
                    llvm_dis_output.status.code().unwrap()
                );
                let stdout = String::from_utf8(llvm_dis_output.stdout).unwrap();
                let stderr = String::from_utf8(llvm_dis_output.stderr).unwrap();
                for l in stdout.lines() {
                    println!("stdout | {}", l);
                }
                for l in stderr.lines() {
                    println!("stderr | {}", l);
                }
                panic!("llvm-dis is unhappy");
            }

            let ll_contents = fs::read_to_string(ll_path).expect("Failed to open ll file");
            println!("LLVM IR");
            for l in ll_contents.lines() {
                println!("    {}", l);
            }

            std::fs::remove_file(bc_path).unwrap();
            std::fs::remove_file(ll_path).unwrap();
        }

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
            // context: inkwell_context,
            module,
            execution_engine,
            function: compiled_fn,
        }
    }

    pub(super) fn eval(&self, _dst: &mut [f32], _context: &Context) {
        todo!()
    }
}
