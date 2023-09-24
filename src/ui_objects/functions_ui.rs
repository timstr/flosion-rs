use eframe::egui;

use serialization::{Deserializer, Serializable, Serializer};

use crate::{
    core::number::numbersource::NumberSourceHandle,
    objects::functions::*,
    ui_core::{
        graph_ui::ObjectUiState,
        lexicallayout::lexicallayout::NumberSourceLayout,
        numbergraphui::NumberGraphUi,
        numbergraphuicontext::NumberGraphUiContext,
        numbergraphuistate::{NumberGraphUiState, NumberObjectUiData},
        numbersourceui::NumberSourceUi,
        object_ui::{ObjectUi, UiInitialization},
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
        // NumberSourceUi::new_unnamed(constant.id()).show(ui, ctx, ui_state);
        NumberSourceUi::new_named(constant.id(), format!("{}", constant.value()))
            .show(ui, ctx, ui_state);
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
        NumberSourceUi::new_unnamed(variable.id()).show_with(ui, ctx, ui_state, |ui, _ui_state| {
            let mut v = variable.get_value();
            let v_old = v;
            ui.add(egui::Slider::new(
                &mut v,
                data.state.min_value..=data.state.max_value,
            ));
            if v != v_old {
                variable.set_value(v);
            }
        });
    }

    fn make_ui_state(
        &self,
        object: &NumberSourceHandle<Variable>,
        init: UiInitialization,
    ) -> (Self::StateType, NumberSourceLayout) {
        let state = match init {
            // TODO: add back initialization from some kind of arguments
            UiInitialization::Default => {
                let v = object.get_value();
                VariableUiState {
                    min_value: if v < 0.0 { 2.0 * v } else { 0.0 },
                    max_value: 2.0 * v.abs(),
                    name: "Variable".to_string(),
                }
            }
        };
        (state, NumberSourceLayout::default())
    }
}

macro_rules! unary_number_source_ui {
    ($name: ident, $object: ident, $display_name: literal, $aliases: expr, $layout: expr) => {
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
                NumberSourceUi::new_named(object.id(), $display_name.to_string())
                    .show(ui, ctx, ui_state);
            }

            fn aliases(&self) -> &'static [&'static str] {
                &$aliases
            }

            fn make_ui_state(
                &self,
                _object: &NumberSourceHandle<$object>,
                _init: UiInitialization,
            ) -> (Self::StateType, NumberSourceLayout) {
                ((), $layout)
            }
        }
    };
}

macro_rules! binary_number_source_ui {
    ($name: ident, $object: ident, $display_name: literal, $aliases: expr, $layout: expr) => {
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
                NumberSourceUi::new_named(object.id(), $display_name.to_string())
                    .show(ui, ctx, ui_state);
            }

            fn aliases(&self) -> &'static [&'static str] {
                &$aliases
            }

            fn make_ui_state(
                &self,
                _object: &NumberSourceHandle<$object>,
                _init: UiInitialization,
            ) -> (Self::StateType, NumberSourceLayout) {
                ((), $layout)
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
                NumberSourceUi::new_named(object.id(), $display_name.to_string())
                    .show(ui, ctx, ui_state);
            }

            fn aliases(&self) -> &'static [&'static str] {
                &$aliases
            }
        }
    };
}

unary_number_source_ui!(NegateUi, Negate, "Negate", [], NumberSourceLayout::Prefix);
unary_number_source_ui!(FloorUi, Floor, "Floor", [], NumberSourceLayout::Function);
unary_number_source_ui!(CeilUi, Ceil, "Ceil", [], NumberSourceLayout::Function);
unary_number_source_ui!(RoundUi, Round, "Round", [], NumberSourceLayout::Function);
unary_number_source_ui!(TruncUi, Trunc, "Trunc", [], NumberSourceLayout::Function);
unary_number_source_ui!(FractUi, Fract, "Fract", [], NumberSourceLayout::Function);
unary_number_source_ui!(AbsUi, Abs, "Abs", [], NumberSourceLayout::Function);
unary_number_source_ui!(SignumUi, Signum, "Signum", [], NumberSourceLayout::Function);
unary_number_source_ui!(ExpUi, Exp, "Exp", [], NumberSourceLayout::Function);
unary_number_source_ui!(Exp2Ui, Exp2, "Exp2", [], NumberSourceLayout::Function);
unary_number_source_ui!(Exp10Ui, Exp10, "Exp10", [], NumberSourceLayout::Function);
unary_number_source_ui!(LogUi, Log, "Log", [], NumberSourceLayout::Function);
unary_number_source_ui!(Log2Ui, Log2, "Log2", [], NumberSourceLayout::Function);
unary_number_source_ui!(Log10Ui, Log10, "Log10", [], NumberSourceLayout::Function);
unary_number_source_ui!(SqrtUi, Sqrt, "Sqrt", [], NumberSourceLayout::Function);
// unary_number_source_ui!(CbrtUi, Cbrt, "Cbrt", []);
unary_number_source_ui!(SinUi, Sin, "Sin", [], NumberSourceLayout::Function);
unary_number_source_ui!(CosUi, Cos, "Cos", [], NumberSourceLayout::Function);
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

unary_number_source_ui!(
    SineWaveUi,
    SineWave,
    "SineWave",
    [],
    NumberSourceLayout::Function
);
unary_number_source_ui!(
    CosineWaveUi,
    CosineWave,
    "CosineWave",
    [],
    NumberSourceLayout::Function
);
unary_number_source_ui!(
    SquareWaveUi,
    SquareWave,
    "SquareWave",
    [],
    NumberSourceLayout::Function
);
unary_number_source_ui!(
    SawWaveUi,
    SawWave,
    "SawWave",
    [],
    NumberSourceLayout::Function
);
unary_number_source_ui!(
    TriangleWaveUi,
    TriangleWave,
    "TriangleWave",
    [],
    NumberSourceLayout::Function
);

binary_number_source_ui!(AddUi, Add, "Add", ["+", "plus"], NumberSourceLayout::Infix);
binary_number_source_ui!(
    SubtractUi,
    Subtract,
    "Subtract",
    ["-", "minus"],
    NumberSourceLayout::Infix
);
binary_number_source_ui!(
    MultiplyUi,
    Multiply,
    "Multiply",
    ["*", "times"],
    NumberSourceLayout::Infix
);
binary_number_source_ui!(DivideUi, Divide, "Divide", ["/"], NumberSourceLayout::Infix);
// binary_number_source_ui!(HypotUi, Hypot, "Hypot", []);
binary_number_source_ui!(
    CopysignUi,
    Copysign,
    "Copysign",
    [],
    NumberSourceLayout::Function
);
binary_number_source_ui!(PowUi, Pow, "Pow", [], NumberSourceLayout::Function);
// binary_number_source_ui!(Atan2Ui, Atan2, "Atan2", []);

ternary_number_source_ui!(LerpUi, Lerp, "Lerp", []);
