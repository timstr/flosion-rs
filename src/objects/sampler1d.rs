use std::sync::Arc;

use atomicslice::AtomicSlice;
use chive::ChiveIn;
use inkwell::{
    values::{FloatValue, IntValue, PointerValue},
    AtomicOrdering, AtomicRMWBinOp, IntPredicate,
};

use crate::{
    core::{
        expression::{
            expressionnode::StatefulExpressionNode, expressionnodeinput::ExpressionNodeInputHandle,
            expressionnodetools::ExpressionNodeTools,
        },
        jit::codegen::CodeGen,
        objecttype::{ObjectType, WithObjectType},
    },
    ui_core::arguments::ParsedArguments,
};

pub struct Sampler1d {
    input: ExpressionNodeInputHandle,
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

impl StatefulExpressionNode for Sampler1d {
    fn new(mut tools: ExpressionNodeTools<'_>, args: &ParsedArguments) -> Result<Self, ()> {
        // TODO: use args?
        let mut value = Vec::new();
        value.resize(256, 0.0);
        Ok(Sampler1d {
            input: tools.add_input(0.0),
            value: Arc::new(AtomicSlice::new(value)),
        })
    }

    const NUM_VARIABLES: usize = 0;

    type CompileState<'ctx> = Sampler1dCompileState<'ctx>;

    fn serialize(&self, mut chive_in: ChiveIn) {
        // TODO
        todo!()
    }

    fn compile_start_over<'ctx>(&self, _codegen: &mut CodeGen<'ctx>) -> Vec<FloatValue<'ctx>> {
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
        let ptr_status = codegen
            .builder()
            .build_int_to_ptr(
                addr_status,
                codegen.types.u64_pointer_type,
                "p_atomicstatus",
            )
            .unwrap();
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
        let current_slice = codegen
            .builder()
            .build_and(status_val, current_slice_mask, "current_slice")
            .unwrap();
        let first_slice_is_active = codegen
            .builder()
            .build_int_compare(
                IntPredicate::EQ,
                current_slice,
                codegen.types.u64_type.const_zero(),
                "current_slice_is_zero_A",
            )
            .unwrap();
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
            .unwrap()
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
        let ptr_data = codegen
            .builder()
            .build_int_to_ptr(data_addr, codegen.types.f32_pointer_type, "ptr_data")
            .unwrap();
        let offset = codegen
            .builder()
            .build_select(
                first_slice_is_active,
                codegen.types.usize_type.const_zero(),
                slice_len,
                "offset",
            )
            .unwrap()
            .into_int_value();
        let ptr_slice = unsafe {
            codegen
                .builder()
                .build_gep(ptr_data, &[offset], "ptr_slice")
        }
        .unwrap();
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
        let first_slice_is_active = codegen
            .builder()
            .build_int_compare(
                IntPredicate::EQ,
                compile_state.current_slice,
                codegen.types.u64_type.const_zero(),
                "current_slice_is_zero_B",
            )
            .unwrap();
        let inc_other_slice = codegen
            .builder()
            .build_select(first_slice_is_active, inc_slice_1, inc_slice_2, "inc_slice")
            .unwrap()
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
        // TODO: move this into a codegen helper function

        let input = inputs[0];
        let floor_input = codegen.build_unary_intrinsic_call("llvm.floor", input);
        let input_wrapped = codegen
            .builder()
            .build_float_sub(input, floor_input, "input_wrapped")
            .unwrap();

        let index_float = codegen
            .builder()
            .build_float_mul(
                input_wrapped,
                codegen.types.f32_type.const_float(self.value.len() as f64),
                "index_float",
            )
            .unwrap();
        let index_floor = codegen.build_unary_intrinsic_call("llvm.floor", index_float);
        let index_ceil = codegen.build_unary_intrinsic_call("llvm.ceil", index_float);
        let index_fract = codegen
            .builder()
            .build_float_sub(index_float, index_floor, "index_fract")
            .unwrap();
        let index_floor_int = codegen
            .builder()
            .build_float_to_unsigned_int(index_floor, codegen.types.usize_type, "index_floor_int")
            .unwrap();
        let index_ceil_int = codegen
            .builder()
            .build_float_to_unsigned_int(index_ceil, codegen.types.usize_type, "index_ceil_int")
            .unwrap();
        let slice_len = codegen
            .types
            .usize_type
            .const_int(self.value.len() as u64, false);
        let zero = codegen.types.usize_type.const_zero();
        let index_floor_int_is_n = codegen
            .builder()
            .build_int_compare(
                IntPredicate::EQ,
                index_floor_int,
                slice_len,
                "index_floor_int_is_n",
            )
            .unwrap();
        let index_ceil_int_is_n = codegen
            .builder()
            .build_int_compare(
                IntPredicate::EQ,
                index_ceil_int,
                slice_len,
                "index_ceil_int_is_n",
            )
            .unwrap();
        let i0 = codegen
            .builder()
            .build_select(index_floor_int_is_n, zero, index_floor_int, "i0")
            .unwrap()
            .into_int_value();
        let i1 = codegen
            .builder()
            .build_select(index_ceil_int_is_n, zero, index_ceil_int, "i0")
            .unwrap()
            .into_int_value();

        let ptr_slice = compile_state.ptr_slice;

        let ptr_v0 = unsafe { codegen.builder().build_gep(ptr_slice, &[i0], "ptr_v0") }.unwrap();
        let ptr_v1 = unsafe { codegen.builder().build_gep(ptr_slice, &[i1], "ptr_v0") }.unwrap();
        let v0 = codegen
            .builder()
            .build_load(ptr_v0, "v0")
            .unwrap()
            .into_float_value();
        let v1 = codegen
            .builder()
            .build_load(ptr_v1, "v1")
            .unwrap()
            .into_float_value();
        let diff = codegen.builder().build_float_sub(v1, v0, "diff").unwrap();
        let scaled_diff = codegen
            .builder()
            .build_float_mul(index_fract, diff, "scaled_diff")
            .unwrap();
        let v = codegen
            .builder()
            .build_float_add(v0, scaled_diff, "v")
            .unwrap();

        v
    }
}

impl WithObjectType for Sampler1d {
    const TYPE: ObjectType = ObjectType::new("sampler1d");
}
