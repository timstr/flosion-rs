use eframe::egui;
use hashstash::{InplaceUnstasher, Stashable, Stasher, UnstashError, UnstashableInplace};

use crate::{
    core::expression::expressionnode::ExpressionNodeWithId,
    objects::purefunctions::*,
    ui_core::{
        arguments::{ArgumentList, FloatRangeArgument, ParsedArguments, StringIdentifierArgument},
        expressiongraphuicontext::ExpressionGraphUiContext,
        expressiongraphuistate::ExpressionGraphUiState,
        expressionobjectui::ExpressionObjectUi,
        expressionodeui::{DisplayStyle, ExpressionNodeUi},
        lexicallayout::lexicallayout::ExpressionNodeLayout,
        object_ui::NoObjectUiState,
    },
};

#[derive(Default)]
pub struct ConstantUi {}

impl ConstantUi {
    pub const ARG_NAME: StringIdentifierArgument = StringIdentifierArgument("name");
}

impl ExpressionObjectUi for ConstantUi {
    type ObjectType = ExpressionNodeWithId<Constant>;
    type StateType = NoObjectUiState;

    fn ui(
        &self,
        constant: &mut ExpressionNodeWithId<Constant>,
        _graph_ui_state: &mut ExpressionGraphUiState,
        ui: &mut egui::Ui,
        ctx: &ExpressionGraphUiContext,
        _state: &mut NoObjectUiState,
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

    fn make_properties(&self) -> ExpressionNodeLayout {
        ExpressionNodeLayout::Function
    }

    fn make_ui_state(
        &self,
        _object: &Self::ObjectType,
        _args: ParsedArguments,
    ) -> Result<NoObjectUiState, ()> {
        Ok(NoObjectUiState)
    }
}

#[derive(Default)]
pub struct SliderUi {}

impl SliderUi {
    pub const ARG_RANGE: FloatRangeArgument = FloatRangeArgument("range");
}

pub struct SliderUiState {
    min_value: f32,
    max_value: f32,
    show_settings: bool,
}

impl Stashable for SliderUiState {
    fn stash(&self, stasher: &mut Stasher) {
        stasher.f32(self.min_value);
        stasher.f32(self.max_value);
        stasher.bool(self.show_settings);
    }
}

impl UnstashableInplace for SliderUiState {
    fn unstash_inplace(&mut self, unstasher: &mut InplaceUnstasher) -> Result<(), UnstashError> {
        unstasher.f32_inplace(&mut self.min_value)?;
        unstasher.f32_inplace(&mut self.max_value)?;
        unstasher.bool_inplace(&mut self.show_settings)?;
        Ok(())
    }
}

impl ExpressionObjectUi for SliderUi {
    type ObjectType = ExpressionNodeWithId<Variable>;
    type StateType = SliderUiState;
    fn ui(
        &self,
        variable: &mut ExpressionNodeWithId<Variable>,
        _graph_ui_state: &mut ExpressionGraphUiState,
        ui: &mut eframe::egui::Ui,
        ctx: &ExpressionGraphUiContext,
        state: &mut SliderUiState,
    ) {
        ExpressionNodeUi::new_unnamed(variable.id(), DisplayStyle::Framed).show_with(
            ui,
            ctx,
            |ui| {
                let mut v = variable.get_value();
                let v_old = v;
                let response = ui.add(egui::Slider::new(&mut v, state.min_value..=state.max_value));
                if v != v_old {
                    variable.set_value(v);
                }
                if response.drag_stopped() {
                    ctx.request_snapshot();
                }
                if ui.add(egui::Button::new("...")).clicked() {
                    state.show_settings = !state.show_settings;
                }

                if state.show_settings {
                    ui.label("min");
                    ui.add(egui::DragValue::new(&mut state.min_value));
                    ui.label("max");
                    ui.add(egui::DragValue::new(&mut state.max_value));
                }
            },
        );
    }

    fn summon_names(&self) -> &'static [&'static str] {
        &["slider"]
    }

    fn summon_arguments(&self) -> ArgumentList {
        ArgumentList::new_empty()
            .add(&Variable::ARG_VALUE)
            .add(&SliderUi::ARG_RANGE)
    }

    fn make_properties(&self) -> ExpressionNodeLayout {
        ExpressionNodeLayout::Function
    }

