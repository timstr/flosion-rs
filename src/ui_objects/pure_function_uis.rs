use eframe::egui;

use serialization::{Deserializer, Serializable, Serializer};

use crate::{
    core::expression::{
        expressiongraph::ExpressionGraph, expressionnode::PureExpressionNodeHandle,
    },
    objects::purefunctions::*,
    ui_core::{
        arguments::{ArgumentList, FloatRangeArgument, StringIdentifierArgument},
        expressiongraphui::ExpressionGraphUi,
        expressiongraphuicontext::ExpressionGraphUiContext,
        expressiongraphuistate::ExpressionNodeObjectUiData,
        expressionodeui::{DisplayStyle, ExpressionNodeUi},
        graph_ui::ObjectUiState,
        lexicallayout::lexicallayout::ExpressionNodeLayout,
        object_ui::{ObjectUi, UiInitialization},
    },
};

#[derive(Default)]
pub struct ConstantUi {}

impl ConstantUi {
    pub const ARG_NAME: StringIdentifierArgument = StringIdentifierArgument("name");
}

impl ObjectUi for ConstantUi {
    type GraphUi = ExpressionGraphUi;
    type HandleType = PureExpressionNodeHandle<Constant>;
    type StateType = ();

    fn ui(
        &self,
        constant: PureExpressionNodeHandle<Constant>,
        ui_state: &mut (),
        ui: &mut egui::Ui,
        ctx: &mut ExpressionGraphUiContext,
        _data: ExpressionNodeObjectUiData<()>,
        _graph: &mut ExpressionGraph,
    ) {
        ExpressionNodeUi::new_named(
            constant.id(),
            format!("{}", constant.value()),
            DisplayStyle::Framed,
        )
        .show(ui, ctx);
    }

