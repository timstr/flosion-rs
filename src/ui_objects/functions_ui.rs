use eframe::egui;

use serialization::{Deserializer, Serializable, Serializer};

use crate::{
    core::number::{numbergraph::NumberGraph, numbersource::NumberSourceHandle},
    objects::functions::*,
    ui_core::{
        arguments::{ArgumentList, FloatRangeArgument, StringIdentifierArgument},
        graph_ui::ObjectUiState,
        lexicallayout::lexicallayout::NumberSourceLayout,
        numbergraphui::NumberGraphUi,
        numbergraphuicontext::NumberGraphUiContext,
        numbergraphuistate::{NumberGraphUiState, NumberObjectUiData},
        numbersourceui::{DisplayStyle, NumberSourceUi},
        object_ui::{ObjectUi, UiInitialization},
    },
};

#[derive(Default)]
pub struct ConstantUi {}

impl ConstantUi {
    pub const ARG_NAME: StringIdentifierArgument = StringIdentifierArgument("name");
}

impl ObjectUi for ConstantUi {
    type GraphUi = NumberGraphUi;
    type HandleType = NumberSourceHandle<Constant>;
    type StateType = ();

    fn ui(
        &self,
        constant: NumberSourceHandle<Constant>,
        ui_state: &mut NumberGraphUiState,
        ui: &mut egui::Ui,
        ctx: &mut NumberGraphUiContext,
        _data: NumberObjectUiData<()>,
        _number_graph: &mut NumberGraph,
    ) {
        // TODO: add ui state for custom name
        // NumberSourceUi::new_unnamed(constant.id()).show(ui, ctx, ui_state);
        NumberSourceUi::new_named(
            constant.id(),
            format!("{}", constant.value()),
            DisplayStyle::Framed,
        )
        .show(ui, ctx, ui_state);
    }

    fn summon_names(&self) -> &'static [&'static str] {
        &["constant"]
    }

    fn summon_arguments(&self) -> ArgumentList {
        ArgumentList::new_empty()
            .add(&Constant::ARG_VALUE)
            .add(&ConstantUi::ARG_NAME)
    }
}

#[derive(Default)]
pub struct SliderUi {}

impl SliderUi {
    pub const ARG_NAME: StringIdentifierArgument = StringIdentifierArgument("name");
    pub const ARG_RANGE: FloatRangeArgument = FloatRangeArgument("range");
}

pub struct SliderUiState {
    min_value: f32,
    max_value: f32,
    name: String,
    show_settings: bool,
}

impl Default for SliderUiState {
    fn default() -> Self {
        Self {
            min_value: 0.0,
            max_value: 1.0,
            name: "Variable".to_string(),
            show_settings: false,
        }
    }
}

impl Serializable for SliderUiState {
    fn serialize(&self, serializer: &mut Serializer) {
        serializer.f32(self.min_value);
        serializer.f32(self.max_value);
        serializer.string(&self.name);
    }

    fn deserialize(deserializer: &mut Deserializer) -> Result<Self, ()> {
        Ok(SliderUiState {
            min_value: deserializer.f32()?,
            max_value: deserializer.f32()?,
            name: deserializer.string()?,
            show_settings: false,
        })
    }
}

impl ObjectUiState for SliderUiState {}

impl ObjectUi for SliderUi {
    type GraphUi = NumberGraphUi;
    type HandleType = NumberSourceHandle<Variable>;
    type StateType = SliderUiState;
    fn ui(
        &self,
        variable: NumberSourceHandle<Variable>,
        ui_state: &mut NumberGraphUiState,
        ui: &mut eframe::egui::Ui,
        ctx: &mut NumberGraphUiContext,
        data: NumberObjectUiData<SliderUiState>,
        _number_graph: &mut NumberGraph,
    ) {
        NumberSourceUi::new_named(variable.id(), data.state.name.clone(), DisplayStyle::Framed)
            .show_with(ui, ctx, ui_state, |ui, _ui_state| {
                let mut v = variable.get_value();
                let v_old = v;
                ui.add(egui::Slider::new(
                    &mut v,
                    data.state.min_value..=data.state.max_value,
                ));
                if v != v_old {
                    variable.set_value(v);
                }
                if ui.add(egui::Button::new("edit")).clicked() {
                    data.state.show_settings = !data.state.show_settings;
                }

                if data.state.show_settings {
                    ui.label("min");
                    ui.add(egui::DragValue::new(&mut data.state.min_value));
                    ui.label("max");
                    ui.add(egui::DragValue::new(&mut data.state.max_value));
                }
            });
    }

