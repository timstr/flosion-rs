use inkwell::types::{FloatType, IntType, PointerType, VoidType};

pub(crate) struct JitTypes<'ctx> {
    pub(crate) void_type: VoidType<'ctx>,
    pub(crate) pointer_type: PointerType<'ctx>,
    pub(crate) u8_type: IntType<'ctx>,
    pub(crate) u64_type: IntType<'ctx>,
    pub(crate) f32_type: FloatType<'ctx>,
    pub(crate) usize_type: IntType<'ctx>,
}

impl<'ctx> JitTypes<'ctx> {
    pub(super) fn new(
        address_space: inkwell::AddressSpace,
        execution_engine: &inkwell::execution_engine::ExecutionEngine<'ctx>,
        inkwell_context: &'ctx inkwell::context::Context,
    ) -> JitTypes<'ctx> {
        let target_data = execution_engine.get_target_data();

        let void_type = inkwell_context.void_type();
        let pointer_type = inkwell_context.ptr_type(address_space);
        let u8_type = inkwell_context.i8_type();
        let u64_type = inkwell_context.i64_type();
        let f32_type = inkwell_context.f32_type();
        let usize_type = inkwell_context.ptr_sized_int_type(target_data, Some(address_space));

        JitTypes {
            void_type,
            pointer_type,
            u8_type,
            u64_type,
            f32_type,
            usize_type,
        }
    }
}
