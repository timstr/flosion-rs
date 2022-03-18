use std::iter;

#[derive(Copy, Clone, Debug)]
pub struct GridSpan {
    // linear index of the first item
    start_index: usize,

    // Number of items in each consecutive group
    items_per_row: usize,

    // Number of items between the start of any two adjacent consecutive groups
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
        debug_assert!(items_per_row > 0);
        debug_assert!(row_stride > 0);
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

    pub fn contains(&self, index: usize) -> bool {
        if index < self.start_index {
            return false;
        }
        let index = index - self.start_index;
        let inner_index = index % self.row_stride;
        let outer_index = index / self.row_stride;
        (inner_index < self.items_per_row) && (outer_index < self.num_rows)
    }

    pub fn last_index(&self) -> usize {
        debug_assert!(self.num_rows > 0);
        debug_assert!(self.items_per_row > 0);
        self.start_index + (self.row_stride * (self.num_rows - 1)) + self.items_per_row - 1
    }

    pub fn num_items(&self) -> usize {
        self.items_per_row * self.num_rows
    }

    pub fn insert_with<T, F: Fn() -> T>(&self, data: Vec<T>, f: F) -> Vec<T> {
        if self.num_items() == 0 {
            return data;
        }
        debug_assert!(self.start_index <= data.len());
        debug_assert!(self.last_index() <= data.len() + self.num_items());
        let mut new_states = Vec::<T>::new();
        let old_len = data.len();
        new_states.reserve(old_len + self.num_items());
        for (i, s) in data.into_iter().enumerate() {
            if (i >= self.start_index)
                && (i <= self.last_index())
                && (i - self.start_index) % self.row_stride == 0
            {
                new_states.extend(iter::repeat_with(&f).take(self.items_per_row));
            }
            new_states.push(s);
        }
        if self.last_index() == old_len {
            new_states.extend(iter::repeat_with(&f).take(self.items_per_row));
        }
        assert_eq!(new_states.len(), old_len + self.num_items());
        new_states
    }

    pub fn erase<T>(&self, data: Vec<T>) -> Vec<T> {
        if self.num_items() == 0 {
            return data;
        }
        data.into_iter()
            .enumerate()
            .filter_map(|(i, s)| if self.contains(i) { None } else { Some(s) })
            .collect()
    }
}
