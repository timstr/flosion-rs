use std::any::Any;

pub trait Argument {
    type ValueType: Any;

    fn name(&self) -> &'static str;

    fn try_parse(s: &str) -> Option<Self::ValueType>;
}

pub struct StringIdentifier(pub &'static str);

pub struct Float(pub &'static str);

pub struct FloatRange(pub &'static str);

impl Argument for StringIdentifier {
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

impl Argument for Float {
    type ValueType = f64;

    fn name(&self) -> &'static str {
        self.0
    }

    fn try_parse(s: &str) -> Option<f64> {
        s.parse().ok()
    }
}

impl Argument for FloatRange {
    type ValueType = std::ops::RangeInclusive<f64>;

    fn name(&self) -> &'static str {
        self.0
    }

    fn try_parse(s: &str) -> Option<std::ops::RangeInclusive<f64>> {
        let mut parts = s.split("..");
        let Some(start) = parts.next() else {
            return None;
        };
        let Some(end) = parts.next() else {
            return None;
        };
        if parts.next().is_some() {
            return None;
        }
        let Ok(start) = start.parse::<f64>() else {
            return None;
        };
        let Ok(end) = end.parse::<f64>() else {
            return None;
        };
        Some(start..=end)
    }
}

pub trait AnyArgument {
    fn name(&self) -> &str;

    fn try_parse(s: &str) -> Option<Box<dyn Any>>;
}

impl<T: Argument> AnyArgument for T {
    fn name(&self) -> &str {
        Argument::name(self)
    }

    fn try_parse(s: &str) -> Option<Box<dyn Any>> {
        Argument::try_parse(s).and_then(Box::new)
    }
}

pub struct ParsedArguments {
    argument_values: Vec<(&'static str, Box<dyn Any>)>,
}

impl ParsedArguments {
    fn new() -> ParsedArguments {
        ParsedArguments {
            argument_values: Vec::new(),
        }
    }

    fn add(&mut self, name: &'static str, value: Box<dyn Any>) {
        self.argument_values.push((name, value));
    }

    pub fn get<T: Argument>(&self, argument: &'static T) -> Option<&T::ValueType> {
        let val: Option<&dyn Any> = self.argument_values.iter().find_map(|(name, val)| {
            if *name == argument.name() {
                Some(&**val)
            } else {
                None
            }
        });
        let Some(val) = val else {
            // no matching argument found by name
            return None;
        };
        let Some(val) = val.downcast_ref() else {
            panic!("Parsed argument has the wrong type");
        };
        Some(val)
    }
}

pub struct ArgumentSet {
    arguments: Vec<&'static dyn AnyArgument>,
}

impl ArgumentSet {
    pub fn new_empty() -> ArgumentSet {
        ArgumentSet {
            arguments: Vec::new(),
        }
    }

    pub fn add(mut self, argument: &'static dyn AnyArgument) -> ArgumentSet {
        self.arguments.push(argument);
        self
    }

    pub fn parse(terms: Vec<String>) -> ParsedArguments {
        // let mut parsed_arguments = ParsedArguments
        todo!()
    }
}
