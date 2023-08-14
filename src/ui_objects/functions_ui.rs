use eframe::egui;

use crate::{
    core::{
        arguments::{ArgumentList, ArgumentValue},
        number::numbersource::NumberSourceHandle,
        serialization::{Deserializer, Serializable, Serializer},
    },
    objects::functions::*,
    ui_core::{
        numbergraphui::NumberGraphUi,
        numbergraphuicontext::NumberGraphUiContext,
        numbergraphuistate::{NumberGraphUiState, NumberObjectUiData},
        numbersourceui::NumberSourceUi,
        object_ui::{ObjectUi, ObjectUiState, UiInitialization},
    },
};

#[derive(Default)]
pub struct ConstantUi {}

impl ObjectUi for ConstantUi {
    type GraphUi = NumberGraphUi;
    type HandleType = NumberSourceHandle<Constant>;
    type StateType = ();

    fn ui(
        &self,
        constant: NumberSourceHandle<Constant>,
        ui_state: &mut NumberGraphUiState,
        ui: &mut egui::Ui,
        ctx: &NumberGraphUiContext,
        _data: NumberObjectUiData<()>,
    ) {
        // TODO: add ui state for custom name
        NumberSourceUi::new(constant.id(), "Constant").show(ui, ctx, ui_state);
    }

    fn arguments(&self) -> ArgumentList {
        let mut args = ArgumentList::new();
        args.add("value", ArgumentValue::Float(0.0));
        args
    }
}

#[derive(Default)]
pub struct VariableUi {}

pub struct VariableUiState {
    min_value: f32,
    max_value: f32,
    name: String,
}

impl Default for VariableUiState {
    fn default() -> Self {
        Self {
            min_value: 0.0,
            max_value: 1.0,
            name: "Constant".to_string(),
        }
    }
}

impl Serializable for VariableUiState {
    fn serialize(&self, serializer: &mut Serializer) {
        serializer.f32(self.min_value);
        serializer.f32(self.max_value);
        serializer.string(&self.name);
    }

    fn deserialize(deserializer: &mut Deserializer) -> Result<Self, ()> {
        Ok(VariableUiState {
            min_value: deserializer.f32()?,
            max_value: deserializer.f32()?,
            name: deserializer.string()?,
        })
    }
}

impl ObjectUiState for VariableUiState {}

impl ObjectUi for VariableUi {
    type GraphUi = NumberGraphUi;
    type HandleType = NumberSourceHandle<Variable>;
    type StateType = VariableUiState;
    fn ui(
        &self,
        variable: NumberSourceHandle<Variable>,
        ui_state: &mut NumberGraphUiState,
        ui: &mut eframe::egui::Ui,
        ctx: &NumberGraphUiContext,
        data: NumberObjectUiData<VariableUiState>,
    ) {
        NumberSourceUi::new(variable.id(), "Variable").show_with(
            ui,
            ctx,
            ui_state,
            |ui, _ui_state| {
                let mut v = variable.get_value();
                let v_old = v;
                ui.add(egui::Slider::new(
                    &mut v,
                    data.state.min_value..=data.state.max_value,
                ));
                if v != v_old {
                    variable.set_value(v);
                }
            },
        );
    }

    fn arguments(&self) -> ArgumentList {
        let mut args = ArgumentList::new();
        args.add("value", ArgumentValue::Float(0.0));
        args.add("min", ArgumentValue::Float(0.0));
        args.add("max", ArgumentValue::Float(1.0));
        args.add("name", ArgumentValue::String("Constant".to_string()));
        args
    }

    fn make_ui_state(
        &self,
        object: &NumberSourceHandle<Variable>,
        init: UiInitialization,
    ) -> Self::StateType {
        match init {
            UiInitialization::Args(args) => VariableUiState {
                min_value: args.get("min").as_float().unwrap(),
                max_value: args.get("max").as_float().unwrap(),
                name: args.get("name").as_string().unwrap().to_string(),
            },
            UiInitialization::Default => {
                let v = object.get_value();
                VariableUiState {
                    min_value: if v < 0.0 { 2.0 * v } else { 0.0 },
                    max_value: 2.0 * v.abs(),
                    name: "Variable".to_string(),
                }
            }
        }
    }
}

macro_rules! unary_number_source_ui {
    ($name: ident, $object: ident, $display_name: literal, $aliases: expr) => {
        #[derive(Default)]
        pub struct $name {}

        impl ObjectUi for $name {
            type GraphUi = NumberGraphUi;
            type HandleType = NumberSourceHandle<$object>;
            type StateType = ();
            fn ui(
                &self,
                object: NumberSourceHandle<$object>,
                ui_state: &mut NumberGraphUiState,
                ui: &mut egui::Ui,
                ctx: &NumberGraphUiContext,
                _data: NumberObjectUiData<Self::StateType>,
            ) {
                NumberSourceUi::new(object.id(), $display_name).show(ui, ctx, ui_state);
            }

            fn aliases(&self) -> &'static [&'static str] {
                &$aliases
            }
        }
    };
}