    fn summon_names(&self) -> &'static [&'static str] {
        &["constant"]
    }

    fn summon_arguments(&self) -> ArgumentList {
        ArgumentList::new_empty()
            .add(&Constant::ARG_VALUE)
            .add(&ConstantUi::ARG_NAME)
    }

    fn make_ui_state(
        &self,
        _handle: &Self::HandleType,
        _init: UiInitialization,
    ) -> (Self::StateType, ExpressionNodeLayout) {
        ((), ExpressionNodeLayout::Function)
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
    type GraphUi = ExpressionGraphUi;
    type HandleType = PureExpressionNodeHandle<Variable>;
    type StateType = SliderUiState;
    fn ui(
        &self,
        variable: PureExpressionNodeHandle<Variable>,
        ui_state: &mut (),
        ui: &mut eframe::egui::Ui,
        ctx: &mut ExpressionGraphUiContext,
        data: ExpressionNodeObjectUiData<SliderUiState>,
        _graph: &mut ExpressionGraph,
    ) {
        ExpressionNodeUi::new_named(variable.id(), data.state.name.clone(), DisplayStyle::Framed)
            .show_with(ui, ctx, |ui| {
                let mut v = variable.get_value();
                let v_old = v;
                ui.add(egui::Slider::new(
                    &mut v,
                    data.state.min_value..=data.state.max_value,
                ));
                if v != v_old {
                    variable.set_value(v);
                }
                if ui.add(egui::Button::new("...")).clicked() {
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
        object: &PureExpressionNodeHandle<Variable>,
        init: UiInitialization,
    ) -> (Self::StateType, ExpressionNodeLayout) {
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
        (state, ExpressionNodeLayout::default())
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

macro_rules! unary_expression_node_ui {
    ($name: ident, $object: ident, $display_name: literal, $display_style: expr, $summon_names: expr, $layout: expr) => {
        #[derive(Default)]
        pub struct $name {}

        impl ObjectUi for $name {
            type GraphUi = ExpressionGraphUi;
            type HandleType = PureExpressionNodeHandle<$object>;
            type StateType = ();
            fn ui(
                &self,
                object: PureExpressionNodeHandle<$object>,
                ui_state: &mut (),
                ui: &mut egui::Ui,
                ctx: &mut ExpressionGraphUiContext,
                _data: ExpressionNodeObjectUiData<Self::StateType>,
                _expr_graph: &mut ExpressionGraph,
            ) {
                ExpressionNodeUi::new_named(object.id(), $display_name.to_string(), $display_style)
                    .show(ui, ctx);
            }

            fn summon_names(&self) -> &'static [&'static str] {
                &$summon_names
            }

            fn make_ui_state(
                &self,
                _object: &PureExpressionNodeHandle<$object>,
                _init: UiInitialization,
            ) -> (Self::StateType, ExpressionNodeLayout) {
                ((), $layout)
            }
        }
    };
}

macro_rules! binary_expression_node_ui {
    ($name: ident, $object: ident, $display_name: literal, $display_style: expr, $summon_names: expr, $layout: expr) => {
        #[derive(Default)]
        pub struct $name {}

        impl ObjectUi for $name {
            type GraphUi = ExpressionGraphUi;
            type HandleType = PureExpressionNodeHandle<$object>;
            type StateType = ();
            fn ui(
                &self,
                object: PureExpressionNodeHandle<$object>,
                ui_state: &mut (),
                ui: &mut egui::Ui,
                ctx: &mut ExpressionGraphUiContext,
                _data: ExpressionNodeObjectUiData<Self::StateType>,
                _expr_graph: &mut ExpressionGraph,
            ) {
                ExpressionNodeUi::new_named(object.id(), $display_name.to_string(), $display_style)
                    .show(ui, ctx);
            }

            fn summon_names(&self) -> &'static [&'static str] {
                &$summon_names
            }

            fn make_ui_state(
                &self,
                _object: &PureExpressionNodeHandle<$object>,
                _init: UiInitialization,
            ) -> (Self::StateType, ExpressionNodeLayout) {
                ((), $layout)
            }
        }
    };
}

macro_rules! ternary_expression_node_ui {
    ($name: ident, $object: ident, $display_name: literal, $display_style: expr, $summon_names: expr) => {
        #[derive(Default)]
        pub struct $name {}

        impl ObjectUi for $name {
            type GraphUi = ExpressionGraphUi;
            type HandleType = PureExpressionNodeHandle<$object>;
            type StateType = ();
            fn ui(
                &self,
                object: PureExpressionNodeHandle<$object>,
                ui_state: &mut (),
                ui: &mut egui::Ui,
                ctx: &mut ExpressionGraphUiContext,
                _data: ExpressionNodeObjectUiData<Self::StateType>,
                _expr_graph: &mut ExpressionGraph,
            ) {
                ExpressionNodeUi::new_named(object.id(), $display_name.to_string(), $display_style)
                    .show(ui, ctx);
            }

            fn summon_names(&self) -> &'static [&'static str] {
                &$summon_names
            }

            fn make_ui_state(
                &self,
                _handle: &Self::HandleType,
                _init: UiInitialization,
            ) -> (Self::StateType, ExpressionNodeLayout) {
                ((), ExpressionNodeLayout::Function)
            }
        }
    };
}

unary_expression_node_ui!(
    NegateUi,
    Negate,
    "Negate",
    DisplayStyle::Framed,
    ["negate"],
    ExpressionNodeLayout::Prefix
);
unary_expression_node_ui!(
    FloorUi,
    Floor,
    "Floor",
    DisplayStyle::Framed,
    ["floor"],
    ExpressionNodeLayout::Function
);
unary_expression_node_ui!(
    CeilUi,
    Ceil,
    "Ceil",
    DisplayStyle::Framed,
    ["ceil"],
    ExpressionNodeLayout::Function
);
unary_expression_node_ui!(
    RoundUi,
    Round,
    "Round",
    DisplayStyle::Framed,
    ["round"],
    ExpressionNodeLayout::Function
);
unary_expression_node_ui!(
    TruncUi,
    Trunc,
    "Trunc",
    DisplayStyle::Framed,
    ["trunc"],
    ExpressionNodeLayout::Function
);
unary_expression_node_ui!(
    FractUi,
    Fract,
    "Fract",
    DisplayStyle::Framed,
    ["fract"],
    ExpressionNodeLayout::Function
);
unary_expression_node_ui!(
    AbsUi,
    Abs,
    "Abs",
    DisplayStyle::Framed,
    ["abs"],
    ExpressionNodeLayout::Function
);
unary_expression_node_ui!(
    SignumUi,
    Signum,
    "Signum",
    DisplayStyle::Framed,
    ["signum"],
    ExpressionNodeLayout::Function
);
unary_expression_node_ui!(
    ExpUi,
    Exp,
    "Exp",
    DisplayStyle::Framed,
    ["exp"],
    ExpressionNodeLayout::Function
);
unary_expression_node_ui!(
    Exp2Ui,
    Exp2,
    "Exp2",
    DisplayStyle::Framed,
    ["exp2"],
    ExpressionNodeLayout::Function
);
unary_expression_node_ui!(
    Exp10Ui,
    Exp10,
    "Exp10",
    DisplayStyle::Framed,
    ["exp10"],
    ExpressionNodeLayout::Function
);
unary_expression_node_ui!(
    LogUi,
    Log,
    "Log",
    DisplayStyle::Framed,
    ["log"],
    ExpressionNodeLayout::Function
);
unary_expression_node_ui!(
    Log2Ui,
    Log2,
    "Log2",
    DisplayStyle::Framed,
    ["log2"],
    ExpressionNodeLayout::Function
);
unary_expression_node_ui!(
    Log10Ui,
    Log10,
    "Log10",
    DisplayStyle::Framed,
    ["log10"],
    ExpressionNodeLayout::Function
);
unary_expression_node_ui!(
    SqrtUi,
    Sqrt,
    "Sqrt",
    DisplayStyle::Framed,
    ["sqrt"],
    ExpressionNodeLayout::Function
);
unary_expression_node_ui!(
    SinUi,
    Sin,
    "Sin",
    DisplayStyle::Framed,
    ["sin"],
    ExpressionNodeLayout::Function
);
unary_expression_node_ui!(
    CosUi,
    Cos,
    "Cos",
    DisplayStyle::Framed,
    ["cos"],
    ExpressionNodeLayout::Function
);

unary_expression_node_ui!(
    SineWaveUi,
    SineWave,
    "SineWave",
    DisplayStyle::Framed,
    ["sinewave"],
    ExpressionNodeLayout::Function
);
unary_expression_node_ui!(
    CosineWaveUi,
    CosineWave,
    "CosineWave",
    DisplayStyle::Framed,
    ["cosinewave"],
    ExpressionNodeLayout::Function
);
unary_expression_node_ui!(
    SquareWaveUi,
    SquareWave,
    "SquareWave",
    DisplayStyle::Framed,
    ["squarewave"],
    ExpressionNodeLayout::Function
);
unary_expression_node_ui!(
    SawWaveUi,
    SawWave,
    "SawWave",
    DisplayStyle::Frameless,
    ["sawwave"],
    ExpressionNodeLayout::Function
);
unary_expression_node_ui!(
    TriangleWaveUi,
    TriangleWave,
    "TriangleWave",
    DisplayStyle::Framed,
    ["trianglewave"],
    ExpressionNodeLayout::Function
);

binary_expression_node_ui!(
    AddUi,
    Add,
    "+",
    DisplayStyle::Frameless,
    ["add", "+", "plus"],
    ExpressionNodeLayout::Infix
);
binary_expression_node_ui!(
    SubtractUi,
    Subtract,
    "-",
    DisplayStyle::Frameless,
    ["subtract", "-", "minus"],
    ExpressionNodeLayout::Infix
);
binary_expression_node_ui!(
    MultiplyUi,
    Multiply,
    "*",
    DisplayStyle::Frameless,
    ["multiply", "*", "times"],
    ExpressionNodeLayout::Infix
);
binary_expression_node_ui!(
    DivideUi,
    Divide,
    "/",
    DisplayStyle::Frameless,
    ["divide", "/"],
    ExpressionNodeLayout::Infix
);
binary_expression_node_ui!(
    CopysignUi,
    Copysign,
    "Copysign",
    DisplayStyle::Framed,
    ["copysign"],
    ExpressionNodeLayout::Function
);
binary_expression_node_ui!(
    PowUi,
    Pow,
    "^",
    DisplayStyle::Frameless,
    ["pow", "^"],
    ExpressionNodeLayout::Infix
);

ternary_expression_node_ui!(LerpUi, Lerp, "Lerp", DisplayStyle::Framed, ["lerp"]);
