use std::any::Any;

struct AnyArgumentValue {
    value: Box<dyn ArgumentValue>,
}

impl AnyArgumentValue {
    fn new<T: ArgumentValue>(value: T) -> AnyArgumentValue {
        AnyArgumentValue {
            value: Box::new(value),
        }
    }
}

impl Clone for AnyArgumentValue {
    fn clone(&self) -> Self {
        AnyArgumentValue {
            value: self.value.box_clone(),
        }
    }
}

pub trait ArgumentValue: 'static {
    fn box_clone(&self) -> Box<dyn ArgumentValue>;

    fn as_any(&self) -> &dyn Any;
}

impl<T: 'static + Clone> ArgumentValue for T {
    fn box_clone(&self) -> Box<dyn ArgumentValue> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

pub trait Argument {
    type ValueType: ArgumentValue + Clone;

    fn name(&self) -> &'static str;

    fn try_parse(s: &str) -> Option<Self::ValueType>;
}

pub struct StringIdentifierArgument(pub &'static str);

pub struct FloatArgument(pub &'static str);

pub struct FloatRangeArgument(pub &'static str);

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

impl Argument for FloatArgument {
    type ValueType = f64;

    fn name(&self) -> &'static str {
        self.0
    }

    fn try_parse(s: &str) -> Option<f64> {
        s.parse().ok()
    }
}

impl Argument for FloatRangeArgument {
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

    fn try_parse(&self, s: &str) -> Option<AnyArgumentValue>;
}

impl<T: Argument> AnyArgument for T {
    fn name(&self) -> &str {
        Argument::name(self)
    }

    fn try_parse(&self, s: &str) -> Option<AnyArgumentValue> {
        if let Some(v) = <T as Argument>::try_parse(s) {
            Some(AnyArgumentValue::new(v))
        } else {
            None
        }
    }
}

#[derive(Clone)]
pub struct ParsedArguments {
    argument_values: Vec<(&'static str, AnyArgumentValue)>,
}

impl ParsedArguments {
    pub fn new_empty() -> ParsedArguments {
        ParsedArguments {
            argument_values: Vec::new(),
        }
    }

    fn add<T: ArgumentValue>(&mut self, name: &'static str, value: T) {
        self.argument_values
            .push((name, AnyArgumentValue::new(value)));
    }

    pub fn get<T: Argument>(&self, argument: &'static T) -> Option<T::ValueType> {
        let val: Option<&AnyArgumentValue> = self.argument_values.iter().find_map(|(name, val)| {
            if *name == argument.name() {
                Some(val)
            } else {
                None
            }
        });
        let Some(val) = val else {
            // no matching argument found by name
            return None;
        };
        let Some(val) = val.as_any().downcast_ref::<T::ValueType>() else {
            panic!("Parsed argument has the wrong type");
        };
        let val: T::ValueType = val.clone();
        Some(val)
    }
}

pub struct ArgumentList {
    arguments: Vec<&'static dyn AnyArgument>,
}

impl ArgumentList {
    pub fn new_empty() -> ArgumentList {
        ArgumentList {
            arguments: Vec::new(),
        }
    }

    pub fn add(mut self, argument: &'static dyn AnyArgument) -> ArgumentList {
        self.arguments.push(argument);
        self
    }

    fn try_parse_term(&self, term: &str) -> Option<(&'static str, AnyArgumentValue)> {
        for arg in &self.arguments {
            if let Some(v) = arg.try_parse(&term) {
                return Some((arg.name(), v));
            }
        }
        None
    }

    pub fn parse(&self, terms: Vec<String>) -> ParsedArguments {
        let mut parsed_arguments = ParsedArguments::new_empty();

        for term in terms {
            if let Some((name, value)) = self.try_parse_term(&term) {
                parsed_arguments.add(name, value);
            } else {
                println!(
                    "ArgumentSet warning: the term \"{}\" was not parsed as \
                    any argument",
                    term
                );
            }
        }
        parsed_arguments
    }
}
