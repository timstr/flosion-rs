use super::Argument;

pub struct FloatArgument(pub &'static str);

impl Argument for FloatArgument {
    type ValueType = f64;

    fn name(&self) -> &'static str {
        self.0
    }

    fn try_parse(s: &str) -> Option<f64> {
        s.parse().ok()
    }
}
