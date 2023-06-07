use std::{collections::HashMap, sync::Arc};

use atomic_float::AtomicF32;
use inkwell::{
    builder::Builder,
    intrinsics::Intrinsic,
    module::Module,
    types::{FloatType, IntType, PointerType},
    values::{BasicValue, FloatValue, FunctionValue, InstructionValue, IntValue, PointerValue},
    AtomicOrdering,
};

use crate::core::{
    number::{
        numbergraph::NumberGraphOutputId, numbergraphdata::NumberTarget,
        numbergraphtopology::NumberGraphTopology, numberinput::NumberInputId,
    },
    sound::{soundinput::SoundInputId, soundprocessor::SoundProcessorId},
    uniqueid::UniqueId,
};

use super::wrappers::{ArrayReadFunc, ScalarReadFunc};

pub(super) struct InstructionLocations<'ctx> {
    pub(super) end_of_bb_entry: InstructionValue<'ctx>,
    pub(super) end_of_bb_loop: InstructionValue<'ctx>,
}

pub(super) struct LocalVariables<'ctx> {
    pub(super) loop_counter: IntValue<'ctx>,
    pub(super) dst_ptr: PointerValue<'ctx>,
    pub(super) dst_len: IntValue<'ctx>,
    pub(super) context_ptr: PointerValue<'ctx>,
}

pub(super) struct Types<'ctx> {
    pub(super) pointer_type: PointerType<'ctx>,
    pub(super) float_type: FloatType<'ctx>,
    pub(super) float_pointer_type: PointerType<'ctx>,
    pub(super) usize_type: IntType<'ctx>,
}

pub(super) struct WrapperFunctions<'ctx> {
    pub(super) processor_scalar_read_wrapper: FunctionValue<'ctx>,
    pub(super) input_scalar_read_wrapper: FunctionValue<'ctx>,
    pub(super) processor_array_read_wrapper: FunctionValue<'ctx>,
    pub(super) input_array_read_wrapper: FunctionValue<'ctx>,
    pub(super) processor_time_wrapper: FunctionValue<'ctx>,
    pub(super) input_time_wrapper: FunctionValue<'ctx>,
}

pub struct CodeGen<'ctx> {
    pub(super) instruction_locations: InstructionLocations<'ctx>,
    pub(super) local_variables: LocalVariables<'ctx>,
    pub(super) types: Types<'ctx>,
    pub(super) wrapper_functions: WrapperFunctions<'ctx>,
    pub(super) builder: Builder<'ctx>,
    pub(super) module: Module<'ctx>,
    pub(super) atomic_captures: Vec<Arc<AtomicF32>>,
    pub(super) compiled_targets: HashMap<NumberTarget, FloatValue<'ctx>>,
}

impl<'ctx> CodeGen<'ctx> {
    pub(super) fn new(
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
            compiled_targets: HashMap::new(),
        }
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
                .float_type
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

                let input_values: Vec<_> = source_data
                    .number_inputs()
                    .iter()
                    .map(|niid| self.visit_input(*niid, topology))
                    .collect();
                let v = source_data.instance().compile(self, &input_values);
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

    pub fn build_input_time(&mut self, input_id: SoundInputId) -> FloatValue<'ctx> {
        self.builder
            .position_before(&self.instruction_locations.end_of_bb_entry);
        let siid = self
            .types
            .usize_type
            .const_int(input_id.value() as u64, false);
        let ptr_time = self.builder.build_alloca(self.types.float_type, "time");
        let ptr_speed = self.builder.build_alloca(self.types.float_type, "speed");
        self.builder.build_call(
            self.wrapper_functions.input_time_wrapper,
            &[
                self.local_variables.context_ptr.into(),
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

    pub(super) fn run(&mut self, output_id: NumberGraphOutputId, topology: &NumberGraphTopology) {
        self.builder
            .position_before(&self.instruction_locations.end_of_bb_loop);
        let output_data = topology.graph_output(output_id).unwrap();
        let final_value = match output_data.target() {
            Some(target) => self.visit_target(target, topology),
            None => self
                .types
                .float_type
                .const_float(output_data.default_value() as f64)
                .into(),
        };
        let dst_elem_ptr = unsafe {
            self.builder.build_gep(
                self.local_variables.dst_ptr,
                &[self.local_variables.loop_counter],
                "dst_elem_ptr",
            )
        };
        self.builder.build_store(dst_elem_ptr, final_value);
    }

    pub(super) fn into_atomic_captures(self) -> Vec<Arc<AtomicF32>> {
        self.atomic_captures
    }
}
