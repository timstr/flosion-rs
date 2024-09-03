use crate::{
    core::expression::{
        expressiongraph::ExpressionGraph, expressionnode::StatefulExpressionNodeHandle,
    },
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
    type HandleType = StatefulExpressionNodeHandle<LinearApproach>;
    type StateType = ();

    fn ui<'a, 'b>(
        &self,
        handle: StatefulExpressionNodeHandle<LinearApproach>,
        _graph_ui_state: &mut ExpressionGraphUiState,
        ui: &mut eframe::egui::Ui,
        ctx: &ExpressionGraphUiContext,
        _state: &mut (),
        _graph: &mut ExpressionGraph,
    ) {
        ExpressionNodeUi::new_named(
            handle.id(),
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

    fn make_ui_state(&self, _handle: &Self::HandleType, _args: ParsedArguments) -> Result<(), ()> {
        Ok(())
    }
}

#[derive(Default)]
pub struct ExponentialApproachUi {}

impl ExpressionObjectUi for ExponentialApproachUi {
    type HandleType = StatefulExpressionNodeHandle<ExponentialApproach>;
    type StateType = ();

    fn ui<'a, 'b>(
        &self,
        handle: StatefulExpressionNodeHandle<ExponentialApproach>,
        _ui_state: &mut ExpressionGraphUiState,
        ui: &mut eframe::egui::Ui,
        ctx: &ExpressionGraphUiContext,
        _data: &mut (),
        _graph: &mut ExpressionGraph,
    ) {
        ExpressionNodeUi::new_named(
            handle.id(),
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

    fn make_ui_state(&self, _handle: &Self::HandleType, _args: ParsedArguments) -> Result<(), ()> {
        Ok(())
    }
}

#[derive(Default)]
pub struct IntegratorUi {}

impl ExpressionObjectUi for IntegratorUi {
    type HandleType = StatefulExpressionNodeHandle<Integrator>;
    type StateType = ();

    fn ui<'a, 'b>(
        &self,
        handle: StatefulExpressionNodeHandle<Integrator>,
        _ui_state: &mut ExpressionGraphUiState,
        ui: &mut eframe::egui::Ui,
        ctx: &ExpressionGraphUiContext,
        _data: &mut (),
        _graph: &mut ExpressionGraph,
    ) {
        ExpressionNodeUi::new_named(handle.id(), "Integrator".to_string(), DisplayStyle::Framed)
            .show(ui, ctx);
    }

    fn summon_names(&self) -> &'static [&'static str] {
        &["integrator"]
    }

    fn make_properties(&self) -> ExpressionNodeLayout {
        ExpressionNodeLayout::Function
    }

    fn make_ui_state(&self, _handle: &Self::HandleType, _args: ParsedArguments) -> Result<(), ()> {
        Ok(())
    }
}

#[derive(Default)]
pub struct WrappingIntegratorUi {}

impl ExpressionObjectUi for WrappingIntegratorUi {
    type HandleType = StatefulExpressionNodeHandle<WrappingIntegrator>;
    type StateType = ();

    fn ui<'a, 'b>(
        &self,
        handle: StatefulExpressionNodeHandle<WrappingIntegrator>,
        _ui_state: &mut ExpressionGraphUiState,
        ui: &mut eframe::egui::Ui,
        ctx: &ExpressionGraphUiContext,
        _data: &mut (),
        _graph: &mut ExpressionGraph,
    ) {
        ExpressionNodeUi::new_named(
            handle.id(),
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

    fn make_ui_state(&self, _handle: &Self::HandleType, _args: ParsedArguments) -> Result<(), ()> {
        Ok(())
    }
}
