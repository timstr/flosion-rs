use std::sync::Arc;

use atomicslice::AtomicSlice;
use hashstash::{InplaceUnstasher, Stashable, Stasher, UnstashError, UnstashableInplace};
use inkwell::{
    values::{FloatValue, IntValue, PointerValue},
    AtomicOrdering, AtomicRMWBinOp, IntPredicate,
};

use crate::{
    core::{
        expression::{
            expressioninput::ExpressionInput,
            expressionnode::{ExpressionNode, ExpressionNodeVisitor, ExpressionNodeVisitorMut},
        },
        jit::jit::Jit,
        objecttype::{ObjectType, WithObjectType},
    },
    ui_core::arguments::ParsedArguments,
};

pub struct Sampler1d {
    input: ExpressionInput,
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

impl ExpressionNode for Sampler1d {
    fn new(args: &ParsedArguments) -> Sampler1d {
        // TODO: use args?
        let mut value = Vec::new();
        value.resize(256, 0.0);
        Sampler1d {
            input: ExpressionInput::new(0.0),
            value: Arc::new(AtomicSlice::new(value)),
        }
    }

    const NUM_VARIABLES: usize = 0;

    type CompileState<'ctx> = Sampler1dCompileState<'ctx>;

    fn compile_start_over<'ctx>(&self, _jit: &mut Jit<'ctx>) -> Vec<FloatValue<'ctx>> {
        vec![]
    }

    fn compile_pre_loop<'ctx>(&self, jit: &mut Jit<'ctx>) -> Sampler1dCompileState<'ctx> {
        let ptr_data;
        let ptr_status;
        unsafe {
            ptr_data = self.value.raw_data();
            ptr_status = self.value.raw_status();
        }
        let addr_status = jit.types.usize_type.const_int(ptr_status as u64, false);
        let ptr_status = jit
            .builder()
            .build_int_to_ptr(addr_status, jit.types.pointer_type, "p_atomicstatus")
            .unwrap();
        let inc_all_slices = jit
            .types
            .u64_type
            .const_int(atomicslice::constants::INC_ALL_SLICES, false);
        let status_val = jit
            .builder()
            .build_atomicrmw(
                AtomicRMWBinOp::Add,
                ptr_status,
                inc_all_slices,
                AtomicOrdering::SequentiallyConsistent,
            )
            .unwrap();
        let current_slice_mask = jit
            .types
            .u64_type
            .const_int(atomicslice::constants::CURRENT_SLICE_MASK, false);
        let current_slice = jit
            .builder()
            .build_and(status_val, current_slice_mask, "current_slice")
            .unwrap();
        let first_slice_is_active = jit
            .builder()
            .build_int_compare(
                IntPredicate::EQ,
                current_slice,
                jit.types.u64_type.const_zero(),
                "current_slice_is_zero_A",
            )
            .unwrap();
        let inc_slice_1 = jit
            .types
            .u64_type
            .const_int(atomicslice::constants::SLICE_1_INC, false);
        let inc_slice_2 = jit
            .types
            .u64_type
            .const_int(atomicslice::constants::SLICE_2_INC, false);
        let inc_other_slice = jit
            .builder()
            .build_select(
                first_slice_is_active,
                inc_slice_2,
                inc_slice_1,
                "inc_other_slice",
            )
            .unwrap()
            .into_int_value();
        jit.builder()
            .build_atomicrmw(
                AtomicRMWBinOp::Sub,
                ptr_status,
                inc_other_slice,
                AtomicOrdering::SequentiallyConsistent,
            )
            .unwrap();
        let slice_len = jit
            .types
            .usize_type
            .const_int(self.value.len() as u64, false);
        let data_addr = jit.types.usize_type.const_int(ptr_data as u64, false);
        let ptr_data = jit
            .builder()
            .build_int_to_ptr(data_addr, jit.types.pointer_type, "ptr_data")
            .unwrap();
        let offset = jit
            .builder()
            .build_select(
                first_slice_is_active,
                jit.types.usize_type.const_zero(),
                slice_len,
                "offset",
            )
            .unwrap()
            .into_int_value();
        let ptr_slice = unsafe {
            jit.builder()
                .build_gep(jit.types.f32_type, ptr_data, &[offset], "ptr_slice")
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
        jit: &mut Jit<'ctx>,
        compile_state: &Sampler1dCompileState<'ctx>,
    ) {
        let inc_slice_1 = jit
            .types
            .u64_type
            .const_int(atomicslice::constants::SLICE_1_INC, false);
        let inc_slice_2 = jit
            .types
            .u64_type
            .const_int(atomicslice::constants::SLICE_2_INC, false);
        let first_slice_is_active = jit
            .builder()
            .build_int_compare(
                IntPredicate::EQ,
                compile_state.current_slice,
                jit.types.u64_type.const_zero(),
                "current_slice_is_zero_B",
            )
            .unwrap();
        let inc_other_slice = jit
            .builder()
            .build_select(first_slice_is_active, inc_slice_1, inc_slice_2, "inc_slice")
            .unwrap()
            .into_int_value();
        jit.builder()
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
        jit: &mut Jit<'ctx>,
        inputs: &[FloatValue<'ctx>],
        variables: &[PointerValue<'ctx>],
        compile_state: &Sampler1dCompileState<'ctx>,
    ) -> FloatValue<'ctx> {
        debug_assert_eq!(inputs.len(), 1);
        debug_assert_eq!(variables.len(), 0);
        // TODO: move this into a jit helper function

        let input = inputs[0];
        let floor_input = jit.build_unary_intrinsic_call("llvm.floor", input);
        let input_wrapped = jit
            .builder()
            .build_float_sub(input, floor_input, "input_wrapped")
            .unwrap();

        let index_float = jit
            .builder()
            .build_float_mul(
                input_wrapped,
                jit.types.f32_type.const_float(self.value.len() as f64),
                "index_float",
            )
            .unwrap();
        let index_floor = jit.build_unary_intrinsic_call("llvm.floor", index_float);
        let index_ceil = jit.build_unary_intrinsic_call("llvm.ceil", index_float);
        let index_fract = jit
            .builder()
            .build_float_sub(index_float, index_floor, "index_fract")
            .unwrap();
        let index_floor_int = jit
            .builder()
            .build_float_to_unsigned_int(index_floor, jit.types.usize_type, "index_floor_int")
            .unwrap();
        let index_ceil_int = jit
            .builder()
            .build_float_to_unsigned_int(index_ceil, jit.types.usize_type, "index_ceil_int")
            .unwrap();
        let slice_len = jit
            .types
            .usize_type
            .const_int(self.value.len() as u64, false);
        let zero = jit.types.usize_type.const_zero();
        let index_floor_int_is_n = jit
            .builder()
            .build_int_compare(
                IntPredicate::EQ,
                index_floor_int,
                slice_len,
                "index_floor_int_is_n",
            )
            .unwrap();
        let index_ceil_int_is_n = jit
            .builder()
            .build_int_compare(
                IntPredicate::EQ,
                index_ceil_int,
                slice_len,
                "index_ceil_int_is_n",
            )
            .unwrap();
        let i0 = jit
            .builder()
            .build_select(index_floor_int_is_n, zero, index_floor_int, "i0")
            .unwrap()
            .into_int_value();
        let i1 = jit
            .builder()
            .build_select(index_ceil_int_is_n, zero, index_ceil_int, "i0")
            .unwrap()
            .into_int_value();

        let ptr_slice = compile_state.ptr_slice;

        let ptr_v0 = unsafe {
            jit.builder()
                .build_gep(jit.types.f32_type, ptr_slice, &[i0], "ptr_v0")
        }
        .unwrap();
        let ptr_v1 = unsafe {
            jit.builder()
                .build_gep(jit.types.f32_type, ptr_slice, &[i1], "ptr_v0")
        }
        .unwrap();
        let v0 = jit
            .builder()
            .build_load(jit.types.f32_type, ptr_v0, "v0")
            .unwrap()
            .into_float_value();
        let v1 = jit
            .builder()
            .build_load(jit.types.f32_type, ptr_v1, "v1")
            .unwrap()
            .into_float_value();
        let diff = jit.builder().build_float_sub(v1, v0, "diff").unwrap();
        let scaled_diff = jit
            .builder()
            .build_float_mul(index_fract, diff, "scaled_diff")
            .unwrap();
        let v = jit.builder().build_float_add(v0, scaled_diff, "v").unwrap();

        v
    }

    fn visit(&self, visitor: &mut dyn ExpressionNodeVisitor) {
        visitor.input(&self.input);
    }
    fn visit_mut(&mut self, visitor: &mut dyn ExpressionNodeVisitorMut) {
        visitor.input(&mut self.input);
    }
}

impl Stashable for Sampler1d {
    fn stash(&self, stasher: &mut Stasher) {
        stasher.object(&self.input);
        // TODO: how should changes to this NOT trigger a recompilation?
        let reader = self.value.read();
        stasher.array_of_f32_slice(&reader);
    }
}

impl UnstashableInplace for Sampler1d {
    fn unstash_inplace(&mut self, unstasher: &mut InplaceUnstasher) -> Result<(), UnstashError> {
        unstasher.object_inplace(&mut self.input)?;
        let new_values = unstasher.array_of_f32_iter()?;
        if unstasher.time_to_write() {
            let new_values: Vec<f32> = new_values.collect();
            self.value.write(&new_values);
        }
        Ok(())
    }
}

impl WithObjectType for Sampler1d {
    const TYPE: ObjectType = ObjectType::new("sampler1d");
}
