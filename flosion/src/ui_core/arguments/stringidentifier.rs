use super::Argument;

pub struct StringIdentifierArgument(pub &'static str);

impl Argument for StringIdentifierArgument {
    type ValueType = String;

    fn name(&self) -> &'static str {
        self.0
    }

    fn try_parse(s: &str) -> Option<String> {
        debug_assert!(s.len() > 0);
        let first_char = s.chars().next().unwrap();
        if !first_char.is_alphabetic() {
            return None;
        }
        if !s.chars().all(|c| c.is_alphanumeric()) {
            return None;
        }
        return Some(s.to_string());
    }
}
