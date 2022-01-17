use crate::sound::{context::Context, numbersource::NumberSource};

pub struct Sine {
    // TODO
// input: XXX,
}

impl NumberSource for Sine {
    fn eval(&self, dst: &mut [f32], context: Context) {}
}
