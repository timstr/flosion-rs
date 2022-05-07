#[derive(Copy, Clone, Debug)]
pub struct GridSpan {
    // linear index of the first item
    start_index: usize,

    // Number of items in each consecutive group
    items_per_row: usize,

    // Number of items between the start of any two adjacent consecutive groups
    // if inserting elements, this may be any value
    // if iterating or erasing elements, this must be at least items_per_row
    row_stride: usize,

    // Number of consecutive groups
    num_rows: usize,
}

impl GridSpan {
    pub fn new(
        start_index: usize,
        items_per_row: usize,
        row_stride: usize,
        num_rows: usize,
    ) -> GridSpan {
        GridSpan {
            start_index,
            items_per_row,
            row_stride,
            num_rows,
        }
    }

    pub fn start_index(&self) -> usize {
        self.start_index
    }

    pub fn items_per_row(&self) -> usize {
        self.items_per_row
    }

    pub fn row_stride(&self) -> usize {
        self.row_stride
    }

    pub fn num_rows(&self) -> usize {
        self.num_rows
    }

    pub fn new_contiguous(index: usize, count: usize) -> GridSpan {
        GridSpan::new(index, count, 1, 1)
    }

    pub fn new_empty() -> GridSpan {
        GridSpan::new(0, 1, 1, 0)
    }

    pub fn offset(&self, additional_start_offset: usize) -> GridSpan {
        let mut gs = *self;
        gs.start_index += additional_start_offset;
        gs
    }

    pub fn inflate(&self, items_out_per_items_in: usize) -> GridSpan {
        GridSpan {
            start_index: self.start_index * items_out_per_items_in,
            items_per_row: self.start_index * items_out_per_items_in,
            row_stride: self.row_stride * items_out_per_items_in,
            num_rows: self.num_rows,
        }
    }

    pub fn num_items(&self) -> usize {
        self.items_per_row * self.num_rows
    }

    pub fn insert_with<T, F: Fn() -> T>(&self, data: Vec<T>, f: F) -> Vec<T> {
        if self.num_items() == 0 {
            return data;
        }
        debug_assert!(self.start_index <= data.len(), "Attempted to insert states into a Vec using grid span whose start index is out of range for that vec");
        let mut new_states = Vec::<T>::new();
        let old_len = data.len();
        new_states.reserve(old_len + self.num_items());
        let mut it = data.into_iter();
        for _ in 0..self.start_index() {
            new_states.push(it.next().unwrap());
        }
        for row in 0..self.num_rows {
            if row != 0 {
                for _gap_item in 0..self.row_stride {
                    new_states.push(it.next().unwrap());
                }
            }
            for _row_item in 0..self.items_per_row {
                new_states.push(f());
            }
        }
        loop {
            match it.next() {
                Some(s) => new_states.push(s),
                None => break,
            }
        }
        assert_eq!(new_states.len(), old_len + self.num_items(), "The number of states added by a grid span to a vec does not match the number of items in the grid span");
        new_states
    }

    pub fn erase<T>(&self, data: Vec<T>) -> Vec<T> {
        debug_assert!(self.row_stride >= self.items_per_row, "Attempted to erase states from a vec using a grid span whose row stride is larger than its items per row");
        debug_assert!(self.start_index <= data.len(), "Attempted to erase states from a vec using a grid span whose start index is out of range for that vec");
        if self.num_items() == 0 {
            return data;
        }
        let mut new_states = Vec::<T>::new();
        let old_len = data.len();
        debug_assert!(old_len >= self.num_items(), "Attempted to erase states from a vec using a grid span which contains too many items for that vec");
        new_states.reserve(old_len - self.num_items());
        let mut it = data.into_iter();
        for _ in 0..self.start_index() {
            new_states.push(it.next().unwrap());
        }
        let row_gap = self.row_stride - self.items_per_row;
        for row in 0..self.num_rows {
            if row != 0 {
                for _gap_item in 0..row_gap {
                    new_states.push(it.next().unwrap());
                }
            }
            for _row_item in 0..self.items_per_row {
                it.next().unwrap();
            }
        }
        loop {
            match it.next() {
                Some(s) => new_states.push(s),
                None => break,
            }
        }
        assert_eq!(new_states.len(), old_len - self.num_items(), "The number of states removed from a vec by a grid span does not match the number of items in the grid span");
        new_states
    }

    pub fn visit_with<T, F: Fn(&T)>(&self, data: &[T], f: F) {
        if self.num_items() == 0 {
            return;
        }
        debug_assert!(self.row_stride >= self.items_per_row, "Attempted to visit a slice of states using a grid span whose row stride is greater than or equal to its items per row");
        debug_assert!(self.start_index <= data.len(), "Attempted to visit a slice of states using a grid span whose start index is out of range for that slice");
        let data = &data[self.start_index..];
        let mut it = data.iter();
        let row_gap = self.row_stride - self.items_per_row;
        for row in 0..self.num_rows {
            if row != 0 {
                for _gap_item in 0..row_gap {
                    it.next().unwrap();
                }
            }
            for _row_item in 0..self.items_per_row {
                f(it.next().unwrap());
            }
        }
    }
}
