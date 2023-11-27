use std::{
    cell::UnsafeCell,
    ops::Deref,
    sync::{
        atomic::{AtomicBool, AtomicU16, Ordering},
        Arc,
    },
};

struct SharedData<T> {
    data: UnsafeCell<Vec<T>>,
    stride: usize,
    slice_use_counts: Vec<AtomicU16>,
    current_index: AtomicU16,
    currently_writing: AtomicBool,
}

pub(crate) struct AtomicSlice<T> {
    shared_data: Arc<SharedData<T>>,
}

pub(crate) struct AtomicSliceReadGuard<'a, T> {
    slice: &'a [T],
    use_count: &'a AtomicU16,
}

impl<T: Default + Clone> AtomicSlice<T> {
    pub fn new(mut data: Vec<T>) -> AtomicSlice<T> {
        let stride = data.len();
        let pool_size = 2; // bare minimum is 2. Consider making this larger and profiling
        data.resize(stride * pool_size, T::default());
        let shared_data = SharedData {
            data: UnsafeCell::new(data),
            stride,
            slice_use_counts: (0..pool_size).map(|_| AtomicU16::new(0)).collect(),
            current_index: AtomicU16::new(0),
            currently_writing: AtomicBool::new(false),
        };
        AtomicSlice {
            shared_data: Arc::new(shared_data),
        }
    }

    pub fn read<'a>(&'a self) -> AtomicSliceReadGuard<'a, T> {
        let i = self.shared_data.current_index.load(Ordering::SeqCst) as usize;
        let use_count = &self.shared_data.slice_use_counts[i];
        // AHHHHHHHHHHHHHHHHH right here seems to be the mistake! If the writer updates the the current
        // index and then checks the use count right here, it could see zero and proceed to write over
        // the data without being synchronized properly.
        // The writer must be prevented from observing this intermediate state, or this intermediate
        // state must be removed or else made unproblematic.
        // Ideally, getting the index and marking it as in use would be one and only one operation.
        // How to do???
        // Maybe some exotic trickery with low and high order bits?
        // Actually, that seems fine, assuming that there are a smallish number of readers and the
        // pool is smallish in size
        //
        // Consider:
        // - pool size is always two
        // - use count of both slices and which slice is active are all stored packed into one big atomic integer
        // - when fetching the active slice, use a fetch_add to simultaneously increment use counts for *BOTH*
        //   slices. Then as a second step, decrement the use count of the inactive slice
        // - This way, the reader simultaneously retrieves and locks the active slice, and there is no longer
        //   an intermediate step in which the current slice is loaded but not yet locked
        use_count.fetch_add(1, Ordering::SeqCst);
        let stride = self.shared_data.stride;
        let offset = i * stride;
        let slice: &[T] = unsafe {
            let ptr_vec = self.shared_data.data.get();
            let ptr_data = (*ptr_vec).as_ptr();
            let ptr_begin = ptr_data.add(offset);
            std::slice::from_raw_parts(ptr_begin, stride)
        };
        AtomicSliceReadGuard { slice, use_count }
    }

    pub fn write(&self, data: &[T]) {
        let stride = self.shared_data.stride;
        if data.len() != stride {
            panic!("Attempted to write slice of the wrong length to AtomicSlice");
        }

        while self
            .shared_data
            .currently_writing
            .swap(true, Ordering::SeqCst)
        {
            std::hint::spin_loop();
        }
        // A useful precondition here would be that a slice exists which no current readers are accessing
        // and which no future readers will access until after the atomic current index is written to to
        // point to it. If that was given, then that slice would be the logical place to write new data
        // to and assign the new current index to. To maintain that precondition for the next write,
        // this needs to be a postcondition. So then, assuming that a completely unused slice exists,
        // how to guarantee the existence of another one by the end of this function? In other words,
        // how to guarantee by the end of this function that all readers are done with the old index (or
        // some other index) and will always read the new index?
        //
        // Why do problems only appear to manifest when there is more than one writer?
        let i = self.shared_data.current_index.load(Ordering::SeqCst) as usize;
        let next_i = (i + 1) % self.shared_data.slice_use_counts.len();
        // let prev_use_count = &self.shared_data.slice_use_counts[i];
        // let next_use_count = &self.shared_data.slice_use_counts[next_i];
        // while next_use_count.load(Ordering::SeqCst) != 0 {
        //     std::hint::spin_loop();
        // }
        let offset = i * stride;
        let slice: &mut [T] = unsafe {
            let ptr_vec = self.shared_data.data.get();
            let ptr_data = (*ptr_vec).as_mut_ptr();
            let ptr_begin = ptr_data.add(offset);
            std::slice::from_raw_parts_mut(ptr_begin, stride)
        };
        for (i, v) in slice.iter_mut().enumerate() {
            *v = data[i].clone();
        }
        self.shared_data
            .current_index
            .store(next_i as u16, Ordering::SeqCst);

        // while prev_use_count.load(Ordering::SeqCst) != 0 {
        //     std::hint::spin_loop();
        // }

        self.shared_data
            .currently_writing
            .store(false, Ordering::SeqCst);
    }
}

