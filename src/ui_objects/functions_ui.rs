use eframe::egui;

use crate::{
    core::{
        arguments::{ArgumentList, ArgumentValue},
        graphobject::ObjectId,
        numbersource::PureNumberSourceHandle,
        serialization::{Deserializer, Serializable, Serializer},
    },
    objects::functions::*,
    ui_core::{
        graph_ui_state::GraphUIState,
        object_ui::{
            NoUIState, NumberInputWidget, NumberOutputWidget, ObjectUi, ObjectWindow,
            UiInitialization,
        },
    },
};

// TODO: distinguish constant from slider

#[derive(Default)]
pub struct ConstantUi {}

pub struct ConstantUiState {
    min_value: f32,
    max_value: f32,
    name: String,
}

impl Default for ConstantUiState {
    fn default() -> Self {
        Self {
            min_value: 0.0,
            max_value: 1.0,
            name: "Constant".to_string(),
        }
    }
}

impl Serializable for ConstantUiState {
    fn serialize(&self, serializer: &mut Serializer) {
        serializer.f32(self.min_value);
        serializer.f32(self.max_value);
        serializer.string(&self.name);
    }

    fn deserialize(deserializer: &mut Deserializer) -> Result<Self, ()> {
        Ok(ConstantUiState {
            min_value: deserializer.f32()?,
            max_value: deserializer.f32()?,
            name: deserializer.string()?,
        })
    }
}

impl ObjectUi for ConstantUi {
    type HandleType = PureNumberSourceHandle<Constant>;
    type StateType = ConstantUiState;
    fn ui(
        &self,
        id: ObjectId,
        object: PureNumberSourceHandle<Constant>,
        graph_tools: &mut GraphUIState,
        ui: &mut eframe::egui::Ui,
        state: &ConstantUiState,
    ) {
        let id = id.as_number_source_id().unwrap();
        ObjectWindow::new_number_source(id).show(ui.ctx(), graph_tools, |ui, graph_tools| {
            let mut v = object.get_value();
            let v_old = v;
            ui.label(&state.name);
            ui.add(egui::Slider::new(&mut v, state.min_value..=state.max_value));
            if v != v_old {
                object.set_value(v);
            }
            ui.add(NumberOutputWidget::new(id, "Output", graph_tools));
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
        object: &PureNumberSourceHandle<Constant>,
        init: UiInitialization,
    ) -> Self::StateType {
        match init {
            UiInitialization::Args(args) => ConstantUiState {
                min_value: args.get("min").as_float().unwrap(),
                max_value: args.get("max").as_float().unwrap(),
                name: args.get("name").as_string().unwrap().to_string(),
            },
            UiInitialization::Default => {
                let v = object.get_value();
                ConstantUiState {
                    min_value: if v < 0.0 { 2.0 * v } else { 0.0 },
                    max_value: 2.0 * v.abs(),
                    name: "Constant".to_string(),
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
            type StateType = NoUIState;
            fn ui(
                &self,
                id: ObjectId,
                object: PureNumberSourceHandle<$object>,
                graph_tools: &mut GraphUIState,
                ui: &mut eframe::egui::Ui,
                _state: &Self::StateType,
            ) {
                let id = id.as_number_source_id().unwrap();
                ObjectWindow::new_number_source(id).show(
                    ui.ctx(),
                    graph_tools,
                    |ui, graph_tools| {
                        ui.label($display_name);
                        ui.add(NumberInputWidget::new(
                            object.input.id(),
                            "Input",
                            graph_tools,
                        ));
                        ui.add(NumberOutputWidget::new(id, "Output", graph_tools));
                    },
                );
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
            type StateType = NoUIState;
            fn ui(
                &self,
                id: ObjectId,
                object: PureNumberSourceHandle<$object>,
                graph_tools: &mut GraphUIState,
                ui: &mut eframe::egui::Ui,
                _state: &Self::StateType,
            ) {
                let id = id.as_number_source_id().unwrap();
                ObjectWindow::new_number_source(id).show(
                    ui.ctx(),
                    graph_tools,
                    |ui, graph_tools| {
                        ui.label($display_name);
                        ui.add(NumberInputWidget::new(
                            object.input_1.id(),
                            "Input 1",
                            graph_tools,
                        ));
                        ui.add(NumberInputWidget::new(
                            object.input_2.id(),
                            "Input 2",
                            graph_tools,
                        ));
                        ui.add(NumberOutputWidget::new(id, "Output", graph_tools));
                    },
                );
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
// unary_number_source_ui!(SignumUi, Signum, "Signum", []);
unary_number_source_ui!(ExpUi, Exp, "Exp", []);
unary_number_source_ui!(Exp2Ui, Exp2, "Exp2", []);
// unary_number_source_ui!(Exp10Ui, Exp10, "Exp10", []);
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
// binary_number_source_ui!(CopysignUi, Copysign, "Copysign", []);
binary_number_source_ui!(PowUi, Pow, "Pow", []);
// binary_number_source_ui!(Atan2Ui, Atan2, "Atan2", []);
