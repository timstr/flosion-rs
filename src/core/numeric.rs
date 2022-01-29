pub fn apply_unary<F: Fn(f32) -> f32>(src: &[f32], f: F, dst: &mut [f32]) {
    let n = src.len();
    if dst.len() != n {
        panic!("Attempted to call apply_unary() on slices of different length");
    }
    unsafe {
        // SAFETY: unsafe is used here to index into both slices without
        // bounds checking. This code is safe because the slices are
        // only indexed into over the range 0..n, and both have been
        // guaranteed by the above check to have exactly this length
        for i in 0..n {
            let s = src.get_unchecked(i);
            let d = dst.get_unchecked_mut(i);
            *d = f(*s);
        }
    }
}
pub fn apply_unary_inplace<F: Fn(f32) -> f32>(src_dst: &mut [f32], f: F) {
    let n = src_dst.len();
    unsafe {
        // SAFETY: unsafe is used here to index into the slice without
        // bounds checking. This code is safe because the slice is
        // only indexed into over the range 0..n, where n is the length
        // of the slice.
        for i in 0..n {
            let sd = src_dst.get_unchecked_mut(i);
            *sd = f(*sd);
        }
    }
}

pub fn apply_binary<F: Fn(f32, f32) -> f32>(src1: &[f32], src2: &[f32], f: F, dst: &mut [f32]) {
    let n = src1.len();
    if src2.len() != n || dst.len() != n {
        panic!("Attempted to call apply_binary() on slices of different length");
    }
    unsafe {
        // SAFETY: unsafe is used here to index into all slices without
        // bounds checking. This code is safe because the slices are
        // only indexed into over the range 0..n, and all have been
        // guaranteed by the above check to have exactly this length
        for i in 0..n {
            let s1 = src1.get_unchecked(i);
            let s2 = src2.get_unchecked(i);
            let d = dst.get_unchecked_mut(i);
            *d = f(*s1, *s2);
        }
    }
}

pub fn apply_binary_inplace<F: Fn(f32, f32) -> f32>(src1_dst: &mut [f32], src2: &[f32], f: F) {
    let n = src1_dst.len();
    if src2.len() != n {
        panic!("Attempted to call apply_binary() on slices of different length");
    }
    unsafe {
        // SAFETY: unsafe is used here to index into all slices without
        // bounds checking. This code is safe because the slices are
        // only indexed into over the range 0..n, and all have been
        // guaranteed by the above check to have exactly this length
        for i in 0..n {
            let s1d = src1_dst.get_unchecked_mut(i);
            let s2 = src2.get_unchecked(i);
            *s1d = f(*s1d, *s2);
        }
    }
}

pub fn inclusive_scan<F: Fn(f32, f32) -> f32>(src: &[f32], f: F, dst: &mut [f32]) {
    let n = src.len();
    if dst.len() != n {
        panic!("Attempted to call inclusive_scan() on slices of different length");
    }
    if n == 0 {
        return;
    }
    unsafe {
        // SAFETY: unsafe is used here to index into both slices without
        // bounds checking. This code is safe because the slices are
        // only indexed into over the range 0..n, and both have been
        // guaranteed by the above check to have exactly this length.
        // Reading both slices at index zero is safe because their
        // length has been checked above to be non-zero.
        let mut prev = *src.get_unchecked(0);
        *dst.get_unchecked_mut(0) = prev;
        for i in 1..n {
            let s = src.get_unchecked(i);
            let d = dst.get_unchecked_mut(i);
            let x = f(prev, *s);
            *d = x;
            prev = x;
        }
    }
}

pub fn inclusive_scan_inplace<F: Fn(f32, f32) -> f32>(src_dst: &mut [f32], f: F) {
    let n = src_dst.len();
    if n == 0 {
        return;
    }
    unsafe {
        // SAFETY: unsafe is used here to index into the slice without
        // bounds checking. This code is safe because the slice is
        // only indexed into over the range 0..n, where n is the length
        // of the slice.
        // Reading the slice at index zero is safe because their
        // length has been checked above to be non-zero.
        let mut prev = *src_dst.get_unchecked(0);
        for i in 1..n {
            let d = src_dst.get_unchecked_mut(i);
            let x = f(prev, *d);
            *d = x;
            prev = x;
        }
    }
}

pub fn exclusive_scan<F: Fn(f32, f32) -> f32>(
    src: &[f32],
    previous_value: f32,
    f: F,
    dst: &mut [f32],
) {
    let n = src.len();
    if dst.len() != n {
        panic!("Attempted to call exclusive_scan() on slices of different length");
    }
    unsafe {
        // SAFETY: unsafe is used here to index into both slices without
        // bounds checking. This code is safe because the slices are
        // only indexed into over the range 0..n, and both have been
        // guaranteed by the above check to have exactly this length.
        let mut prev = previous_value;
        for i in 0..n {
            let s = src.get_unchecked(i);
            let d = dst.get_unchecked_mut(i);
            let x = f(prev, *s);
            *d = x;
            prev = x;
        }
    }
}

