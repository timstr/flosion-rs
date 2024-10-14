use std::{
    cell::RefCell,
    collections::HashMap,
    ops::{Deref, DerefMut},
    rc::Rc,
};

/// The integer base-2 logarithm, rounded up,
/// such that `1 >> ilog2(n)` gives the smallest
/// power of 2 that is equal to or greater than n
fn ilog2(mut size: usize) -> u8 {
    let mut i: u8 = 0;
    size = size.max(1) - 1;
    while size > 0 {
        size >>= 1;
        i += 1;
    }
    return i;
}

/// A handle around an owned slice of f32 that returns itself
/// to the ScratchArena when it is dropped, in order to save
/// on memory allocations when it is repeatedly requested.
pub struct BorrowedSlice {
    /// The data. Option is used so that it can be taken
    /// inside the drop() implementation.
    slice: Option<Box<[f32]>>,

    /// The apparent number of elements in the slice as
    /// given to the client. Internally, the slice may
    /// be larger.
    size: usize,

    /// Shared pointer to the queue where the slice
    /// will be returned when dropped.
    queue: Rc<RefCell<SliceQueue>>,
}

impl BorrowedSlice {
    /// Access the data
    fn get(&self) -> &[f32] {
        &(*self.slice.as_ref().unwrap())[0..self.size]
    }
    /// Access the data mutably
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
        // Return the allocated slice back to the queue
        // to avoid reallocating it next time it's requested.
        // This does not allocate if the queue's Vec already
        // has held this many elements and thus has sufficient
        // capacity to store this slice.
        self.queue
            .borrow_mut()
            .slices
            .push(self.slice.take().unwrap())
    }
}

/// A list of pre-allocated slices of f32, used to supply
/// requests for BorrowedSlice instances.
struct SliceQueue {
    slices: Vec<Box<[f32]>>,
}

impl SliceQueue {
    /// Creates a new SliceQueue that is empty
    fn new() -> SliceQueue {
        SliceQueue { slices: Vec::new() }
    }
}

/// ScratchArena is a pool of heap-allocated slices of f32,
/// designed for sharing and reuse and avoiding reallocation.
/// Individual slices are lazily allocated as needed to meet
/// requests, but once dropped, they are returned and made
/// available for future reuse. Thus, one or a few rounds of
/// allocations may be needed to 'warm up' the arena, after
/// which slices can be requested following the same existing
/// pattern without any additional memory allocation.
pub(crate) struct ScratchArena {
    // The different queues of pre-allocated slices,
    // by their integer log size rounded up.
    queues: RefCell<HashMap<u8, Rc<RefCell<SliceQueue>>>>,
}

impl ScratchArena {
    /// Creates a new scratch arena. Does not allocate any data yet.
    pub(crate) fn new() -> ScratchArena {
        ScratchArena {
            queues: RefCell::new(HashMap::new()),
        }
    }

    /// Request and receive an owned slice of f32 data of the given
    /// size. If a slice of similar length has previously been
    /// allocated and then dropped, it will get reused. The arena stores
    /// slices for different ranges of sizes and also stores multiple
    /// slices of the same length if they have been requested at
    /// overlapping times in the past. Otherwise, a new slice is
    /// allocated on the heap, but will be returned to the arena
    /// and available for reuse when it is dropped.
    pub(crate) fn borrow_slice(&self, size: usize) -> BorrowedSlice {
        // Allocate and index according to the next largest
        // power of 2
        let k = ilog2(size);
        let s = 1_usize << k;
        let mut qs = self.queues.borrow_mut();

        // If a similar size has been requested, use the existing
        // slice queue and do not allocate. Otherwise, allocate a
        // new (empty) slice queue.
        let q = qs
            .entry(k)
            .or_insert_with(|| Rc::new(RefCell::new(SliceQueue::new())));

        // If a slice is available in the queue, take it. Keep the
        // queue's Vec to retain its capacity when the slice is
        // returned later. Otherwise, allocate a new one/
        let s = match q.borrow_mut().slices.pop() {
            Some(s) => s,
            None => {
                let mut v = Vec::new();
                v.resize(s, 0.0);
                v.into_boxed_slice()
            }
        };

        // return a borrowed slice, pointing at the queue of the
        // correct size to which the vec will be returned when
        // the borrowed slice is dropped.
        BorrowedSlice {
            slice: Some(s),
            size,
            queue: Rc::clone(&q),
        }
    }
}
