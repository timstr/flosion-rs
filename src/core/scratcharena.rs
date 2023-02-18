use std::{
    cell::RefCell,
    collections::HashMap,
    ops::{Deref, DerefMut},
    rc::Rc,
};

fn ilog2(mut size: usize) -> u8 {
    let mut i: u8 = 0;
    size = size.max(1) - 1;
    while size > 0 {
        size >>= 1;
        i += 1;
    }
    return i;
}

pub struct BorrowedSlice {
    slice: Option<Box<[f32]>>,
    size: usize,
    queue: Rc<RefCell<SliceQueue>>,
}

impl BorrowedSlice {
    fn get(&self) -> &[f32] {
        &(*self.slice.as_ref().unwrap())[0..self.size]
    }
    fn get_mut(&mut self) -> &mut [f32] {
        &mut (*self.slice.as_mut().unwrap())[0..self.size]
    }
}

impl Deref for BorrowedSlice {
    type Target = [f32];

    fn deref(&self) -> &Self::Target {
        self.get()
    }
}

impl DerefMut for BorrowedSlice {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.get_mut()
    }
}

impl Drop for BorrowedSlice {
    fn drop(&mut self) {
        self.queue
            .borrow_mut()
            .slices
            .push(self.slice.take().unwrap())
    }
}

struct SliceQueue {
    slices: Vec<Box<[f32]>>,
}

impl SliceQueue {
    fn new() -> SliceQueue {
        SliceQueue { slices: Vec::new() }
    }
}

pub(crate) struct ScratchArena {
    queues: RefCell<HashMap<u8, Rc<RefCell<SliceQueue>>>>,
}

impl ScratchArena {
    pub(crate) fn new() -> ScratchArena {
        ScratchArena {
            queues: RefCell::new(HashMap::new()),
        }
    }

    pub(crate) fn borrow_slice(&self, size: usize) -> BorrowedSlice {
        let k = ilog2(size);
        let s = 1_usize << k;
        let mut qs = self.queues.borrow_mut();
        let q = qs
            .entry(k)
            .or_insert_with(|| Rc::new(RefCell::new(SliceQueue::new())));
        let s = match q.borrow_mut().slices.pop() {
            Some(s) => s,
            None => {
                let mut v = Vec::new();
                v.resize(s, 0.0);
                v.into_boxed_slice()
            }
        };
        BorrowedSlice {
            slice: Some(s),
            size,
            queue: Rc::clone(&q),
        }
    }
}
