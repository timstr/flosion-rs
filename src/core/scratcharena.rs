use std::{
    cell::{Cell, Ref, RefCell, UnsafeCell},
    ops::{Deref, DerefMut},
};

// TODO: rewrite this whole thing, I don't like it anymore

struct Stack {
    data: UnsafeCell<Vec<f32>>,
    occupied: Cell<usize>,
}

pub struct ScratchSlice<'a> {
    data: *mut [f32],
    stack: Ref<'a, Stack>,
    offset: usize,
}

impl<'a> ScratchSlice<'a> {
    pub fn get(&self) -> &'a [f32] {
        unsafe { &*self.data }
    }

    pub fn get_mut(&mut self) -> &'a mut [f32] {
        unsafe { &mut *self.data }
    }
}

impl<'a> Drop for ScratchSlice<'a> {
    fn drop(&mut self) {
        let len = unsafe { (*self.data).len() };
        self.stack.return_slice(self.offset, len);
    }
}

impl<'a> Deref for ScratchSlice<'a> {
    type Target = [f32];
    fn deref(&self) -> &[f32] {
        self.get()
    }
}

impl<'a> DerefMut for ScratchSlice<'a> {
    fn deref_mut(&mut self) -> &mut [f32] {
        self.get_mut()
    }
}

impl Stack {
    fn new(size: usize) -> Stack {
        let mut data = Vec::<f32>::new();
        data.resize(size, 0.0);
        Stack {
            data: UnsafeCell::new(data),
            occupied: Cell::new(0),
        }
    }

    fn total_size(&self) -> usize {
        unsafe { (*self.data.get()).len() }
    }

    fn in_use(&self) -> bool {
        self.occupied.get() != 0
    }

    fn borrow_slice<'a>(size: usize, self_ref: Ref<'a, Stack>) -> Option<ScratchSlice<'a>> {
        // SAFETY: coming soon
        let data = unsafe { &mut *self_ref.data.get() };
        let total_size = data.len();
        let used_size = self_ref.occupied.get();
        let new_used_size = used_size + size;
        if new_used_size > total_size {
            return None;
        }
        let slice = &mut data[used_size..new_used_size];
        self_ref.occupied.set(new_used_size);
        Some(ScratchSlice {
            data: slice,
            stack: self_ref,
            offset: used_size,
        })
    }

    fn return_slice(&self, offset: usize, size: usize) {
        let used_size = self.occupied.get();
        if offset + size != used_size {
            panic!("A StackSlice was dropped out of order")
        }
        self.occupied.set(offset);
    }

    fn remaining_size(&self) -> usize {
        let used = self.occupied.get();
        let total_size = unsafe { (*self.data.get()).len() };
        debug_assert!(used <= total_size);
        return total_size - used;
    }
}

pub struct ScratchArena {
    primary_stack: RefCell<Stack>,
    overflow_stacks: RefCell<Vec<Stack>>,
    peak_size: Cell<usize>,
}

impl ScratchArena {
    pub fn new() -> ScratchArena {
        let size = 4096; // TODO: accept a size hint parameter
        ScratchArena {
            primary_stack: RefCell::new(Stack::new(size)),
            overflow_stacks: RefCell::new(Vec::new()),
            peak_size: Cell::new(size),
        }
    }

    fn active_stack_index(&self) -> usize {
        if self.primary_stack.borrow().remaining_size() > 0 {
            debug_assert!(self
                .overflow_stacks
                .borrow()
                .iter()
                .all(|s| s.remaining_size() == 0));
            return 0;
        }
        let index = match self
            .overflow_stacks
            .borrow()
            .iter()
            .position(|s| s.remaining_size() > 0)
        {
            Some(i) => i,
            None => return self.overflow_stacks.borrow().len(),
        };
        debug_assert!(self.overflow_stacks.borrow()[index..]
            .iter()
            .all(|s| s.remaining_size() == 0));
        index
    }

    fn total_size(&self) -> usize {
        let primary_size = self.primary_stack.borrow().total_size();
        let overflow_size: usize = self
            .overflow_stacks
            .borrow()
            .iter()
            .map(|s| s.total_size())
            .sum();
        primary_size + overflow_size
    }

    pub fn borrow_slice<'a>(&'a self, size: usize) -> ScratchSlice<'a> {
        let primary_stack = self.primary_stack.borrow();
        if size <= primary_stack.remaining_size() {
            return Stack::borrow_slice(size, primary_stack).unwrap();
        }
        let index = self.active_stack_index();
        debug_assert!(index >= 1);
        let slice;
        {
            let count = self.overflow_stacks.borrow().len();
            if index == count {
                self.peak_size.set(self.total_size() + size);
                self.overflow_stacks.borrow_mut().push(Stack::new(size));
            }
            let overflow = self.overflow_stacks.borrow();
            let stack_ref: Ref<'a, Stack> = Ref::map(overflow, |s| &s[index - 1]);
            slice = Stack::borrow_slice(size, stack_ref).unwrap()
        }
        slice
    }

    pub fn cleanup(&mut self) {
        if self.primary_stack.borrow().in_use()
            || self.overflow_stacks.borrow().iter().any(|s| s.in_use())
        {
            panic!("Attempted to clean up ScratchStorage while it was still in use")
        }
        self.overflow_stacks.borrow_mut().clear();
        if self.primary_stack.borrow().total_size() < self.peak_size.get() {
            *self.primary_stack.borrow_mut() = Stack::new(self.peak_size.get());
        }
    }
}
