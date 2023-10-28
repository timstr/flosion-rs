use crate::{
    core::{
        graph::graphobject::{ObjectInitialization, ObjectType, WithObjectType},
        jit::codegen::CodeGen,
        number::{
            numberinput::NumberInputHandle, numbersource::PureNumberSource,
            numbersourcetools::NumberSourceTools,
        },
    },
    ui_core::arguments::FloatArgument,
};
use atomic_float::AtomicF32;
use inkwell::{values::FloatValue, FloatPredicate};
use serialization::Serializer;
use std::sync::{atomic::Ordering, Arc};

pub struct Constant {
    value: f32,
}

impl Constant {
    pub fn value(&self) -> f32 {
        self.value
    }

    pub const ARG_VALUE: FloatArgument = FloatArgument("value");
}

impl PureNumberSource for Constant {
    fn new(_tools: NumberSourceTools<'_>, init: ObjectInitialization) -> Result<Self, ()> {
        let value = match init {
            // ObjectInitialization::Args(a) => a.get("value").as_float().unwrap_or(0.0),
            ObjectInitialization::Archive(mut d) => d.f32()?,
            ObjectInitialization::Default => 0.0,
            ObjectInitialization::Arguments(args) => {
                args.get(&Constant::ARG_VALUE).unwrap_or(0.0) as f32
            }
        };
        Ok(Constant { value })
    }

    fn serialize(&self, mut serializer: Serializer) {
        serializer.f32(self.value);
    }

    fn compile<'ctx>(
        &self,
        codegen: &mut CodeGen<'ctx>,
        inputs: &[FloatValue<'ctx>],
    ) -> FloatValue<'ctx> {
        debug_assert!(inputs.is_empty());
        codegen.float_type().const_float(self.value as f64)
    }
}

impl WithObjectType for Constant {
    const TYPE: ObjectType = ObjectType::new("constant");
}

pub struct Variable {
    value: Arc<AtomicF32>,
}

impl Variable {
    pub fn get_value(&self) -> f32 {
        self.value.load(Ordering::SeqCst)
    }

    pub fn set_value(&self, value: f32) {
        self.value.store(value, Ordering::SeqCst);
    }

    pub const ARG_VALUE: FloatArgument = FloatArgument("value");
}

