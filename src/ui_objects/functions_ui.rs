use eframe::egui;

use crate::{
    core::{
        arguments::{ArgumentList, ArgumentValue},
        numbersource::PureNumberSourceHandle,
        serialization::{Deserializer, Serializable, Serializer},
    },
    objects::functions::*,
    ui_core::{
        graph_ui_state::GraphUiState,
        object_ui::{ObjectUi, ObjectUiData, ObjectWindow, UiInitialization},
    },
};

#[derive(Default)]
pub struct ConstantUi {}

impl ObjectUi for ConstantUi {
    type HandleType = PureNumberSourceHandle<Constant>;

    type StateType = ();

    fn ui(
        &self,
        handle: Self::HandleType,
        graph_state: &mut GraphUiState,
        ui: &mut egui::Ui,
        data: ObjectUiData<Self::StateType>,
    ) {
        // TODO: add ui state for custom name
        ObjectWindow::new_number_source(handle.id(), "Constant", data.color)
            // .add_right_peg(&handle, "Output")
            .show(ui.ctx(), graph_state);
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

impl ObjectUi for VariableUi {
    type HandleType = PureNumberSourceHandle<Variable>;
    type StateType = VariableUiState;
    fn ui(
        &self,
        constant: PureNumberSourceHandle<Variable>,
        ui_state: &mut GraphUiState,
        ui: &mut eframe::egui::Ui,
        data: ObjectUiData<VariableUiState>,
    ) {
        // TODO: use data.state.name instead of always "Variable"
        // Will need to accept something other than &'static str in ObjectWindow
        ObjectWindow::new_number_source(constant.id(), "Variable", data.color)
            // .add_right_peg(&constant, "Output")
            .show_with(ui.ctx(), ui_state, |ui, _ui_state| {
                let mut v = constant.get_value();
                let v_old = v;
                ui.add(egui::Slider::new(
                    &mut v,
                    data.state.min_value..=data.state.max_value,
                ));
                if v != v_old {
                    constant.set_value(v);
                }
            });
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
        object: &PureNumberSourceHandle<Variable>,
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
            type HandleType = PureNumberSourceHandle<$object>;
            type StateType = ();
            fn ui(
                &self,
                object: PureNumberSourceHandle<$object>,
                ui_state: &mut GraphUiState,
                ui: &mut eframe::egui::Ui,
                data: ObjectUiData<Self::StateType>,
            ) {
                ObjectWindow::new_number_source(object.id(), $display_name, data.color)
                    // .add_left_peg(&object.input, "Input")
                    // .add_right_peg(&object, "Output")
                    .show(ui.ctx(), ui_state);
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
            type HandleType = PureNumberSourceHandle<$object>;
            type StateType = ();
            fn ui(
                &self,
                object: PureNumberSourceHandle<$object>,
                ui_state: &mut GraphUiState,
                ui: &mut eframe::egui::Ui,
                data: ObjectUiData<Self::StateType>,
            ) {
                ObjectWindow::new_number_source(object.id(), $display_name, data.color)
                    // .add_left_peg(&object.input_1, "Input 1")
                    // .add_left_peg(&object.input_2, "Input 2")
                    // .add_right_peg(&object, "Output")
                    .show(ui.ctx(), ui_state);
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
            type HandleType = PureNumberSourceHandle<$object>;
            type StateType = ();
            fn ui(
                &self,
                object: PureNumberSourceHandle<$object>,
                ui_state: &mut GraphUiState,
                ui: &mut eframe::egui::Ui,
                data: ObjectUiData<Self::StateType>,
            ) {
                ObjectWindow::new_number_source(object.id(), $display_name, data.color)
                    // .add_left_peg(&object.input_1, "Input 1")
                    // .add_left_peg(&object.input_2, "Input 2")
                    // .add_left_peg(&object.input_3, "Input 3")
                    // .add_right_peg(&object, "Output")
                    .show(ui.ctx(), ui_state);
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