macro_rules! binary_number_source_ui {
    ($name: ident, $object: ident, $display_name: literal, $aliases: expr) => {
        #[derive(Default)]
        pub struct $name {}

        impl ObjectUi for $name {
            type GraphUi = NumberGraphUi;
            type HandleType = NumberSourceHandle<$object>;
            type StateType = ();
            fn ui(
                &self,
                object: NumberSourceHandle<$object>,
                ui_state: &mut NumberGraphUiState,
                ui: &mut egui::Ui,
                ctx: &NumberGraphUiContext,
                _data: NumberObjectUiData<Self::StateType>,
            ) {
                NumberSourceUi::new(object.id(), $display_name).show(ui, ctx, ui_state);
            }

            fn aliases(&self) -> &'static [&'static str] {
                &$aliases
            }
        }
    };
}

macro_rules! ternary_number_source_ui {
    ($name: ident, $object: ident, $display_name: literal, $aliases: expr) => {
        #[derive(Default)]
        pub struct $name {}

        impl ObjectUi for $name {
            type GraphUi = NumberGraphUi;
            type HandleType = NumberSourceHandle<$object>;
            type StateType = ();
            fn ui(
                &self,
                object: NumberSourceHandle<$object>,
                ui_state: &mut NumberGraphUiState,
                ui: &mut egui::Ui,
                ctx: &NumberGraphUiContext,
                _data: NumberObjectUiData<Self::StateType>,
            ) {
                NumberSourceUi::new(object.id(), $display_name).show(ui, ctx, ui_state);
            }

            fn aliases(&self) -> &'static [&'static str] {
                &$aliases
            }
        }
    };
}

unary_number_source_ui!(NegateUi, Negate, "Negate", []);
unary_number_source_ui!(FloorUi, Floor, "Floor", []);
unary_number_source_ui!(CeilUi, Ceil, "Ceil", []);
unary_number_source_ui!(RoundUi, Round, "Round", []);
unary_number_source_ui!(TruncUi, Trunc, "Trunc", []);
unary_number_source_ui!(FractUi, Fract, "Fract", []);
unary_number_source_ui!(AbsUi, Abs, "Abs", []);
unary_number_source_ui!(SignumUi, Signum, "Signum", []);
unary_number_source_ui!(ExpUi, Exp, "Exp", []);
unary_number_source_ui!(Exp2Ui, Exp2, "Exp2", []);
unary_number_source_ui!(Exp10Ui, Exp10, "Exp10", []);
unary_number_source_ui!(LogUi, Log, "Log", []);
unary_number_source_ui!(Log2Ui, Log2, "Log2", []);
unary_number_source_ui!(Log10Ui, Log10, "Log10", []);
unary_number_source_ui!(SqrtUi, Sqrt, "Sqrt", []);
// unary_number_source_ui!(CbrtUi, Cbrt, "Cbrt", []);
unary_number_source_ui!(SinUi, Sin, "Sin", []);
unary_number_source_ui!(CosUi, Cos, "Cos", []);
// unary_number_source_ui!(TanUi, Tan, "Tan", []);
// unary_number_source_ui!(AsinUi, Asin, "Asin", []);
// unary_number_source_ui!(AcosUi, Acos, "Acos", []);
// unary_number_source_ui!(AtanUi, Atan, "Atan", []);
// unary_number_source_ui!(SinhUi, Sinh, "Sinh", []);
// unary_number_source_ui!(CoshUi, Cosh, "Cosh", []);
// unary_number_source_ui!(TanhUi, Tanh, "Tanh", []);
// unary_number_source_ui!(AsinhUi, Asinh, "Asinh", []);
// unary_number_source_ui!(AcoshUi, Acosh, "Acosh", []);
// unary_number_source_ui!(AtanhUi, Atanh, "Atanh", []);

unary_number_source_ui!(SineWaveUi, SineWave, "SineWave", []);
unary_number_source_ui!(CosineWaveUi, CosineWave, "CosineWave", []);
unary_number_source_ui!(SquareWaveUi, SquareWave, "SquareWave", []);
unary_number_source_ui!(SawWaveUi, SawWave, "SawWave", []);
unary_number_source_ui!(TriangleWaveUi, TriangleWave, "TriangleWave", []);

binary_number_source_ui!(AddUi, Add, "Add", ["+", "plus"]);
binary_number_source_ui!(SubtractUi, Subtract, "Subtract", ["-", "minus"]);
binary_number_source_ui!(MultiplyUi, Multiply, "Multiply", ["*", "times"]);
binary_number_source_ui!(DivideUi, Divide, "Divide", ["/"]);
// binary_number_source_ui!(HypotUi, Hypot, "Hypot", []);
binary_number_source_ui!(CopysignUi, Copysign, "Copysign", []);
binary_number_source_ui!(PowUi, Pow, "Pow", []);
// binary_number_source_ui!(Atan2Ui, Atan2, "Atan2", []);

ternary_number_source_ui!(LerpUi, Lerp, "Lerp", []);
