use crate::core::{
    compilednumberinput::CodeGen,
    context::Context,
    graphobject::{ObjectInitialization, ObjectType, WithObjectType},
    numberinput::NumberInputHandle,
    numbersource::PureNumberSource,
    numbersourcetools::NumberSourceTools,
    numeric,
    serialization::Serializer,
};
use atomic_float::AtomicF32;
use inkwell::{intrinsics::Intrinsic, values::FloatValue};
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

impl PureNumberSource for Constant {
    fn new(_tools: NumberSourceTools<'_>, init: ObjectInitialization) -> Result<Self, ()> {
        let value = match init {
            // TODO: I don't like the hidden dependency on ConstantUi right here
            // for the argument name
            ObjectInitialization::Args(a) => a.get("value").as_float().unwrap_or(0.0),
            ObjectInitialization::Archive(mut d) => d.f32()?,
            ObjectInitialization::Default => 0.0,
        };
        Ok(Constant {
            value: AtomicF32::new(value),
        })
    }

    fn eval(&self, dst: &mut [f32], _context: &Context) {
        numeric::fill(dst, self.value.load(Ordering::SeqCst));
    }

    fn serialize(&self, mut serializer: Serializer) {
        serializer.f32(self.get_value());
    }

    fn compile<'ctx>(
        &self,
        codegen: &CodeGen<'ctx>,
        inputs: &[FloatValue<'ctx>],
    ) -> FloatValue<'ctx> {
        debug_assert!(inputs.is_empty());
        codegen
            .float_type()
            .const_float(self.value.load(Ordering::SeqCst) as f64)
    }
}

impl WithObjectType for Constant {
    const TYPE: ObjectType = ObjectType::new("constant");
}

enum LlvmImplementation {
    IntrinsicUnary(&'static str),
    ExpressionUnary(for<'a, 'b> fn(&'a CodeGen<'b>, FloatValue<'b>) -> FloatValue<'b>),
}

impl LlvmImplementation {
    fn compile<'ctx>(
        &self,
        codegen: &CodeGen<'ctx>,
        inputs: &[FloatValue<'ctx>],
    ) -> FloatValue<'ctx> {
        match self {
            LlvmImplementation::IntrinsicUnary(name) => {
                debug_assert_eq!(inputs.len(), 1);
                let input = inputs[0];
                // TODO: error handling
                let intrinsic = Intrinsic::find(name).unwrap();

                let decl =
                    intrinsic.get_declaration(codegen.module(), &[codegen.float_type().into()]);

                // TODO: error handling
                let decl = decl.unwrap();

                let callsiteval =
                    codegen
                        .builder()
                        .build_call(decl, &[input.into()], &format!("{}_call", name));

                // TODO: error handling
                callsiteval
                    .try_as_basic_value()
                    .left()
                    .unwrap()
                    .into_float_value()
            }
            LlvmImplementation::ExpressionUnary(f) => {
                debug_assert_eq!(inputs.len(), 1);
                let input = inputs[0];
                f(codegen, input)
            }
        }
    }
}

macro_rules! unary_number_source {
    ($name: ident, $namestr: literal, $f: expr, $llvm_impl: expr) => {
        pub struct $name {
            pub input: NumberInputHandle,
        }

        impl PureNumberSource for $name {
            fn new(
                mut tools: NumberSourceTools<'_>,
                _init: ObjectInitialization,
            ) -> Result<$name, ()> {
                Ok($name {
                    input: tools.add_number_input(),
                })
            }

            fn eval(&self, dst: &mut [f32], context: &Context) {
                self.input.eval(dst, context);
                numeric::apply_unary_inplace(dst, $f);
            }

            fn compile<'ctx>(
                &self,
                codegen: &CodeGen<'ctx>,
                inputs: &[FloatValue<'ctx>],
            ) -> FloatValue<'ctx> {
                let imp: LlvmImplementation = $llvm_impl;
                imp.compile(codegen, inputs)
            }
        }

        impl WithObjectType for $name {
            const TYPE: ObjectType = ObjectType::new($namestr);
        }
    };
}

