use crate::{
    core::expression::expressionnode::ExpressionNodeWithId,
    objects::statefulfunctions::{
        ExponentialApproach, Integrator, LinearApproach, WrappingIntegrator,
    },
    ui_core::{
        arguments::ParsedArguments,
        expressiongraphuicontext::ExpressionGraphUiContext,
        expressiongraphuistate::ExpressionGraphUiState,
        expressionobjectui::ExpressionObjectUi,
        expressionodeui::{DisplayStyle, ExpressionNodeUi},
        lexicallayout::lexicallayout::ExpressionNodeLayout,
    },
};

#[derive(Default)]
pub struct LinearApproachUi {}

impl ExpressionObjectUi for LinearApproachUi {
    type ObjectType = ExpressionNodeWithId<LinearApproach>;
    type StateType = ();

    fn ui<'a, 'b>(
        &self,
        object: &mut ExpressionNodeWithId<LinearApproach>,
        _graph_ui_state: &mut ExpressionGraphUiState,
        ui: &mut eframe::egui::Ui,
        ctx: &ExpressionGraphUiContext,
        _state: &mut (),
    ) {
        ExpressionNodeUi::new_named(
            object.id(),
            "LinearApproach".to_string(),
            DisplayStyle::Framed,
        )
        .show(ui, ctx);
    }

    fn summon_names(&self) -> &'static [&'static str] {
        &["linearapproach"]
    }

    fn make_properties(&self) -> ExpressionNodeLayout {
        ExpressionNodeLayout::Function
    }

    fn make_ui_state(&self, _object: &Self::ObjectType, _args: ParsedArguments) -> Result<(), ()> {
        Ok(())
    }
}

#[derive(Default)]
pub struct ExponentialApproachUi {}

impl ExpressionObjectUi for ExponentialApproachUi {
    type ObjectType = ExpressionNodeWithId<ExponentialApproach>;
    type StateType = ();

    fn ui<'a, 'b>(
        &self,
        object: &mut ExpressionNodeWithId<ExponentialApproach>,
        _ui_state: &mut ExpressionGraphUiState,
        ui: &mut eframe::egui::Ui,
        ctx: &ExpressionGraphUiContext,
        _data: &mut (),
    ) {
        ExpressionNodeUi::new_named(
            object.id(),
            "ExponentialApproach".to_string(),
            DisplayStyle::Framed,
        )
        .show(ui, ctx);
    }

    fn summon_names(&self) -> &'static [&'static str] {
        &["exponentialapproach"]
    }

    fn make_properties(&self) -> ExpressionNodeLayout {
        ExpressionNodeLayout::Function
    }

    fn make_ui_state(&self, _object: &Self::ObjectType, _args: ParsedArguments) -> Result<(), ()> {
        Ok(())
    }
}

#[derive(Default)]
pub struct IntegratorUi {}

impl ExpressionObjectUi for IntegratorUi {
    type ObjectType = ExpressionNodeWithId<Integrator>;
    type StateType = ();

    fn ui<'a, 'b>(
        &self,
        object: &mut ExpressionNodeWithId<Integrator>,
        _ui_state: &mut ExpressionGraphUiState,
        ui: &mut eframe::egui::Ui,
        ctx: &ExpressionGraphUiContext,
        _data: &mut (),
    ) {
        ExpressionNodeUi::new_named(object.id(), "Integrator".to_string(), DisplayStyle::Framed)
            .show(ui, ctx);
    }

    fn summon_names(&self) -> &'static [&'static str] {
        &["integrator"]
    }

    fn make_properties(&self) -> ExpressionNodeLayout {
        ExpressionNodeLayout::Function
    }

    fn make_ui_state(&self, _object: &Self::ObjectType, _args: ParsedArguments) -> Result<(), ()> {
        Ok(())
    }
}

#[derive(Default)]
pub struct WrappingIntegratorUi {}

impl ExpressionObjectUi for WrappingIntegratorUi {
    type ObjectType = ExpressionNodeWithId<WrappingIntegrator>;
    type StateType = ();

    fn ui<'a, 'b>(
        &self,
        object: &mut ExpressionNodeWithId<WrappingIntegrator>,
        _ui_state: &mut ExpressionGraphUiState,
        ui: &mut eframe::egui::Ui,
        ctx: &ExpressionGraphUiContext,
        _data: &mut (),
    ) {
        ExpressionNodeUi::new_named(
            object.id(),
            "WrappingIntegrator".to_string(),
            DisplayStyle::Framed,
        )
        .show(ui, ctx);
    }

    fn summon_names(&self) -> &'static [&'static str] {
        &["wrappingintegrator"]
    }

    fn make_properties(&self) -> ExpressionNodeLayout {
        ExpressionNodeLayout::Function
    }

    fn make_ui_state(&self, _object: &Self::ObjectType, _args: ParsedArguments) -> Result<(), ()> {
        Ok(())
    }
}
