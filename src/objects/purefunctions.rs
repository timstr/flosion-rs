use crate::{
    core::{
        expression::{
            expressionnode::PureExpressionNode, expressionnodeinput::ExpressionNodeInputHandle,
            expressionnodetools::ExpressionNodeTools,
        },
        graph::graphobject::{ObjectInitialization, ObjectType, WithObjectType},
        jit::codegen::CodeGen,
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

impl PureExpressionNode for Constant {
    fn new(_tools: ExpressionNodeTools<'_>, init: ObjectInitialization) -> Result<Self, ()> {
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

// Note: Variable isn't strictly speaking "pure" in the mathematical sense,
// but it is intended to not vary rapidly (e.g. at audio rates) and
// doesn't need any extra per-node state to be stored.
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

impl PureExpressionNode for Variable {
    fn new(_tools: ExpressionNodeTools<'_>, init: ObjectInitialization) -> Result<Self, ()> {
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

macro_rules! unary_expression_node {
    ($name: ident, $namestr: literal, $default_input: expr, $f: expr, $llvm_impl: expr) => {
        pub struct $name {
            pub input: ExpressionNodeInputHandle,
        }

        impl PureExpressionNode for $name {
            fn new(
                mut tools: ExpressionNodeTools<'_>,
                _init: ObjectInitialization,
            ) -> Result<$name, ()> {
                let default_value: f32 = $default_input;
                Ok($name {
                    input: tools.add_input(default_value),
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

macro_rules! binary_expression_node {
    ($name: ident, $namestr: literal, $default_inputs: expr, $f: expr, $llvm_impl: expr) => {
        pub struct $name {
            pub input_1: ExpressionNodeInputHandle,
            pub input_2: ExpressionNodeInputHandle,
        }

        impl PureExpressionNode for $name {
            fn new(
                mut tools: ExpressionNodeTools<'_>,
                _init: ObjectInitialization,
            ) -> Result<$name, ()> {
                let default_values: (f32, f32) = $default_inputs;
                Ok($name {
                    input_1: tools.add_input(default_values.0),
                    input_2: tools.add_input(default_values.1),
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

macro_rules! ternary_expression_node {
    ($name: ident, $namestr: literal, $default_inputs: expr, $f: expr, $llvm_impl: expr) => {
        pub struct $name {
            pub input_1: ExpressionNodeInputHandle,
            pub input_2: ExpressionNodeInputHandle,
            pub input_3: ExpressionNodeInputHandle,
        }

        impl PureExpressionNode for $name {
            fn new(
                mut tools: ExpressionNodeTools<'_>,
                _init: ObjectInitialization,
            ) -> Result<$name, ()> {
                let default_values: (f32, f32, f32) = $default_inputs;
                Ok($name {
                    input_1: tools.add_input(default_values.0),
                    input_2: tools.add_input(default_values.1),
                    input_3: tools.add_input(default_values.2),
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

unary_expression_node!(
    Negate,
    "negate",
    0.0,
    |x| -x,
    LlvmImplementation::ExpressionUnary(|codegen, x| {
        codegen.builder().build_float_neg(x, "x").unwrap()
    })
);
unary_expression_node!(
    Floor,
    "floor",
    0.0,
    |x| x.floor(),
    LlvmImplementation::IntrinsicUnary("llvm.floor")
);
unary_expression_node!(
    Ceil,
    "ceil",
    0.0,
    |x| x.ceil(),
    LlvmImplementation::IntrinsicUnary("llvm.ceil")
);
unary_expression_node!(
    Round,
    "round",
    0.0,
    |x| x.round(),
    LlvmImplementation::IntrinsicUnary("llvm.round")
);
unary_expression_node!(
    Trunc,
    "trunc",
    0.0,
    |x| x.trunc(),
    LlvmImplementation::IntrinsicUnary("llvm.trunc")
);
unary_expression_node!(
    Fract,
    "fract",
    0.0,
    |x| x.fract(),
    LlvmImplementation::ExpressionUnary(|codegen, x| {
        let x_trunc = codegen.build_unary_intrinsic_call("llvm.trunc", x);
        codegen
            .builder()
            .build_float_sub(x, x_trunc, "fract")
            .unwrap()
    })
);
unary_expression_node!(
    Abs,
    "abs",
    0.0,
    |x| x.abs(),
    LlvmImplementation::IntrinsicUnary("llvm.fabs")
);
unary_expression_node!(
    Signum,
    "signum",
    0.0,
    |x| x.signum(),
    LlvmImplementation::ExpressionUnary(|codegen, x| {
        let one = codegen.float_type().const_float(1.0);
        codegen.build_binary_intrinsic_call("llvm.copysign", one, x)
    })
);
unary_expression_node!(
    Exp,
    "exp",
    0.0,
    |x| x.exp(),
    LlvmImplementation::IntrinsicUnary("llvm.exp")
);
unary_expression_node!(
    Exp2,
    "exp2",
    0.0,
    |x| x.exp2(),
    LlvmImplementation::IntrinsicUnary("llvm.exp2")
);
unary_expression_node!(
    Exp10,
    "exp10",
    0.0,
    |x| (x * std::f32::consts::LN_10).exp(),
    LlvmImplementation::ExpressionUnary(|codegen, x| {
        let ln_10 = codegen
            .float_type()
            .const_float(std::f32::consts::LN_10 as f64);
        let x_times_ln_10 = codegen
            .builder()
            .build_float_mul(x, ln_10, "x_times_ln_10")
            .unwrap();
        codegen.build_unary_intrinsic_call("llvm.exp", x_times_ln_10)
    })
);
unary_expression_node!(
    Log,
    "log",
    1.0,
    |x| x.ln(),
    LlvmImplementation::IntrinsicUnary("llvm.log")
);
unary_expression_node!(
    Log2,
    "log2",
    1.0,
    |x| x.log2(),
    LlvmImplementation::IntrinsicUnary("llvm.log2")
);
unary_expression_node!(
    Log10,
    "log10",
    1.0,
    |x| x.log10(),
    LlvmImplementation::IntrinsicUnary("llvm.log10")
);
unary_expression_node!(
    Sqrt,
    "sqrt",
    0.0,
    |x| x.sqrt(),
    LlvmImplementation::IntrinsicUnary("llvm.sqrt")
);
// TODO:
// - cbrt
unary_expression_node!(
    Sin,
    "sin",
    0.0,
    |x| x.sin(),
    LlvmImplementation::IntrinsicUnary("llvm.sin")
);
unary_expression_node!(
    Cos,
    "cos",
    0.0,
    |x| x.cos(),
    LlvmImplementation::IntrinsicUnary("llvm.cos")
);
// TODO:
//  - tan
//  - asin
//  - acos
//  - atan
//  - sinh
//  - cosh
//  - tanh
//  - asinh
//  - acosh
//  - atanh

unary_expression_node!(
    SineWave,
    "sinewave",
    0.0,
    |x| (x * std::f32::consts::TAU).sin(),
    LlvmImplementation::ExpressionUnary(|codegen, x| {
        let tau = codegen.float_type().const_float(std::f64::consts::TAU);
        let tau_x = codegen.builder().build_float_mul(tau, x, "tau_x").unwrap();
        let sin_tau_x = codegen.build_unary_intrinsic_call("llvm.sin", tau_x);
        sin_tau_x
    })
);
unary_expression_node!(
    CosineWave,
    "cosinewave",
    0.0,
    |x| (x * std::f32::consts::TAU).cos(),
    LlvmImplementation::ExpressionUnary(|codegen, x| {
        let tau = codegen.float_type().const_float(std::f64::consts::TAU);
        let tau_x = codegen.builder().build_float_mul(tau, x, "tau_x").unwrap();
        let sin_tau_x = codegen.build_unary_intrinsic_call("llvm.cos", tau_x);
        sin_tau_x
    })
);
unary_expression_node!(
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
        let x_fract = codegen
            .builder()
            .build_float_sub(x, x_floor, "x_fract")
            .unwrap();
        let x_fract_ge_half = codegen
            .builder()
            .build_float_compare(FloatPredicate::UGE, x_fract, a_half, "x_fract_ge_half")
            .unwrap();
        codegen
            .builder()
            .build_select(x_fract_ge_half, plus_one, minus_one, "square_wave")
            .unwrap()
            .into_float_value()
    })
);
unary_expression_node!(
    SawWave,
    "sawwave",
    0.0,
    |x| 2.0 * (x - x.floor()) - 1.0,
    LlvmImplementation::ExpressionUnary(|codegen, x| {
        let one = codegen.float_type().const_float(1.0);
        let two = codegen.float_type().const_float(2.0);
        let x_floor = codegen.build_unary_intrinsic_call("llvm.floor", x);
        let x_fract = codegen
            .builder()
            .build_float_sub(x, x_floor, "x_fract")
            .unwrap();
        let two_x_fract = codegen
            .builder()
            .build_float_mul(x_fract, two, "2x_fract")
            .unwrap();
        codegen
            .builder()
            .build_float_sub(two_x_fract, one, "saw_wave")
            .unwrap()
    })
);
unary_expression_node!(
    TriangleWave,
    "trianglewave",
    0.0,
    |x| 4.0 * (x - (x + 0.5).floor()).abs() - 1.0,
    LlvmImplementation::ExpressionUnary(|codegen, x| {
        let one = codegen.float_type().const_float(1.0);
        let four = codegen.float_type().const_float(4.0);
        let a_half = codegen.float_type().const_float(0.5);

        let x_plus_half = codegen
            .builder()
            .build_float_add(x, a_half, "x_plus_half")
            .unwrap();
        let floored = codegen.build_unary_intrinsic_call("llvm.floor", x_plus_half);
        let x_minus_floored = codegen
            .builder()
            .build_float_sub(x, floored, "x_minus_floored")
            .unwrap();
        let abs = codegen.build_unary_intrinsic_call("llvm.fabs", x_minus_floored);

        let four_abs = codegen
            .builder()
            .build_float_mul(abs, four, "four_abs")
            .unwrap();
        codegen
            .builder()
            .build_float_sub(four_abs, one, "triangle_wave")
            .unwrap()
    })
);

binary_expression_node!(
    Add,
    "add",
    (0.0, 0.0),
    |a, b| a + b,
    LlvmImplementation::ExpressionBinary(|codegen, a, b| {
        codegen.builder().build_float_add(a, b, "sum").unwrap()
    })
);
binary_expression_node!(
    Subtract,
    "subtract",
    (0.0, 0.0),
    |a, b| a - b,
    LlvmImplementation::ExpressionBinary(|codegen, a, b| {
        codegen
            .builder()
            .build_float_sub(a, b, "difference")
            .unwrap()
    })
);
binary_expression_node!(
    Multiply,
    "multiply",
    (1.0, 1.0),
    |a, b| a * b,
    LlvmImplementation::ExpressionBinary(|codegen, a, b| {
        codegen.builder().build_float_mul(a, b, "product").unwrap()
    })
);
binary_expression_node!(
    Divide,
    "divide",
    (1.0, 1.0),
    |a, b| a / b,
    LlvmImplementation::ExpressionBinary(|codegen, a, b| {
        codegen.builder().build_float_div(a, b, "quotient").unwrap()
    })
);
// TODO:
//  - hypot
binary_expression_node!(
    Copysign,
    "copysign",
    (0.0, 0.0),
    |a, b| a.copysign(b),
    LlvmImplementation::IntrinsicBinary("llvm.copysign")
);
binary_expression_node!(
    Pow,
    "pow",
    (0.0, 1.0),
    |a, b| a.powf(b),
    LlvmImplementation::ExpressionBinary(|codegen, a, b| {
        // TODO: use the intrinsic that already exists!
        // https://llvm.org/docs/LangRef.html#llvm-pow-intrinsic
        // x = a^b
        // x = e^(ln(a^b))
        // x = e^(b * ln(a))
        let ln_a = codegen.build_unary_intrinsic_call("llvm.log", a);
        let b_ln_a = codegen
            .builder()
            .build_float_mul(b, ln_a, "b_ln_a")
            .unwrap();
        codegen.build_unary_intrinsic_call("llvm.exp", b_ln_a)
    })
);
// TODO:
//  - atan2

ternary_expression_node!(
    Lerp,
    "lerp",
    (0.0, 1.0, 0.0),
    |a, b, c| { a + c * (b - a) },
    LlvmImplementation::ExpressionTernary(|codegen, a, b, c| {
        let diff = codegen.builder().build_float_sub(b, a, "diff").unwrap();
        let scaled_diff = codegen
            .builder()
            .build_float_mul(c, diff, "scaled_diff")
            .unwrap();
        codegen
            .builder()
            .build_float_add(a, scaled_diff, "lerp")
            .unwrap()
    })
);