macro_rules! binary_number_source {
    ($name: ident, $namestr: literal, $f: expr, $llvm_impl: expr) => {
        pub struct $name {
            pub input_1: NumberInputHandle,
            pub input_2: NumberInputHandle,
        }

        impl PureNumberSource for $name {
            fn new(
                mut tools: NumberSourceTools<'_>,
                _init: ObjectInitialization,
            ) -> Result<$name, ()> {
                Ok($name {
                    input_1: tools.add_number_input(),
                    input_2: tools.add_number_input(),
                })
            }

            fn eval(&self, dst: &mut [f32], context: &Context) {
                self.input_1.eval(dst, context);
                let mut scratch_space = context.get_scratch_space(dst.len());
                self.input_2.eval(&mut scratch_space, context);
                numeric::apply_binary_inplace(dst, &scratch_space, $f);
            }

            fn compile<'ctx>(
                &self,
                codegen: &CodeGen<'ctx>,
                inputs: &[FloatValue<'ctx>],
            ) -> FloatValue<'ctx> {
                let imp: LlvmImplementation = $llvm_impl;
                imp.compile(codegen, inputs)
            }
        }

        impl WithObjectType for $name {
            const TYPE: ObjectType = ObjectType::new($namestr);
        }
    };
}

// TODO: ternary functions:
// fma
// linear map / lerp

unary_number_source!(
    Negate,
    "negate",
    |x| -x,
    LlvmImplementation::ExpressionUnary(|codegen, x| { codegen.builder().build_float_neg(x, "x") })
);
unary_number_source!(
    Floor,
    "floor",
    |x| x.floor(),
    LlvmImplementation::IntrinsicUnary("llvm.floor")
);
// unary_number_source!(Ceil, "ceil", |x| x.ceil());
// unary_number_source!(Round, "round", |x| x.round());
// unary_number_source!(Trunc, "trunc", |x| x.trunc());
// unary_number_source!(Fract, "fract", |x| x.fract());
// unary_number_source!(Abs, "abs", |x| x.abs());
// unary_number_source!(Signum, "signum", |x| x.signum());
// unary_number_source!(Exp, "exp", |x| x.exp());
// unary_number_source!(Exp2, "exp2", |x| x.exp2());
// unary_number_source!(Exp10, "exp10", |x| (x * std::f32::consts::LN_10).exp());
// unary_number_source!(Log, "log", |x| x.ln());
// unary_number_source!(Log2, "log2", |x| x.log2());
// unary_number_source!(Log10, "log10", |x| x.log10());
// unary_number_source!(Cbrt, "cbrt", |x| x.cbrt());
// unary_number_source!(Sin, "sin", |x| x.sin());
// unary_number_source!(Cos, "cos", |x| x.cos());
// unary_number_source!(Tan, "tan", |x| x.tan());
// unary_number_source!(Asin, "asin", |x| x.asin());
// unary_number_source!(Acos, "acos", |x| x.acos());
// unary_number_source!(Atan, "atan", |x| x.atan());
// unary_number_source!(Sinh, "sinh", |x| x.sinh());
// unary_number_source!(Cosh, "cosh", |x| x.cosh());
// unary_number_source!(Tanh, "tanh", |x| x.tanh());
// unary_number_source!(Asinh, "asinh", |x| x.asinh());
// unary_number_source!(Acosh, "acosh", |x| x.acosh());
// unary_number_source!(Atanh, "atanh", |x| x.atanh());

// unary_number_source!(SineWave, "sinewave", |x| (x * std::f32::consts::TAU).sin());
// unary_number_source!(CosineWave, "cosinewave", |x| (x * std::f32::consts::TAU)
//     .cos());
// unary_number_source!(SquareWave, "squarewave", |x| {
//     if (x - x.floor()) >= 0.5 {
//         1.0
//     } else {
//         -1.0
//     }
// });
// unary_number_source!(SawWave, "sawwave", |x| 2.0 * (x - x.floor()) - 1.0);
// unary_number_source!(TriangleWave, "trianglewave", |x| 4.0
//     * (x - (x + 0.5).floor()).abs()
//     - 1.0);

// binary_number_source!(Add, "add", |a, b| a + b);
// binary_number_source!(Subtract, "subtract", |a, b| a - b);
// binary_number_source!(Multiply, "multiply", |a, b| a * b);
// binary_number_source!(Divide, "divide", |a, b| a / b);
// binary_number_source!(Hypot, "hypot", |a, b| a.hypot(b));
// binary_number_source!(Copysign, "copysign", |a, b| a.copysign(b));
// binary_number_source!(Pow, "pow", |a, b| a.powf(b));
// binary_number_source!(Atan2, "atan2", |a, b| a.atan2(b));
