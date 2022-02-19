#[derive(Clone)]
pub enum ArgumentValue {
    String(String),
    Float(f32),
}

impl ArgumentValue {
    pub fn as_float(&self) -> f32 {
        match self {
            ArgumentValue::Float(v) => *v,
            ArgumentValue::String(_) => panic!(),
        }
    }

    pub fn as_string(&self) -> &String {
        match self {
            ArgumentValue::String(s) => s,
            ArgumentValue::Float(_) => panic!(),
        }
    }
}

pub struct Argument {
    name: &'static str,
    default: ArgumentValue,
}

impl Argument {
    fn new(name: &'static str, default: ArgumentValue) -> Argument {
        Argument { name, default }
    }

    pub fn name(&self) -> &str {
        self.name
    }

    fn parse(&self, contents: &str) -> ArgumentValue {
        match &self.default {
            ArgumentValue::Float(f) => ArgumentValue::Float(contents.parse::<f32>().unwrap_or(*f)),
            ArgumentValue::String(_) => ArgumentValue::String(contents.to_string()),
        }
    }
}

pub struct ArgumentList {
    arguments: Vec<Argument>,
}

impl ArgumentList {
    pub fn new() -> ArgumentList {
        ArgumentList {
            arguments: Vec::new(),
        }
    }

    pub fn items(&self) -> &[Argument] {
        &self.arguments
    }

    pub fn add(&mut self, name: &'static str, default: ArgumentValue) {
        self.arguments.push(Argument::new(name, default));
    }

    pub fn parse(&self, contents: &[&str]) -> ParsedArguments {
        let mut argument_values: Vec<(&'static str, ArgumentValue)> = Vec::new();
        let mut arg_iter = self.arguments.iter();
        let mut arg = arg_iter.next();
        let mut part_iter = contents.iter();
        let mut part = part_iter.next();
        loop {
            if let Some(a) = arg {
                argument_values.push((
                    a.name,
                    if let Some(p) = part {
                        part = part_iter.next();
                        a.parse(p)
                    } else {
                        a.default.clone()
                    },
                ));
                arg = arg_iter.next();
            } else {
                break;
            }
        }
        ParsedArguments { argument_values }
    }
}

pub struct ParsedArguments {
    argument_values: Vec<(&'static str, ArgumentValue)>,
}

impl ParsedArguments {
    pub fn get(&self, name: &'static str) -> &ArgumentValue {
        &self
            .argument_values
            .iter()
            .find(|(n, _)| *n == name)
            .unwrap()
            .1
    }
}