    fn make_ui_state(
        &self,
        variable: &ExpressionNodeWithId<Variable>,
        args: ParsedArguments,
    ) -> Result<SliderUiState, ()> {
        // NOTE: the value argument is already used
        // in Variable's own initialization.
        // Only if it is missing and the range is
        // given do we assign the value here.
        // Importantly, this does not change the
        // value when no arguments are given, which
        // is the case during unstashing and undo/redo.
        let range = args.get(&SliderUi::ARG_RANGE);

        if args.get(&Variable::ARG_VALUE).is_none() {
            if let Some(range) = &range {
                variable.set_value(0.5 * (range.start() + range.end()) as f32);
            }
        }

        let range = range.unwrap_or_else(|| {
            let v = variable.get_value() as f64;
            if v == 0.0 {
                0.0..=1.0
            } else if v < 0.0 {
                (2.0 * v)..=(-2.0 * v)
            } else {
                0.0..=(2.0 * v)
            }
        });

        Ok(SliderUiState {
            min_value: *range.start() as f32,
            max_value: *range.end() as f32,
            show_settings: false,
        })
    }
}

macro_rules! unary_expression_node_ui {
    ($name: ident, $object: ident, $display_name: literal, $display_style: expr, $summon_names: expr, $layout: expr) => {
        #[derive(Default)]
        pub struct $name {}

        impl ExpressionObjectUi for $name {
            type ObjectType = ExpressionNodeWithId<$object>;
            type StateType = NoObjectUiState;
            fn ui(
                &self,
                object: &mut ExpressionNodeWithId<$object>,
                _graph_ui_state: &mut ExpressionGraphUiState,
                ui: &mut egui::Ui,
                ctx: &ExpressionGraphUiContext,
                _state: &mut NoObjectUiState,
            ) {
                ExpressionNodeUi::new_named(object.id(), $display_name.to_string(), $display_style)
                    .show(ui, ctx);
            }

            fn summon_names(&self) -> &'static [&'static str] {
                &$summon_names
            }

            fn make_properties(&self) -> ExpressionNodeLayout {
                $layout
            }

            fn make_ui_state(
                &self,
                _object: &ExpressionNodeWithId<$object>,
                _args: ParsedArguments,
            ) -> Result<NoObjectUiState, ()> {
                Ok(NoObjectUiState)
            }
        }
    };
}

macro_rules! binary_expression_node_ui {
    ($name: ident, $object: ident, $display_name: literal, $display_style: expr, $summon_names: expr, $layout: expr) => {
        #[derive(Default)]
        pub struct $name {}

        impl ExpressionObjectUi for $name {
            type ObjectType = ExpressionNodeWithId<$object>;
            type StateType = NoObjectUiState;
            fn ui(
                &self,
                object: &mut ExpressionNodeWithId<$object>,
                _graph_ui_state: &mut ExpressionGraphUiState,
                ui: &mut egui::Ui,
                ctx: &ExpressionGraphUiContext,
                _state: &mut NoObjectUiState,
            ) {
                ExpressionNodeUi::new_named(object.id(), $display_name.to_string(), $display_style)
                    .show(ui, ctx);
            }

            fn summon_names(&self) -> &'static [&'static str] {
                &$summon_names
            }

            fn make_properties(&self) -> ExpressionNodeLayout {
                $layout
            }

            fn make_ui_state(
                &self,
                _object: &ExpressionNodeWithId<$object>,
                _args: ParsedArguments,
            ) -> Result<NoObjectUiState, ()> {
                Ok(NoObjectUiState)
            }
        }
    };
}

macro_rules! ternary_expression_node_ui {
    ($name: ident, $object: ident, $display_name: literal, $display_style: expr, $summon_names: expr) => {
        #[derive(Default)]
        pub struct $name {}

        impl ExpressionObjectUi for $name {
            type ObjectType = ExpressionNodeWithId<$object>;
            type StateType = NoObjectUiState;
            fn ui(
                &self,
                object: &mut ExpressionNodeWithId<$object>,
                _graph_ui_state: &mut ExpressionGraphUiState,
                ui: &mut egui::Ui,
                ctx: &ExpressionGraphUiContext,
                _state: &mut NoObjectUiState,
            ) {
                ExpressionNodeUi::new_named(object.id(), $display_name.to_string(), $display_style)
                    .show(ui, ctx);
            }

            fn summon_names(&self) -> &'static [&'static str] {
                &$summon_names
            }

            fn make_properties(&self) -> ExpressionNodeLayout {
                ExpressionNodeLayout::Function
            }

            fn make_ui_state(
                &self,
                _object: &ExpressionNodeWithId<$object>,
                _args: ParsedArguments,
            ) -> Result<NoObjectUiState, ()> {
                Ok(NoObjectUiState)
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
