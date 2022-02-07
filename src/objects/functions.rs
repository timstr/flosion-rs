use crate::core::{
    context::NumberContext,
    graphobject::{ObjectType, TypedGraphObject},
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
    fn eval(&self, dst: &mut [f32], _context: NumberContext) {
        numeric::fill(dst, self.value.load(Ordering::SeqCst));
    }
}

impl TypedGraphObject for Constant {
    const TYPE: ObjectType = ObjectType::new("constant");
}

impl PureNumberSource for Constant {
    fn new(_tools: &mut NumberSourceTools<'_>) -> Constant {
        Constant {
            value: AtomicF32::new(0.0),
        }
    }
}

pub struct Add {
    pub input_1: NumberInputHandle,
    pub input_2: NumberInputHandle,
}

impl NumberSource for Add {
    fn eval(&self, dst: &mut [f32], context: NumberContext) {
        self.input_1.eval(dst, context);
        let mut scratch_space = context.get_scratch_space(dst.len());
        self.input_2.eval(&mut scratch_space, context);
        numeric::add_inplace(dst, &mut scratch_space);
    }
}

impl TypedGraphObject for Add {
    const TYPE: ObjectType = ObjectType::new("add");
}

impl PureNumberSource for Add {
    fn new(tools: &mut NumberSourceTools<'_>) -> Add {
        Add {
            input_1: tools.add_number_input().0,
            input_2: tools.add_number_input().0,
        }
    }
}

pub struct Sine {
    pub input: NumberInputHandle,
}

impl NumberSource for Sine {
    fn eval(&self, dst: &mut [f32], context: NumberContext) {
        self.input.eval(dst, context);
        numeric::apply_unary_inplace(dst, |x| x.sin());
    }
}

impl TypedGraphObject for Sine {
    const TYPE: ObjectType = ObjectType::new("sine");
}

impl PureNumberSource for Sine {
    fn new(tools: &mut NumberSourceTools<'_>) -> Sine {
        Sine {
            input: tools.add_number_input().0,
        }
    }
}

pub struct UnitSine {
    pub input: NumberInputHandle,
}

impl NumberSource for UnitSine {
    fn eval(&self, dst: &mut [f32], context: NumberContext) {
        self.input.eval(dst, context);
        numeric::mul_scalar_inplace(dst, std::f32::consts::TAU);
        numeric::apply_unary_inplace(dst, |x| x.sin());
    }
}

impl TypedGraphObject for UnitSine {
    const TYPE: ObjectType = ObjectType::new("unitsine");
}

impl PureNumberSource for UnitSine {
    fn new(tools: &mut NumberSourceTools<'_>) -> UnitSine {
        UnitSine {
            input: tools.add_number_input().0,
        }
    }
}
