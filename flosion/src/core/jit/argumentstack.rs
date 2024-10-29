use std::cell::RefCell;

use inkwell::{
    types::{BasicType, FloatType, IntType, PointerType},
    values::{BasicValue, FloatValue, IntValue, PointerValue},
};

use crate::core::{jit::jit::Jit, sound::argument::ProcessorArgumentId};

#[repr(C, align(8))]
pub struct AlignedWord {
    data: [u8; 8],
}

impl AlignedWord {
    fn new() -> AlignedWord {
        AlignedWord { data: [0; 8] }
    }
}

struct StackStorage {
    data: Vec<AlignedWord>,
    argument_offsets: Vec<(ProcessorArgumentId, usize)>,
}

pub(crate) struct ArgumentStack {
    storage: RefCell<StackStorage>,
}

impl ArgumentStack {
    pub(crate) fn new() -> ArgumentStack {
        ArgumentStack {
            storage: RefCell::new(StackStorage {
                data: Vec::new(),
                argument_offsets: Vec::new(),
            }),
        }
    }

    pub(crate) fn view_at_bottom(&self) -> ArgumentStackView {
        ArgumentStackView {
            storage: &self.storage,
            argument_count: 0,
            data_length: 0,
        }
    }
}

#[derive(Copy, Clone)]
pub(crate) struct ArgumentStackView<'a> {
    storage: &'a RefCell<StackStorage>,
    argument_count: usize,
    data_length: usize,
}

impl<'a> ArgumentStackView<'a> {
    pub(crate) fn all_arguments(&self) -> Vec<ProcessorArgumentId> {
        self.storage.borrow().argument_offsets[..self.argument_count]
            .iter()
            .map(|(id, _)| *id)
            .collect()
    }

    pub(crate) unsafe fn find_argument_ptr(
        &self,
        argument_id: ProcessorArgumentId,
    ) -> Option<*const u8> {
        let storage = self.storage.borrow();
        let Some(first_word) = storage.data.first() else {
            return None;
        };
        let ptr_start: *const u8 = &first_word.data[0];
        storage.argument_offsets[..self.argument_count]
            .iter()
            .find_map(|(id, word_offset)| {
                if *id == argument_id {
                    let offset = 8 * word_offset;
                    Some(ptr_start.add(offset))
                } else {
                    None
                }
            })
    }

    pub(crate) fn push<T: JitArgumentPack>(
        &mut self,
        argument_id: ProcessorArgumentId,
        argument_pack: T,
    ) {
        let mut storage = self.storage.borrow_mut();

        // Discard all argument ids higher than the current view
        storage.argument_offsets.truncate(self.argument_count);

        // discard argument values higher than the current view
        storage.data.truncate(self.data_length);

        // push the argument id and the index to the top of the
        // data stack. This is the offset at which they will
        // be read from later.
        storage
            .argument_offsets
            .push((argument_id, self.data_length));

        // push the individual values
        argument_pack.store(&mut storage.data);

        self.argument_count += 1;
        self.data_length = storage.data.len();
    }
}

//---------------------------------------

pub trait JitArgumentValue {
    type InkwellValue<'ctx>: BasicValue<'ctx>;
    type InkwellType<'ctx>: BasicType<'ctx>;

    fn get_type<'ctx>(jit: &Jit<'ctx>) -> Self::InkwellType<'ctx>;

    fn store(&self, bytes: &mut [u8; 8]);

    fn generate_load_call<'ctx>(
        ptr: PointerValue<'ctx>,
        jit: &mut Jit<'ctx>,
    ) -> Self::InkwellValue<'ctx>;
}

pub trait JitArgumentPack {
    type InkwellValues<'ctx>;

    fn store(&self, storage: &mut Vec<AlignedWord>);

    fn generate_load_calls<'ctx>(
        ptr: PointerValue<'ctx>,
        jit: &mut Jit<'ctx>,
    ) -> Self::InkwellValues<'ctx>;
}

impl<T0> JitArgumentPack for (T0,)
where
    T0: JitArgumentValue,
{
    type InkwellValues<'ctx> = (T0::InkwellValue<'ctx>,);

    fn store(&self, storage: &mut Vec<AlignedWord>) {
        let mut w0 = AlignedWord::new();

        self.0.store(&mut w0.data);

        storage.push(w0);
    }

    fn generate_load_calls<'ctx>(
        ptr: PointerValue<'ctx>,
        jit: &mut Jit<'ctx>,
    ) -> Self::InkwellValues<'ctx> {
        let v0 = T0::generate_load_call(ptr, jit);
        (v0,)
    }
}