impl PureNumberSource for Variable {
    fn new(_tools: NumberSourceTools<'_>, init: ObjectInitialization) -> Result<Self, ()> {
        let value = match init {
            // ObjectInitialization::Args(a) => a.get("value").as_float().unwrap_or(0.0),
            ObjectInitialization::Archive(mut d) => d.f32()?,
            ObjectInitialization::Default => 0.0,
            ObjectInitialization::Arguments(args) => {
                args.get(&Variable::ARG_VALUE).unwrap_or(0.0) as f32
            }
        };
        Ok(Variable {
            value: Arc::new(AtomicF32::new(value)),
        })
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

impl WithObjectType for Variable {
    const TYPE: ObjectType = ObjectType::new("variable");
}

enum LlvmImplementation {
    IntrinsicUnary(&'static str),
    IntrinsicBinary(&'static str),
    ExpressionUnary(for<'a, 'b> fn(&'a mut CodeGen<'b>, FloatValue<'b>) -> FloatValue<'b>),
    ExpressionBinary(
        for<'a, 'b> fn(&'a mut CodeGen<'b>, FloatValue<'b>, FloatValue<'b>) -> FloatValue<'b>,
    ),
    ExpressionTernary(
        for<'a, 'b> fn(
            &'a mut CodeGen<'b>,
            FloatValue<'b>,
            FloatValue<'b>,
            FloatValue<'b>,
        ) -> FloatValue<'b>,
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
            LlvmImplementation::IntrinsicBinary(name) => {
                debug_assert_eq!(inputs.len(), 2);
                let input1 = inputs[0];
                let input2 = inputs[1];
                codegen.build_binary_intrinsic_call(name, input1, input2)
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
            LlvmImplementation::ExpressionTernary(f) => {
                debug_assert_eq!(inputs.len(), 3);
                let a = inputs[0];
                let b = inputs[1];
                let c = inputs[2];
                f(codegen, a, b, c)
            }
        }
    }
}

macro_rules! unary_number_source {
    ($name: ident, $namestr: literal, $default_input: expr, $f: expr, $llvm_impl: expr) => {
        pub struct $name {
            pub input: NumberInputHandle,
        }

        impl PureNumberSource for $name {
            fn new(
                mut tools: NumberSourceTools<'_>,
                _init: ObjectInitialization,
            ) -> Result<$name, ()> {
                let default_value: f32 = $default_input;
                Ok($name {
                    input: tools.add_number_input(default_value),
                })
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
    ($name: ident, $namestr: literal, $default_inputs: expr, $f: expr, $llvm_impl: expr) => {
        pub struct $name {
            pub input_1: NumberInputHandle,
            pub input_2: NumberInputHandle,
        }

        impl PureNumberSource for $name {
            fn new(
                mut tools: NumberSourceTools<'_>,
                _init: ObjectInitialization,
            ) -> Result<$name, ()> {
                let default_values: (f32, f32) = $default_inputs;
                Ok($name {
                    input_1: tools.add_number_input(default_values.0),
                    input_2: tools.add_number_input(default_values.1),
                })
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

macro_rules! ternary_number_source {
    ($name: ident, $namestr: literal, $default_inputs: expr, $f: expr, $llvm_impl: expr) => {
        pub struct $name {
            pub input_1: NumberInputHandle,
            pub input_2: NumberInputHandle,
            pub input_3: NumberInputHandle,
        }

        impl PureNumberSource for $name {
            fn new(
                mut tools: NumberSourceTools<'_>,
                _init: ObjectInitialization,
            ) -> Result<$name, ()> {
                let default_values: (f32, f32, f32) = $default_inputs;
                Ok($name {
                    input_1: tools.add_number_input(default_values.0),
                    input_2: tools.add_number_input(default_values.1),
                    input_3: tools.add_number_input(default_values.2),
                })
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

// TODO
// fma

unary_number_source!(
    Negate,
    "negate",
    0.0,
    |x| -x,
    LlvmImplementation::ExpressionUnary(|codegen, x| { codegen.builder().build_float_neg(x, "x") })
);
unary_number_source!(
    Floor,
    "floor",
    0.0,
    |x| x.floor(),
    LlvmImplementation::IntrinsicUnary("llvm.floor")
);
unary_number_source!(
    Ceil,
    "ceil",
    0.0,
    |x| x.ceil(),
    LlvmImplementation::IntrinsicUnary("llvm.ceil")
);
unary_number_source!(
    Round,
    "round",
    0.0,
    |x| x.round(),
    LlvmImplementation::IntrinsicUnary("llvm.round")
);
unary_number_source!(
    Trunc,
    "trunc",
    0.0,
    |x| x.trunc(),
    LlvmImplementation::IntrinsicUnary("llvm.trunc")
);
unary_number_source!(
    Fract,
    "fract",
    0.0,
    |x| x.fract(),
    LlvmImplementation::ExpressionUnary(|codegen, x| {
        let x_trunc = codegen.build_unary_intrinsic_call("llvm.trunc", x);
        codegen.builder().build_float_sub(x, x_trunc, "fract")
    })
);
unary_number_source!(
    Abs,
    "abs",
    0.0,
    |x| x.abs(),
    LlvmImplementation::IntrinsicUnary("llvm.fabs")
);
unary_number_source!(
    Signum,
    "signum",
    0.0,
    |x| x.signum(),
    LlvmImplementation::ExpressionUnary(|codegen, x| {
        let one = codegen.float_type().const_float(1.0);
        codegen.build_binary_intrinsic_call("llvm.copysign", one, x)
    })
);
unary_number_source!(
    Exp,
    "exp",
    0.0,
    |x| x.exp(),
    LlvmImplementation::IntrinsicUnary("llvm.exp")
);
unary_number_source!(
    Exp2,
    "exp2",
    0.0,
    |x| x.exp2(),
    LlvmImplementation::IntrinsicUnary("llvm.exp2")
);
unary_number_source!(
    Exp10,
    "exp10",
    0.0,
    |x| (x * std::f32::consts::LN_10).exp(),
    LlvmImplementation::ExpressionUnary(|codegen, x| {
        let ln_10 = codegen
            .float_type()
            .const_float(std::f32::consts::LN_10 as f64);
        let x_times_ln_10 = codegen.builder().build_float_mul(x, ln_10, "x_times_ln_10");
        codegen.build_unary_intrinsic_call("llvm.exp", x_times_ln_10)
    })
);
unary_number_source!(
    Log,
    "log",
    1.0,
    |x| x.ln(),
    LlvmImplementation::IntrinsicUnary("llvm.log")
);
unary_number_source!(
    Log2,
    "log2",
    1.0,
    |x| x.log2(),
    LlvmImplementation::IntrinsicUnary("llvm.log2")
);
unary_number_source!(
    Log10,
    "log10",
    1.0,
    |x| x.log10(),
    LlvmImplementation::IntrinsicUnary("llvm.log10")
);
unary_number_source!(
    Sqrt,
    "sqrt",
    0.0,
    |x| x.sqrt(),
    LlvmImplementation::IntrinsicUnary("llvm.sqrt")
);
// unary_number_source!(Cbrt, "cbrt", 0.0, |x| x.cbrt());
unary_number_source!(
    Sin,
    "sin",
    0.0,
    |x| x.sin(),
    LlvmImplementation::IntrinsicUnary("llvm.sin")
);
unary_number_source!(
    Cos,
    "cos",
    0.0,
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
    0.0,
    |x| (x * std::f32::consts::TAU).sin(),
    LlvmImplementation::ExpressionUnary(|codegen, x| {
        let tau = codegen.float_type().const_float(std::f64::consts::TAU);
        let tau_x = codegen.builder().build_float_mul(tau, x, "tau_x");
        let sin_tau_x = codegen.build_unary_intrinsic_call("llvm.sin", tau_x);
        sin_tau_x
    })
);
unary_number_source!(
    CosineWave,
    "cosinewave",
    0.0,
    |x| (x * std::f32::consts::TAU).cos(),
    LlvmImplementation::ExpressionUnary(|codegen, x| {
        let tau = codegen.float_type().const_float(std::f64::consts::TAU);
        let tau_x = codegen.builder().build_float_mul(tau, x, "tau_x");
        let sin_tau_x = codegen.build_unary_intrinsic_call("llvm.cos", tau_x);
        sin_tau_x
    })
);
unary_number_source!(
    SquareWave,
    "squarewave",
    0.0,
    |x| {
        if (x - x.floor()) >= 0.5 {
            1.0
        } else {
            -1.0
        }
    },
    LlvmImplementation::ExpressionUnary(|codegen, x| {
        let plus_one = codegen.float_type().const_float(1.0);
        let minus_one = codegen.float_type().const_float(-1.0);
        let a_half = codegen.float_type().const_float(0.5);
        let x_floor = codegen.build_unary_intrinsic_call("llvm.floor", x);
        let x_fract = codegen.builder().build_float_sub(x, x_floor, "x_fract");
        let x_fract_ge_half = codegen.builder().build_float_compare(
            FloatPredicate::UGE,
            x_fract,
            a_half,
            "x_fract_ge_half",
        );
        codegen
            .builder()
            .build_select(x_fract_ge_half, plus_one, minus_one, "square_wave")
            .into_float_value()
    })
);
unary_number_source!(
    SawWave,
    "sawwave",
    0.0,
    |x| 2.0 * (x - x.floor()) - 1.0,
    LlvmImplementation::ExpressionUnary(|codegen, x| {
        let one = codegen.float_type().const_float(1.0);
        let two = codegen.float_type().const_float(2.0);
        let x_floor = codegen.build_unary_intrinsic_call("llvm.floor", x);
        let x_fract = codegen.builder().build_float_sub(x, x_floor, "x_fract");
        let two_x_fract = codegen.builder().build_float_mul(x_fract, two, "2x_fract");
        codegen
            .builder()
            .build_float_sub(two_x_fract, one, "saw_wave")
    })
);
unary_number_source!(
    TriangleWave,
    "trianglewave",
    0.0,
    |x| 4.0 * (x - (x + 0.5).floor()).abs() - 1.0,
    LlvmImplementation::ExpressionUnary(|codegen, x| {
        let one = codegen.float_type().const_float(1.0);
        let four = codegen.float_type().const_float(4.0);
        let a_half = codegen.float_type().const_float(0.5);

        let x_plus_half = codegen.builder().build_float_add(x, a_half, "x_plus_half");
        let floored = codegen.build_unary_intrinsic_call("llvm.floor", x_plus_half);
        let x_minus_floored = codegen
            .builder()
            .build_float_sub(x, floored, "x_minus_floored");
        let abs = codegen.build_unary_intrinsic_call("llvm.fabs", x_minus_floored);

        let four_abs = codegen.builder().build_float_mul(abs, four, "four_abs");
        codegen
            .builder()
            .build_float_sub(four_abs, one, "triangle_wave")
    })
);

binary_number_source!(
    Add,
    "add",
    (0.0, 0.0),
    |a, b| a + b,
    LlvmImplementation::ExpressionBinary(|codegen, a, b| {
        codegen.builder().build_float_add(a, b, "sum")
    })
);
binary_number_source!(
    Subtract,
    "subtract",
    (0.0, 0.0),
    |a, b| a - b,
    LlvmImplementation::ExpressionBinary(|codegen, a, b| {
        codegen.builder().build_float_sub(a, b, "difference")
    })
);
binary_number_source!(
    Multiply,
    "multiply",
    (1.0, 1.0),
    |a, b| a * b,
    LlvmImplementation::ExpressionBinary(|codegen, a, b| {
        codegen.builder().build_float_mul(a, b, "product")
    })
);
binary_number_source!(
    Divide,
    "divide",
    (1.0, 1.0),
    |a, b| a / b,
    LlvmImplementation::ExpressionBinary(|codegen, a, b| {
        codegen.builder().build_float_div(a, b, "quotient")
    })
);
// binary_number_source!(Hypot, "hypot", |a, b| a.hypot(b));
binary_number_source!(
    Copysign,
    "copysign",
    (0.0, 0.0),
    |a, b| a.copysign(b),
    LlvmImplementation::IntrinsicBinary("llvm.copysign")
);
binary_number_source!(
    Pow,
    "pow",
    (0.0, 1.0),
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

ternary_number_source!(
    Lerp,
    "lerp",
    (0.0, 1.0, 0.0),
    |a, b, c| { a + c * (b - a) },
    LlvmImplementation::ExpressionTernary(|codegen, a, b, c| {
        let diff = codegen.builder().build_float_sub(b, a, "diff");
        let scaled_diff = codegen.builder().build_float_mul(c, diff, "scaled_diff");
        codegen.builder().build_float_add(a, scaled_diff, "lerp")
    })
);
