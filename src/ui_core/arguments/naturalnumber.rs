use super::Argument;

pub struct NaturalNumberArgument(pub &'static str);

impl Argument for NaturalNumberArgument {
    type ValueType = usize;

    fn name(&self) -> &'static str {
        self.0
    }

    fn try_parse(s: &str) -> Option<Self::ValueType> {
        s.parse::<usize>().ok()
    }
}
