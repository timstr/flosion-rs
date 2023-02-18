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
use inkwell::values::FloatValue;
use std::sync::{atomic::Ordering, Arc};

pub struct Constant {
    value: Arc<AtomicF32>,
}

impl Constant {
    pub fn get_value(&self) -> f32 {
        self.value.load(Ordering::SeqCst)
    }

    pub fn set_value(&self, value: f32) {
        self.value.store(value, Ordering::SeqCst);
    }
}

// TODO: consider renaming this to Variable
// TODO: consider adding a different Constant struct which compiles to a float constant instead of an atomic read
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
            value: Arc::new(AtomicF32::new(value)),
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
        codegen: &mut CodeGen<'ctx>,
        inputs: &[FloatValue<'ctx>],
    ) -> FloatValue<'ctx> {
        debug_assert!(inputs.is_empty());
        codegen.build_atomicf32_load(Arc::clone(&self.value))
    }
}

impl WithObjectType for Constant {
    const TYPE: ObjectType = ObjectType::new("constant");
}

enum LlvmImplementation {
    IntrinsicUnary(&'static str),
    ExpressionUnary(for<'a, 'b> fn(&'a mut CodeGen<'b>, FloatValue<'b>) -> FloatValue<'b>),
    ExpressionBinary(
        for<'a, 'b> fn(&'a mut CodeGen<'b>, FloatValue<'b>, FloatValue<'b>) -> FloatValue<'b>,
    ),
}

impl LlvmImplementation {
    fn compile<'ctx>(
        &self,
        codegen: &mut CodeGen<'ctx>,
        inputs: &[FloatValue<'ctx>],
    ) -> FloatValue<'ctx> {
        match self {
            LlvmImplementation::IntrinsicUnary(name) => {
                debug_assert_eq!(inputs.len(), 1);
                let input = inputs[0];
                codegen.build_unary_intrinsic_call(name, input)
            }
            LlvmImplementation::ExpressionUnary(f) => {
                debug_assert_eq!(inputs.len(), 1);
                let input = inputs[0];
                f(codegen, input)
            }
            LlvmImplementation::ExpressionBinary(f) => {
                debug_assert_eq!(inputs.len(), 2);
                let a = inputs[0];
                let b = inputs[1];
                f(codegen, a, b)
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
                codegen: &mut CodeGen<'ctx>,
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
                codegen: &mut CodeGen<'ctx>,
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
unary_number_source!(
    Ceil,
    "ceil",
    |x| x.ceil(),
    LlvmImplementation::IntrinsicUnary("llvm.ceil")
);
unary_number_source!(
    Round,
    "round",
    |x| x.round(),
    LlvmImplementation::IntrinsicUnary("llvm.round")
);
unary_number_source!(
    Trunc,
    "trunc",
    |x| x.trunc(),
    LlvmImplementation::IntrinsicUnary("llvm.trunc")
);
unary_number_source!(
    Fract,
    "fract",
    |x| x.fract(),
    LlvmImplementation::ExpressionUnary(|codegen, x| {
        let x_trunc = codegen.build_unary_intrinsic_call("llvm.trunc", x);
        codegen.builder().build_float_sub(x, x_trunc, "fract")
    })
);
unary_number_source!(
    Abs,
    "abs",
    |x| x.abs(),
    LlvmImplementation::IntrinsicUnary("llvm.fabs")
);
// unary_number_source!(Signum, "signum", |x| x.signum());
unary_number_source!(
    Exp,
    "exp",
    |x| x.exp(),
    LlvmImplementation::IntrinsicUnary("llvm.exp")
);
unary_number_source!(
    Exp2,
    "exp2",
    |x| x.exp2(),
    LlvmImplementation::IntrinsicUnary("llvm.exp2")
);
// unary_number_source!(Exp10, "exp10", |x| (x * std::f32::consts::LN_10).exp());
unary_number_source!(
    Log,
    "log",
    |x| x.ln(),
    LlvmImplementation::IntrinsicUnary("llvm.log")
);
unary_number_source!(
    Log2,
    "log2",
    |x| x.log2(),
    LlvmImplementation::IntrinsicUnary("llvm.log2")
);
unary_number_source!(
    Log10,
    "log10",
    |x| x.log10(),
    LlvmImplementation::IntrinsicUnary("llvm.log10")
);
unary_number_source!(
    Sqrt,
    "sqrt",
    |x| x.sqrt(),
    LlvmImplementation::IntrinsicUnary("llvm.sqrt")
);
// unary_number_source!(Cbrt, "cbrt", |x| x.cbrt());
unary_number_source!(
    Sin,
    "sin",
    |x| x.sin(),
    LlvmImplementation::IntrinsicUnary("llvm.sin")
);
unary_number_source!(
    Cos,
    "cos",
    |x| x.cos(),
    LlvmImplementation::IntrinsicUnary("llvm.cos")
);
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

unary_number_source!(
    SineWave,
    "sinewave",
    |x| (x * std::f32::consts::TAU).sin(),
    LlvmImplementation::ExpressionUnary(|codegen, x| {
        let tau = codegen.float_type().const_float(std::f64::consts::TAU);
        let tau_x = codegen.builder().build_float_mul(tau, x, "tau_x");
        let sin_tau_x = codegen.build_unary_intrinsic_call("llvm.sin", tau_x);
        sin_tau_x
    })
);
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

binary_number_source!(
    Add,
    "add",
    |a, b| a + b,
    LlvmImplementation::ExpressionBinary(|codegen, a, b| {
        codegen.builder().build_float_add(a, b, "sum")
    })
);
binary_number_source!(
    Subtract,
    "subtract",
    |a, b| a - b,
    LlvmImplementation::ExpressionBinary(|codegen, a, b| {
        codegen.builder().build_float_sub(a, b, "difference")
    })
);
binary_number_source!(
    Multiply,
    "multiply",
    |a, b| a * b,
    LlvmImplementation::ExpressionBinary(|codegen, a, b| {
        codegen.builder().build_float_mul(a, b, "product")
    })
);
binary_number_source!(
    Divide,
    "divide",
    |a, b| a / b,
    LlvmImplementation::ExpressionBinary(|codegen, a, b| {
        codegen.builder().build_float_div(a, b, "quotient")
    })
);
// binary_number_source!(Hypot, "hypot", |a, b| a.hypot(b));
// binary_number_source!(Copysign, "copysign", |a, b| a.copysign(b));
binary_number_source!(
    Pow,
    "pow",
    |a, b| a.powf(b),
    LlvmImplementation::ExpressionBinary(|codegen, a, b| {
        // x = a^b
        // x = e^(ln(a^b))
        // x = e^(b * ln(a))
        let ln_a = codegen.build_unary_intrinsic_call("llvm.log", a);
        let b_ln_a = codegen.builder().build_float_mul(b, ln_a, "b_ln_a");
        codegen.build_unary_intrinsic_call("llvm.exp", b_ln_a)
    })
);
// binary_number_source!(Atan2, "atan2", |a, b| a.atan2(b));
