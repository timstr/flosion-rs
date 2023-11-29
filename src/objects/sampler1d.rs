use std::sync::Arc;

use atomicslice::AtomicSlice;
use inkwell::values::{FloatValue, PointerValue};
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
    slice: PointerValue<'ctx>,
    status: PointerValue<'ctx>,
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
        todo!()
    }

    fn compile_post_loop<'ctx>(
        &self,
        codegen: &mut CodeGen<'ctx>,
        compile_state: &Sampler1dCompileState<'ctx>,
    ) {
        todo!()
    }

    fn compile_loop<'ctx>(
        &self,
        codegen: &mut CodeGen<'ctx>,
        inputs: &[FloatValue<'ctx>],
        variables: &[PointerValue<'ctx>],
        compile_state: &Sampler1dCompileState<'ctx>,
    ) -> FloatValue<'ctx> {
        debug_assert_eq!(inputs.len(), 1);
        debug_assert_eq!(variables.len(), 1);
        todo!("????????????????")
    }
}

impl WithObjectType for Sampler1d {
    const TYPE: ObjectType = ObjectType::new("sampler1d");
}