    fn make_ui_state(
        &self,
        object: &NumberSourceHandle<Variable>,
        init: UiInitialization,
    ) -> (Self::StateType, NumberSourceLayout) {
        let state = match init {
            UiInitialization::Default => {
                let v = object.get_value();
                SliderUiState {
                    min_value: if v < 0.0 { 2.0 * v } else { 0.0 },
                    max_value: 2.0 * v.abs(),
                    name: "".to_string(),
                    show_settings: false,
                }
            }
            UiInitialization::Arguments(args) => {
                let value = args.get(&Variable::ARG_VALUE);
                let range = args.get(&SliderUi::ARG_RANGE);
                let (value, range) = match (value, range) {
                    (Some(v), Some(r)) => (v, r),
                    (None, Some(r)) => (0.5 * (r.start() + r.end()), r),
                    (Some(v), None) => (
                        v,
                        if v == 0.0 {
                            0.0..=1.0
                        } else if v < 0.0 {
                            (2.0 * v)..=(-2.0 * v)
                        } else {
                            0.0..=(2.0 * v)
                        },
                    ),
                    (None, None) => (1.0, 0.0..=2.0),
                };
                object.set_value(value as f32);
                SliderUiState {
                    min_value: *range.start() as f32,
                    max_value: *range.end() as f32,
                    name: args
                        .get(&SliderUi::ARG_NAME)
                        .unwrap_or_else(|| "".to_string()),
                    show_settings: false,
                }
            }
        };
        (state, NumberSourceLayout::default())
    }

    fn summon_names(&self) -> &'static [&'static str] {
        &["slider"]
    }

    fn summon_arguments(&self) -> ArgumentList {
        ArgumentList::new_empty()
            .add(&Variable::ARG_VALUE)
            .add(&SliderUi::ARG_NAME)
            .add(&SliderUi::ARG_RANGE)
    }
}

macro_rules! unary_number_source_ui {
    ($name: ident, $object: ident, $display_name: literal, $display_style: expr, $summon_names: expr, $layout: expr) => {
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
                ctx: &mut NumberGraphUiContext,
                _data: NumberObjectUiData<Self::StateType>,
                _number_graph: &mut NumberGraph,
            ) {
                NumberSourceUi::new_named(object.id(), $display_name.to_string(), $display_style)
                    .show(ui, ctx, ui_state);
            }

            fn summon_names(&self) -> &'static [&'static str] {
                &$summon_names
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
    ($name: ident, $object: ident, $display_name: literal, $display_style: expr, $summon_names: expr, $layout: expr) => {
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
                ctx: &mut NumberGraphUiContext,
                _data: NumberObjectUiData<Self::StateType>,
                _number_graph: &mut NumberGraph,
            ) {
                NumberSourceUi::new_named(object.id(), $display_name.to_string(), $display_style)
                    .show(ui, ctx, ui_state);
            }

            fn summon_names(&self) -> &'static [&'static str] {
                &$summon_names
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
    ($name: ident, $object: ident, $display_name: literal, $display_style: expr, $summon_names: expr) => {
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
                ctx: &mut NumberGraphUiContext,
                _data: NumberObjectUiData<Self::StateType>,
                _number_graph: &mut NumberGraph,
            ) {
                NumberSourceUi::new_named(object.id(), $display_name.to_string(), $display_style)
                    .show(ui, ctx, ui_state);
            }

            fn summon_names(&self) -> &'static [&'static str] {
                &$summon_names
            }
        }
    };
}

