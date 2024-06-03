use crate::{
    core::expression::{
        expressiongraph::ExpressionGraph, expressionnode::StatefulExpressionNodeHandle,
    },
    objects::statefulfunctions::{
        ExponentialApproach, Integrator, LinearApproach, WrappingIntegrator,
    },
    ui_core::{
        expressiongraphui::ExpressionGraphUi,
        expressiongraphuicontext::ExpressionGraphUiContext,
        expressiongraphuistate::ExpressionNodeObjectUiData,
        expressionodeui::{DisplayStyle, ExpressionNodeUi},
        lexicallayout::lexicallayout::ExpressionNodeLayout,
        object_ui::{ObjectUi, UiInitialization},
    },
};

#[derive(Default)]
pub struct LinearApproachUi {}

impl ObjectUi for LinearApproachUi {
    type GraphUi = ExpressionGraphUi;
    type HandleType = StatefulExpressionNodeHandle<LinearApproach>;
    type StateType = ();

    fn ui<'a, 'b>(
        &self,
        handle: StatefulExpressionNodeHandle<LinearApproach>,
        _ui_state: &mut (),
        ui: &mut eframe::egui::Ui,
        ctx: &mut ExpressionGraphUiContext,
        _data: ExpressionNodeObjectUiData<()>,
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

    fn make_ui_state(
        &self,
        _handle: &Self::HandleType,
        _init: UiInitialization,
    ) -> (Self::StateType, ExpressionNodeLayout) {
        ((), ExpressionNodeLayout::Function)
    }
}

#[derive(Default)]
pub struct ExponentialApproachUi {}

impl ObjectUi for ExponentialApproachUi {
    type GraphUi = ExpressionGraphUi;
    type HandleType = StatefulExpressionNodeHandle<ExponentialApproach>;
    type StateType = ();

    fn ui<'a, 'b>(
        &self,
        handle: StatefulExpressionNodeHandle<ExponentialApproach>,
        _ui_state: &mut (),
        ui: &mut eframe::egui::Ui,
        ctx: &mut ExpressionGraphUiContext,
        _data: ExpressionNodeObjectUiData<()>,
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

    fn make_ui_state(
        &self,
        _handle: &Self::HandleType,
        _init: UiInitialization,
    ) -> (Self::StateType, ExpressionNodeLayout) {
        ((), ExpressionNodeLayout::Function)
    }
}

#[derive(Default)]
pub struct IntegratorUi {}

impl ObjectUi for IntegratorUi {
    type GraphUi = ExpressionGraphUi;
    type HandleType = StatefulExpressionNodeHandle<Integrator>;
    type StateType = ();

    fn ui<'a, 'b>(
        &self,
        handle: StatefulExpressionNodeHandle<Integrator>,
        _ui_state: &mut (),
        ui: &mut eframe::egui::Ui,
        ctx: &mut ExpressionGraphUiContext,
        _data: ExpressionNodeObjectUiData<()>,
        _graph: &mut ExpressionGraph,
    ) {
        ExpressionNodeUi::new_named(handle.id(), "Integrator".to_string(), DisplayStyle::Framed)
            .show(ui, ctx);
    }

    fn summon_names(&self) -> &'static [&'static str] {
        &["integrator"]
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
pub struct WrappingIntegratorUi {}

impl ObjectUi for WrappingIntegratorUi {
    type GraphUi = ExpressionGraphUi;
    type HandleType = StatefulExpressionNodeHandle<WrappingIntegrator>;
    type StateType = ();

    fn ui<'a, 'b>(
        &self,
        handle: StatefulExpressionNodeHandle<WrappingIntegrator>,
        _ui_state: &mut (),
        ui: &mut eframe::egui::Ui,
        ctx: &mut ExpressionGraphUiContext,
        _data: ExpressionNodeObjectUiData<()>,
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

    fn make_ui_state(
        &self,
        _handle: &Self::HandleType,
        _init: UiInitialization,
    ) -> (Self::StateType, ExpressionNodeLayout) {
        ((), ExpressionNodeLayout::Function)
    }
}
