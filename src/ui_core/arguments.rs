use std::any::Any;

pub struct AnyArgumentValue {
    value: Box<dyn ArgumentValue>,
}

impl AnyArgumentValue {
    pub fn new<T: ArgumentValue>(value: T) -> AnyArgumentValue {
        AnyArgumentValue {
            value: Box::new(value),
        }
    }

    fn downcast<T: ArgumentValue + Clone>(&self) -> Option<T> {
        self.value.as_any().downcast_ref::<T>().cloned()
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
        let val: &T = self;
        val
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

pub struct NaturalNumberArgument(pub &'static str);

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

impl Argument for NaturalNumberArgument {
    type ValueType = usize;

    fn name(&self) -> &'static str {
        self.0
    }

    fn try_parse(s: &str) -> Option<Self::ValueType> {
        s.parse::<usize>().ok()
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

    fn add(&mut self, name: &'static str, value: AnyArgumentValue) {
        if self
            .argument_values
            .iter()
            .find(|(n, _)| *n == name)
            .is_some()
        {
            panic!(
                "Adding a parsed argument value for name \"{}\" a second time",
                name
            );
        }
        self.argument_values.push((name, value));
    }

    pub fn add_or_replace<T: Argument>(
        mut self,
        argument: &'static T,
        value: T::ValueType,
    ) -> Self {
        if let Some(val) = self.argument_values.iter_mut().find_map(|(name, val)| {
            if *name == argument.name() {
                Some(val)
            } else {
                None
            }
        }) {
            *val = AnyArgumentValue::new(value);
        } else {
            self.argument_values
                .push((argument.name(), AnyArgumentValue::new(value)));
        }
        self
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
        Some(
            val.downcast::<T::ValueType>()
                .expect("Parsed argument has the wrong type"),
        )
    }

    #[cfg(test)]
    pub(super) fn values(&self) -> &[(&'static str, AnyArgumentValue)] {
        &self.argument_values
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
        if self
            .arguments
            .iter()
            .find(|a| a.name() == argument.name())
            .is_some()
        {
            panic!(
                "Adding an argument called \"{}\" to argument list for the second time",
                argument.name()
            );
        }
        self.arguments.push(argument);
        self
    }

    pub fn arguments(&self) -> &[&dyn AnyArgument] {
        &self.arguments
    }

    fn try_parse_term(
        term: &str,
        remaining_args: &mut Vec<&'static dyn AnyArgument>,
    ) -> Option<(&'static str, AnyArgumentValue)> {
        for (i, arg) in remaining_args.iter().enumerate() {
            if let Some(v) = arg.try_parse(&term) {
                let name = arg.name();
                remaining_args.remove(i);
                return Some((name, v));
            }
        }
        None
    }

    pub fn parse(&self, terms: Vec<String>) -> ParsedArguments {
        if self.arguments.is_empty() {
            return ParsedArguments::new_empty();
        }

        let mut parsed_arguments = ParsedArguments::new_empty();

        let mut remaining_arguments = self.arguments.clone();

        for term in terms {
            if let Some((name, value)) = Self::try_parse_term(&term, &mut remaining_arguments) {
                parsed_arguments.add(name, value);
            }
        }
        parsed_arguments
    }
}
