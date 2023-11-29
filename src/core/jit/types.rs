use inkwell::types::{FloatType, IntType, PointerType, VoidType};

pub(crate) struct JitTypes<'ctx> {
    pub(crate) void_type: VoidType<'ctx>,
    pub(crate) u8_type: IntType<'ctx>,
    pub(crate) u8_pointer_type: PointerType<'ctx>,
    pub(crate) pointer_type: PointerType<'ctx>,
    pub(crate) f32_type: FloatType<'ctx>,
    pub(crate) f32_pointer_type: PointerType<'ctx>,
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
        let u8_type = inkwell_context.i8_type();
        let u8_pointer_type = u8_type.ptr_type(address_space);
        let pointer_type = u8_type.ptr_type(address_space);
        let f32_type = inkwell_context.f32_type();
        let f32_pointer_type = f32_type.ptr_type(address_space);
        let usize_type = inkwell_context.ptr_sized_int_type(target_data, Some(address_space));

        JitTypes {
            void_type,
            u8_type,
            u8_pointer_type,
            pointer_type,
            f32_type,
            f32_pointer_type,
            usize_type,
        }
    }
}
