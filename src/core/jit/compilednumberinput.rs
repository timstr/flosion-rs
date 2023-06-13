use std::sync::Arc;

use atomic_float::AtomicF32;

use inkwell::AddressSpace;

#[cfg(not(debug_assertions))]
use inkwell::{
    passes::{PassManager, PassManagerBuilder},
    OptimizationLevel,
};

use crate::core::{
    jit::{
        codegen::{CodeGen, InstructionLocations, LocalVariables, Types, WrapperFunctions},
        wrappers::{
            input_array_read_wrapper, input_scalar_read_wrapper, input_time_wrapper,
            processor_array_read_wrapper, processor_scalar_read_wrapper, processor_time_wrapper,
        },
    },
    number::numbergraphdata::NumberTarget,
    sound::{
        context::Context, soundgraphtopology::SoundGraphTopology,
        soundnumberinput::SoundNumberInputId,
    },
    uniqueid::UniqueId,
};

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
    // TODO: move everything here that isn't unique to each individual
    // compiled number input into a shared place and stop recreating
    // it for each new input.
    // TODO: either make compiled number input nodes very cheap to copy
    // or find a way to clone the correct number of them ahead of time
    // so that the audio thread is always able to cheaply update its
    // nodes
    pub(crate) fn compile(
        number_input_id: SoundNumberInputId,
        topology: &SoundGraphTopology,
        inkwell_context: &'inkwell_ctx inkwell::context::Context,
    ) -> CompiledNumberInputNode<'inkwell_ctx> {
        let module_name = format!("node_id{}", number_input_id.value());
        let module = inkwell_context.create_module(&module_name);

        let builder = inkwell_context.create_builder();

        // TODO: change optimization level here in release builds
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

        let fn_input_time_wrapper =
            module.add_function("input_time_wrapper", fn_time_wrapper_type, None);

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
        execution_engine.add_global_mapping(&fn_input_time_wrapper, input_time_wrapper as usize);

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

        let function_name = format!("compiled_node_id{}", number_input_id.value());
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
                input_time_wrapper: fn_input_time_wrapper,
            },
            builder,
            module,
        );

        let sg_number_input_data = topology.number_input(number_input_id).unwrap();

        let number_topo = sg_number_input_data.number_graph().topology();

        // pre-compile all number graph inputs
        for (snsid, giid) in sg_number_input_data.input_mapping() {
            let value = topology
                .number_source(snsid)
                .unwrap()
                .instance()
                .compile(&mut codegen);
            codegen.assign_target(NumberTarget::GraphInput(giid), value);
        }

        // TODO: add support for multiple outputs
        assert_eq!(number_topo.graph_outputs().len(), 1);

        // compile the number graph output
        codegen.run(number_topo.graph_outputs()[0].id(), number_topo);

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

        // Apply optimizations in release mode
        #[cfg(not(debug_assertions))]
        {
            let pass_manager_builder = PassManagerBuilder::create();

            pass_manager_builder.set_optimization_level(OptimizationLevel::Aggressive);
            // TODO: other optimization options?

            let pass_manager = PassManager::create(());

            pass_manager_builder.populate_lto_pass_manager(&pass_manager, false, false);

            pass_manager.run_on(codegen.module());
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
            execution_engine,
            function: compiled_fn,
            atomic_captures: codegen.into_atomic_captures(),
        }
    }

    pub(crate) fn eval(&self, dst: &mut [f32], context: &Context) {
        unsafe {
            let context_ptr: *const () = std::mem::transmute_copy(&context);
            self.function.call(dst.as_mut_ptr(), dst.len(), context_ptr);
        }
    }
}