impl<T> Clone for AtomicSlice<T> {
    fn clone(&self) -> Self {
        Self {
            shared_data: Arc::clone(&self.shared_data),
        }
    }
}

unsafe impl<T: Send> Sync for AtomicSlice<T> {}
unsafe impl<T: Send> Send for AtomicSlice<T> {}

impl<'a, T> Deref for AtomicSliceReadGuard<'a, T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        self.slice
    }
}

impl<'a, T> Drop for AtomicSliceReadGuard<'a, T> {
    fn drop(&mut self) {
        self.use_count.fetch_sub(1, Ordering::SeqCst);
    }
}

#[cfg(test)]
mod test {
    use super::AtomicSlice;

    #[test]
    fn test_atomic_slice_u8() {
        let length: usize = 16;
        let num_readers = 4;
        let num_writers = 4;
        let num_iterations = 10_000_000;

        let mut data = Vec::<u8>::new();
        data.resize(length, 0);
        let atomic_slice = AtomicSlice::new(data);

        let readers: Vec<std::thread::JoinHandle<()>> = (0..num_readers)
            .map(|i_reader| {
                let atomic_slice = atomic_slice.clone();
                std::thread::spawn(move || {
                    println!("Reader {} starting", i_reader);
                    for iter in 0..num_iterations {
                        // Read the slice and assert that its length is as expected and that all values are the same
                        let guard = atomic_slice.read();
                        let slice: &[u8] = &*guard;
                        assert_eq!(slice.len(), length);
                        let first_value = slice[0];
                        for other_value in slice[1..].iter().cloned() {
                            assert_eq!(
                                first_value, other_value,
                                "Reader {} encountered a slice with mis-matched values {} != {} on iteration {}: {:?}",
                                i_reader, first_value, other_value, iter, slice
                            );
                        }
                        // println!("reader {}: all {}", i_reader, v0);
                    }
                    println!("Reader {} done", i_reader);
                })
            })
            .collect();

        let writers: Vec<std::thread::JoinHandle<()>> = (0..num_writers)
            .map(|i_writer| {
                let atomic_slice = atomic_slice.clone();
                std::thread::spawn(move || {
                    println!("Writer {} starting", i_writer);
                    let mut data = Vec::<u8>::new();
                    data.resize(length, 0);
                    for _ in 0..num_iterations {
                        // Write an array of identical values to the slice
                        data.fill(i_writer);
                        atomic_slice.write(&data);
                        // println!("writer {} wrote all {}", i_writer, i_writer);
                    }
                    println!("Writer {} done", i_writer);
                })
            })
            .collect();

        for t in readers {
            t.join().unwrap();
        }
        for t in writers {
            t.join().unwrap();
        }
    }
}
