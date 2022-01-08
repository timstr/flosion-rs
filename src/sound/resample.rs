pub fn resample_interleave<F: FnMut() -> (f32, f32)>(
    output: &mut [f32],
    mut get_next_input_sample: F,
    input_sample_rate: u32,
    output_sample_rate: u32,
) {
    // let ratio = (input_sample_rate as f32) / (output_sample_rate as f32);
    assert!(input_sample_rate > 0);
    assert!(output_sample_rate > 0);
    assert!(output.len() % 2 == 0);
    // let mut remainder: f32 = ratio;
    // TODO: implement something nicer than nearest neighbour
    for p in output.chunks_exact_mut(2) {
        // while remainder >= 1.0 {
        //     let _ = get_next_input_sample();
        //     remainder -= 1.0;
        // }
        let s = get_next_input_sample();
        p[0] = s.0;
        p[1] = s.1;
        // remainder += ratio;
    }
}
