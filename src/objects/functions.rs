use crate::core::{
    context::Context,
    graphobject::{ObjectType, WithObjectType},
    numberinput::NumberInputHandle,
    numbersource::{NumberSource, PureNumberSource},
    numbersourcetools::NumberSourceTools,
    numeric,
};
use atomic_float::AtomicF32;
use std::sync::atomic::Ordering;

pub struct Constant {
    value: AtomicF32,
}

impl Constant {
    pub fn get_value(&self) -> f32 {
        self.value.load(Ordering::SeqCst)
    }

    pub fn set_value(&self, value: f32) {
        self.value.store(value, Ordering::SeqCst);
    }
}

impl NumberSource for Constant {
    fn eval(&self, dst: &mut [f32], _context: &Context) {
        numeric::fill(dst, self.value.load(Ordering::SeqCst));
    }
}

impl WithObjectType for Constant {
    const TYPE: ObjectType = ObjectType::new("constant");
}

impl PureNumberSource for Constant {
    fn new(_tools: &mut NumberSourceTools<'_>) -> Constant {
        Constant {
            value: AtomicF32::new(0.0),
        }
    }
}

macro_rules! unary_number_source {
    ($name: ident, $namestr: literal, $f: expr) => {
        pub struct $name {
            pub input: NumberInputHandle,
        }

        impl NumberSource for $name {
            fn eval(&self, dst: &mut [f32], context: &Context) {
                self.input.eval(dst, context);
                numeric::apply_unary_inplace(dst, $f);
            }
        }

        impl WithObjectType for $name {
            const TYPE: ObjectType = ObjectType::new($namestr);
        }

        impl PureNumberSource for $name {
            fn new(tools: &mut NumberSourceTools<'_>) -> $name {
                $name {
                    input: tools.add_number_input(),
                }
            }
        }
    };
}

macro_rules! binary_number_source {
    ($name: ident, $namestr: literal, $f: expr) => {
        pub struct $name {
            pub input_1: NumberInputHandle,
            pub input_2: NumberInputHandle,
        }

        impl NumberSource for $name {
            fn eval(&self, dst: &mut [f32], context: &Context) {
                self.input_1.eval(dst, context);
                let mut scratch_space = context.get_scratch_space(dst.len());
                self.input_2.eval(&mut scratch_space, context);
                numeric::apply_binary_inplace(dst, &scratch_space, $f);
            }
        }

        impl WithObjectType for $name {
            const TYPE: ObjectType = ObjectType::new($namestr);
        }

        impl PureNumberSource for $name {
            fn new(tools: &mut NumberSourceTools<'_>) -> $name {
                $name {
                    input_1: tools.add_number_input(),
                    input_2: tools.add_number_input(),
                }
            }
        }
    };
}

// TODO: ternary functions:
// muladd
// linear map

unary_number_source!(Negate, "negate", |x| -x);
unary_number_source!(Floor, "floor", |x| x.floor());
unary_number_source!(Ceil, "ceil", |x| x.ceil());
unary_number_source!(Round, "round", |x| x.round());
unary_number_source!(Trunc, "trunc", |x| x.trunc());
unary_number_source!(Fract, "fract", |x| x.fract());
unary_number_source!(Abs, "abs", |x| x.abs());
unary_number_source!(Signum, "signum", |x| x.signum());
unary_number_source!(Exp, "exp", |x| x.exp());
unary_number_source!(Exp2, "exp2", |x| x.exp2());
unary_number_source!(Exp10, "exp10", |x| (x * std::f32::consts::LN_10).exp());
unary_number_source!(Log, "log", |x| x.ln());
unary_number_source!(Log2, "log2", |x| x.log2());
unary_number_source!(Log10, "log10", |x| x.log10());
unary_number_source!(Cbrt, "cbrt", |x| x.cbrt());
unary_number_source!(Sin, "sin", |x| x.sin());
unary_number_source!(USin, "usin", |x| (x * std::f32::consts::TAU).sin());
unary_number_source!(Cos, "cos", |x| x.cos());
unary_number_source!(UCos, "ucos", |x| (x * std::f32::consts::TAU).cos());
unary_number_source!(Tan, "tan", |x| x.tan());
unary_number_source!(Asin, "asin", |x| x.asin());
unary_number_source!(Acos, "acos", |x| x.acos());
unary_number_source!(Atan, "atan", |x| x.atan());
unary_number_source!(Sinh, "sinh", |x| x.sinh());
unary_number_source!(Cosh, "cosh", |x| x.cosh());
unary_number_source!(Tanh, "tanh", |x| x.tanh());
unary_number_source!(Asinh, "asinh", |x| x.asinh());
unary_number_source!(Acosh, "acosh", |x| x.acosh());
unary_number_source!(Atanh, "atanh", |x| x.atanh());

binary_number_source!(Add, "add", |a, b| a + b);
binary_number_source!(Subtract, "subtract", |a, b| a - b);
binary_number_source!(Multiply, "multiply", |a, b| a * b);
binary_number_source!(Divide, "divide", |a, b| a / b);
binary_number_source!(Hypot, "hypot", |a, b| a.hypot(b));
binary_number_source!(Copysign, "copysign", |a, b| a.copysign(b));
binary_number_source!(Pow, "pow", |a, b| a.powf(b));
binary_number_source!(Atan2, "atan2", |a, b| a.atan2(b));