unary_number_source_ui!(
    NegateUi,
    Negate,
    "Negate",
    DisplayStyle::Framed,
    ["negate"],
    NumberSourceLayout::Prefix
);
unary_number_source_ui!(
    FloorUi,
    Floor,
    "Floor",
    DisplayStyle::Framed,
    ["floor"],
    NumberSourceLayout::Function
);
unary_number_source_ui!(
    CeilUi,
    Ceil,
    "Ceil",
    DisplayStyle::Framed,
    ["ceil"],
    NumberSourceLayout::Function
);
unary_number_source_ui!(
    RoundUi,
    Round,
    "Round",
    DisplayStyle::Framed,
    ["round"],
    NumberSourceLayout::Function
);
unary_number_source_ui!(
    TruncUi,
    Trunc,
    "Trunc",
    DisplayStyle::Framed,
    ["trunc"],
    NumberSourceLayout::Function
);
unary_number_source_ui!(
    FractUi,
    Fract,
    "Fract",
    DisplayStyle::Framed,
    ["fract"],
    NumberSourceLayout::Function
);
unary_number_source_ui!(
    AbsUi,
    Abs,
    "Abs",
    DisplayStyle::Framed,
    ["abs"],
    NumberSourceLayout::Function
);
unary_number_source_ui!(
    SignumUi,
    Signum,
    "Signum",
    DisplayStyle::Framed,
    ["signum"],
    NumberSourceLayout::Function
);
unary_number_source_ui!(
    ExpUi,
    Exp,
    "Exp",
    DisplayStyle::Framed,
    ["exp"],
    NumberSourceLayout::Function
);
unary_number_source_ui!(
    Exp2Ui,
    Exp2,
    "Exp2",
    DisplayStyle::Framed,
    ["exp2"],
    NumberSourceLayout::Function
);
unary_number_source_ui!(
    Exp10Ui,
    Exp10,
    "Exp10",
    DisplayStyle::Framed,
    ["exp10"],
    NumberSourceLayout::Function
);
unary_number_source_ui!(
    LogUi,
    Log,
    "Log",
    DisplayStyle::Framed,
    ["log"],
    NumberSourceLayout::Function
);
unary_number_source_ui!(
    Log2Ui,
    Log2,
    "Log2",
    DisplayStyle::Framed,
    ["log2"],
    NumberSourceLayout::Function
);
unary_number_source_ui!(
    Log10Ui,
    Log10,
    "Log10",
    DisplayStyle::Framed,
    ["log10"],
    NumberSourceLayout::Function
);
unary_number_source_ui!(
    SqrtUi,
    Sqrt,
    "Sqrt",
    DisplayStyle::Framed,
    ["sqrt"],
    NumberSourceLayout::Function
);
// unary_number_source_ui!(CbrtUi, Cbrt, "Cbrt", []);
unary_number_source_ui!(
    SinUi,
    Sin,
    "Sin",
    DisplayStyle::Framed,
    ["sin"],
    NumberSourceLayout::Function
);
unary_number_source_ui!(
    CosUi,
    Cos,
    "Cos",
    DisplayStyle::Framed,
    ["cos"],
    NumberSourceLayout::Function
);
// TODO
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
    DisplayStyle::Framed,
    ["sinewave"],
    NumberSourceLayout::Function
);
unary_number_source_ui!(
    CosineWaveUi,
    CosineWave,
    "CosineWave",
    DisplayStyle::Framed,
    ["cosinewave"],
    NumberSourceLayout::Function
);
unary_number_source_ui!(
    SquareWaveUi,
    SquareWave,
    "SquareWave",
    DisplayStyle::Framed,
    ["squarewave"],
    NumberSourceLayout::Function
);
unary_number_source_ui!(
    SawWaveUi,
    SawWave,
    "SawWave",
    DisplayStyle::Frameless,
    ["sawwave"],
    NumberSourceLayout::Function
);
unary_number_source_ui!(
    TriangleWaveUi,
    TriangleWave,
    "TriangleWave",
    DisplayStyle::Framed,
    ["trianglewave"],
    NumberSourceLayout::Function
);

binary_number_source_ui!(
    AddUi,
    Add,
    "+",
    DisplayStyle::Frameless,
    ["add", "+", "plus"],
    NumberSourceLayout::Infix
);
binary_number_source_ui!(
    SubtractUi,
    Subtract,
    "-",
    DisplayStyle::Frameless,
    ["subtract", "-", "minus"],
    NumberSourceLayout::Infix
);
binary_number_source_ui!(
    MultiplyUi,
    Multiply,
    "*",
    DisplayStyle::Frameless,
    ["multiply", "*", "times"],
    NumberSourceLayout::Infix
);
binary_number_source_ui!(
    DivideUi,
    Divide,
    "/",
    DisplayStyle::Frameless,
    ["divide", "/"],
    NumberSourceLayout::Infix
);
// binary_number_source_ui!(HypotUi, Hypot, "Hypot", []);
binary_number_source_ui!(
    CopysignUi,
    Copysign,
    "Copysign",
    DisplayStyle::Framed,
    ["copysign"],
    NumberSourceLayout::Function
);
binary_number_source_ui!(
    PowUi,
    Pow,
    "^",
    DisplayStyle::Frameless,
    ["pow", "^"],
    NumberSourceLayout::Infix
);
// binary_number_source_ui!(Atan2Ui, Atan2, "Atan2", []);

ternary_number_source_ui!(LerpUi, Lerp, "Lerp", DisplayStyle::Framed, ["lerp"]);
