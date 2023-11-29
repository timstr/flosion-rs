use std::sync::Arc;

use atomicslice::AtomicSlice;
use inkwell::{
    data_layout,
    values::{FloatValue, IntValue, PointerValue},
    AtomicOrdering, AtomicRMWBinOp, IntPredicate,
};
use serialization::Serializer;

use crate::core::{
    graph::graphobject::{ObjectInitialization, ObjectType, WithObjectType},
    jit::codegen::CodeGen,
    number::{
        numberinput::NumberInputHandle, numbersource::StatefulNumberSource,
        numbersourcetools::NumberSourceTools,
    },
};

pub struct Sampler1d {
    input: NumberInputHandle,
    value: Arc<AtomicSlice<f32>>,
}

impl Sampler1d {
    pub fn value(&self) -> &AtomicSlice<f32> {
        &self.value
    }
}

pub struct Sampler1dCompileState<'ctx> {
    ptr_slice: PointerValue<'ctx>,
    current_slice: IntValue<'ctx>,
    ptr_status: PointerValue<'ctx>,
}

impl StatefulNumberSource for Sampler1d {
    fn new(mut tools: NumberSourceTools<'_>, init: ObjectInitialization) -> Result<Self, ()> {
        // TODO: use init
        let mut value = Vec::new();
        value.resize(256, 0.0);
        Ok(Sampler1d {
            input: tools.add_number_input(0.0),
            value: Arc::new(AtomicSlice::new(value)),
        })
    }

    const NUM_VARIABLES: usize = 0;

    type CompileState<'ctx> = Sampler1dCompileState<'ctx>;

    fn serialize(&self, mut serializer: Serializer) {
        // TODO
    }

    fn compile_init<'ctx>(&self, _codegen: &mut CodeGen<'ctx>) -> Vec<FloatValue<'ctx>> {
        vec![]
    }

    fn compile_pre_loop<'ctx>(&self, codegen: &mut CodeGen<'ctx>) -> Sampler1dCompileState<'ctx> {
        let ptr_data;
        let ptr_status;
        unsafe {
            ptr_data = self.value.raw_data();
            ptr_status = self.value.raw_status();
        }
        let addr_status = codegen.types.usize_type.const_int(ptr_status as u64, false);
        let ptr_status = codegen.builder().build_int_to_ptr(
            addr_status,
            codegen.types.u64_pointer_type,
            "p_atomicstatus",
        );
        let inc_all_slices = codegen
            .types
            .u64_type
            .const_int(atomicslice::constants::INC_ALL_SLICES, false);
        let status_val = codegen
            .builder()
            .build_atomicrmw(
                AtomicRMWBinOp::Add,
                ptr_status,
                inc_all_slices,
                AtomicOrdering::SequentiallyConsistent,
            )
            .unwrap();
        let current_slice_mask = codegen
            .types
            .u64_type
            .const_int(atomicslice::constants::CURRENT_SLICE_MASK, false);
        let current_slice =
            codegen
                .builder()
                .build_and(status_val, current_slice_mask, "current_slice");
        let first_slice_is_active = codegen.builder().build_int_compare(
            IntPredicate::EQ,
            current_slice,
            codegen.types.u64_type.const_zero(),
            "current_slice_is_zero_A",
        );
        let inc_slice_1 = codegen
            .types
            .u64_type
            .const_int(atomicslice::constants::SLICE_1_INC, false);
        let inc_slice_2 = codegen
            .types
            .u64_type
            .const_int(atomicslice::constants::SLICE_2_INC, false);
        let inc_other_slice = codegen
            .builder()
            .build_select(
                first_slice_is_active,
                inc_slice_2,
                inc_slice_1,
                "inc_other_slice",
            )
            .into_int_value();
        codegen
            .builder()
            .build_atomicrmw(
                AtomicRMWBinOp::Sub,
                ptr_status,
                inc_other_slice,
                AtomicOrdering::SequentiallyConsistent,
            )
            .unwrap();
        let slice_len = codegen
            .types
            .usize_type
            .const_int(self.value.len() as u64, false);
        let data_addr = codegen.types.usize_type.const_int(ptr_data as u64, false);
        let ptr_data = codegen.builder().build_int_to_ptr(
            data_addr,
            codegen.types.f32_pointer_type,
            "ptr_data",
        );
        let ptr_slice = unsafe {
            codegen
                .builder()
                .build_gep(ptr_data, &[slice_len], "ptr_slice")
        };
        Sampler1dCompileState {
            ptr_slice,
            current_slice,
            ptr_status,
        }
    }

    fn compile_post_loop<'ctx>(
        &self,
        codegen: &mut CodeGen<'ctx>,
        compile_state: &Sampler1dCompileState<'ctx>,
    ) {
        let inc_slice_1 = codegen
            .types
            .u64_type
            .const_int(atomicslice::constants::SLICE_1_INC, false);
        let inc_slice_2 = codegen
            .types
            .u64_type
            .const_int(atomicslice::constants::SLICE_2_INC, false);
        let first_slice_is_active = codegen.builder().build_int_compare(
            IntPredicate::EQ,
            compile_state.current_slice,
            codegen.types.u64_type.const_zero(),
            "current_slice_is_zero_B",
        );
        let inc_other_slice = codegen
            .builder()
            .build_select(first_slice_is_active, inc_slice_1, inc_slice_2, "inc_slice")
            .into_int_value();
        codegen
            .builder()
            .build_atomicrmw(
                AtomicRMWBinOp::Sub,
                compile_state.ptr_status,
                inc_other_slice,
                AtomicOrdering::SequentiallyConsistent,
            )
            .unwrap();
    }

    fn compile_loop<'ctx>(
        &self,
        codegen: &mut CodeGen<'ctx>,
        inputs: &[FloatValue<'ctx>],
        variables: &[PointerValue<'ctx>],
        compile_state: &Sampler1dCompileState<'ctx>,
    ) -> FloatValue<'ctx> {
        debug_assert_eq!(inputs.len(), 1);
        debug_assert_eq!(variables.len(), 0);
        // TODO:
        // interpolate samples in pointer to slice, using input
        // should probably write a helper for this
        // assume circular boundary condition for now, maybe
        // make that configurable in the future

        let ptr_slice = compile_state.ptr_slice;
        // HACK: just load first value for now
        codegen
            .builder()
            .build_load(ptr_slice, "slice0")
            .into_float_value()
    }
}

impl WithObjectType for Sampler1d {
    const TYPE: ObjectType = ObjectType::new("sampler1d");
}