impl<T0, T1> JitArgumentPack for (T0, T1)
where
    T0: JitArgumentValue,
    T1: JitArgumentValue,
{
    type InkwellValues<'ctx> = (T0::InkwellValue<'ctx>, T1::InkwellValue<'ctx>);

    fn store(&self, storage: &mut Vec<AlignedWord>) {
        let mut w0 = AlignedWord::new();
        let mut w1 = AlignedWord::new();

        self.0.store(&mut w0.data);
        self.1.store(&mut w1.data);

        storage.push(w0);
        storage.push(w1);
    }

    fn generate_load_calls<'ctx>(
        ptr: PointerValue<'ctx>,
        jit: &mut Jit<'ctx>,
    ) -> Self::InkwellValues<'ctx> {
        let ptr0 = ptr;
        let ptr1 = unsafe {
            jit.builder()
                .build_gep(
                    // NOTE: using usize as the pointee type to
                    // perform pointer offsetting so that increments
                    // happen e.g. 8 bytes at a time for 64-bit
                    jit.types.usize_type,
                    ptr,
                    &[jit.types.usize_type.const_int(1, false)],
                    "ptr1",
                )
                .unwrap()
        };

        let v0 = T0::generate_load_call(ptr0, jit);
        let v1 = T1::generate_load_call(ptr1, jit);

        (v0, v1)
    }
}

//---------------------------------------

impl JitArgumentValue for f32 {
    type InkwellValue<'ctx> = FloatValue<'ctx>;
    type InkwellType<'ctx> = FloatType<'ctx>;

    fn get_type<'ctx>(jit: &Jit<'ctx>) -> Self::InkwellType<'ctx> {
        jit.types.f32_type
    }

    fn store(&self, bytes: &mut [u8; 8]) {
        let [b0, b1, b2, b3] = self.to_ne_bytes();
        bytes[0] = b0;
        bytes[1] = b1;
        bytes[2] = b2;
        bytes[3] = b3;
    }

    fn generate_load_call<'ctx>(
        ptr: PointerValue<'ctx>,
        jit: &mut Jit<'ctx>,
    ) -> Self::InkwellValue<'ctx> {
        jit.builder()
            .build_load(jit.types.f32_type, ptr, "val_f32")
            .unwrap()
            .into_float_value()
    }
}

impl JitArgumentValue for *const f32 {
    type InkwellValue<'ctx> = PointerValue<'ctx>;
    type InkwellType<'ctx> = PointerType<'ctx>;

    fn get_type<'ctx>(jit: &Jit<'ctx>) -> Self::InkwellType<'ctx> {
        jit.types.pointer_type
    }

    fn store(&self, bytes: &mut [u8; 8]) {
        let [b0, b1, b2, b3, b4, b5, b6, b7] = (*self as usize).to_ne_bytes();
        bytes[0] = b0;
        bytes[1] = b1;
        bytes[2] = b2;
        bytes[3] = b3;
        bytes[4] = b4;
        bytes[5] = b5;
        bytes[6] = b6;
        bytes[7] = b7;
    }

    fn generate_load_call<'ctx>(
        ptr: PointerValue<'ctx>,
        jit: &mut Jit<'ctx>,
    ) -> Self::InkwellValue<'ctx> {
        jit.builder()
            .build_load(jit.types.pointer_type, ptr, "ptr_val")
            .unwrap()
            .into_pointer_value()
    }
}

impl JitArgumentValue for usize {
    type InkwellValue<'ctx> = IntValue<'ctx>;
    type InkwellType<'ctx> = IntType<'ctx>;

    fn get_type<'ctx>(jit: &Jit<'ctx>) -> Self::InkwellType<'ctx> {
        jit.types.usize_type
    }

    fn store(&self, bytes: &mut [u8; 8]) {
        let [b0, b1, b2, b3, b4, b5, b6, b7] = self.to_ne_bytes();
        bytes[0] = b0;
        bytes[1] = b1;
        bytes[2] = b2;
        bytes[3] = b3;
        bytes[4] = b4;
        bytes[5] = b5;
        bytes[6] = b6;
        bytes[7] = b7;
    }

    fn generate_load_call<'ctx>(
        ptr: PointerValue<'ctx>,
        jit: &mut Jit<'ctx>,
    ) -> Self::InkwellValue<'ctx> {
        jit.builder()
            .build_load(jit.types.usize_type, ptr, "usize_val")
            .unwrap()
            .into_int_value()
    }
}