pub fn exclusive_scan_inplace<F: Fn(f32, f32) -> f32>(
    src_dst: &mut [f32],
    previous_value: f32,
    f: F,
) {
    let n = src_dst.len();
    unsafe {
        // SAFETY: unsafe is used here to index into the slice without
        // bounds checking. This code is safe because the slice is
        // only indexed into over the range 0..n, where n is the length
        // of the slice.
        // Reading the slice at index zero is safe because their
        // length has been checked above to be non-zero.
        let mut prev = previous_value;
        for i in 0..n {
            let d = src_dst.get_unchecked_mut(i);
            let x = f(prev, *d);
            *d = x;
            prev = x;
        }
    }
}

pub fn fill(dst: &mut [f32], value: f32) {
    dst.iter_mut().for_each(|x| *x = value);
}

pub fn copy(src: &[f32], dst: &mut [f32]) {
    apply_unary(src, |x| x, dst);
}

pub fn negate(src: &[f32], dst: &mut [f32]) {
    apply_unary(src, |x| -x, dst);
}

pub fn negate_inplace(src_dst: &mut [f32]) {
    apply_unary_inplace(src_dst, |x| -x);
}

pub fn add(src1: &[f32], src2: &[f32], dst: &mut [f32]) {
    apply_binary(src1, src2, |a, b| a + b, dst);
}

pub fn add_inplace(src1_dst: &mut [f32], src2: &[f32]) {
    apply_binary_inplace(src1_dst, src2, |a, b| a + b);
}

pub fn add_scalar(src: &[f32], scalar: f32, dst: &mut [f32]) {
    apply_unary(src, |x| x + scalar, dst);
}

pub fn add_scalar_inplace(src_dst: &mut [f32], scalar: f32) {
    apply_unary_inplace(src_dst, |x| x + scalar);
}

pub fn sub(src1: &[f32], src2: &[f32], dst: &mut [f32]) {
    apply_binary(src1, src2, |a, b| a - b, dst);
}

pub fn sub_inplace(src1_dst: &mut [f32], src2: &[f32]) {
    apply_binary_inplace(src1_dst, src2, |a, b| a - b);
}

pub fn sub_scalar(src: &[f32], scalar: f32, dst: &mut [f32]) {
    apply_unary(src, |x| x - scalar, dst);
}

pub fn sub_scalar_inplace(src_dst: &mut [f32], scalar: f32) {
    apply_unary_inplace(src_dst, |x| x - scalar);
}

pub fn rsub_scalar(src: &[f32], scalar: f32, dst: &mut [f32]) {
    apply_unary(src, |x| scalar - x, dst);
}

pub fn rsub_scalar_inplace(src_dst: &mut [f32], scalar: f32) {
    apply_unary_inplace(src_dst, |x| scalar - x);
}

pub fn mul(src1: &[f32], src2: &[f32], dst: &mut [f32]) {
    apply_binary(src1, src2, |a, b| a * b, dst);
}

pub fn mul_inplace(src1_dst: &mut [f32], src2: &[f32]) {
    apply_binary_inplace(src1_dst, src2, |a, b| a * b);
}

pub fn mul_scalar(src: &[f32], scalar: f32, dst: &mut [f32]) {
    apply_unary(src, |x| x * scalar, dst);
}

pub fn mul_scalar_inplace(src_dst: &mut [f32], scalar: f32) {
    apply_unary_inplace(src_dst, |x| x * scalar);
}

pub fn div(src1: &[f32], src2: &[f32], dst: &mut [f32]) {
    apply_binary(src1, src2, |a, b| a * b, dst);
}

pub fn div_inplace(src1_dst: &mut [f32], src2: &[f32]) {
    apply_binary_inplace(src1_dst, src2, |a, b| a * b);
}

pub fn div_scalar(src: &[f32], scalar: f32, dst: &mut [f32]) {
    let k = 1.0 / scalar;
    apply_unary(src, |x| k * x, dst);
}

pub fn div_scalar_inplace(src_dst: &mut [f32], scalar: f32) {
    let k = 1.0 / scalar;
    apply_unary_inplace(src_dst, |x| k * x);
}

pub fn rdiv_scalar(src: &[f32], scalar: f32, dst: &mut [f32]) {
    apply_unary(src, |x| scalar / x, dst);
}

pub fn rdiv_scalar_inplace(src_dst: &mut [f32], scalar: f32) {
    apply_unary_inplace(src_dst, |x| scalar / x);
}
