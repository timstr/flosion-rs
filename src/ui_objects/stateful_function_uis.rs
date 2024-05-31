use crate::{
    core::expression::{
        expressiongraph::ExpressionGraph, expressionnode::StatefulExpressionNodeHandle,
    },
    objects::statefulfunctions::{
        ExponentialApproach, Integrator, LinearApproach, WrappingIntegrator,
    },
    ui_core::{
        lexicallayout::lexicallayout::NumberSourceLayout,
        numbergraphui::NumberGraphUi,
        numbergraphuicontext::NumberGraphUiContext,
        numbergraphuistate::{NumberGraphUiState, NumberObjectUiData},
        numbersourceui::{DisplayStyle, NumberSourceUi},
        object_ui::{ObjectUi, UiInitialization},
    },
};

#[derive(Default)]
pub struct LinearApproachUi {}

impl ObjectUi for LinearApproachUi {
    type GraphUi = NumberGraphUi;
    type HandleType = StatefulExpressionNodeHandle<LinearApproach>;
    type StateType = ();

    fn ui<'a, 'b>(
        &self,
        handle: StatefulExpressionNodeHandle<LinearApproach>,
        ui_state: &mut NumberGraphUiState,
        ui: &mut eframe::egui::Ui,
        ctx: &mut NumberGraphUiContext,
        _data: NumberObjectUiData<()>,
        _number_graph: &mut ExpressionGraph,
    ) {
        NumberSourceUi::new_named(
            handle.id(),
            "LinearApproach".to_string(),
            DisplayStyle::Framed,
        )
        .show(ui, ctx, ui_state);
    }

    fn summon_names(&self) -> &'static [&'static str] {
        &["linearapproach"]
    }

    fn make_ui_state(
        &self,
        _handle: &Self::HandleType,
        _init: UiInitialization,
    ) -> (Self::StateType, NumberSourceLayout) {
        ((), NumberSourceLayout::Function)
    }
}

#[derive(Default)]
pub struct ExponentialApproachUi {}

impl ObjectUi for ExponentialApproachUi {
    type GraphUi = NumberGraphUi;
    type HandleType = StatefulExpressionNodeHandle<ExponentialApproach>;
    type StateType = ();

    fn ui<'a, 'b>(
        &self,
        handle: StatefulExpressionNodeHandle<ExponentialApproach>,
        ui_state: &mut NumberGraphUiState,
        ui: &mut eframe::egui::Ui,
        ctx: &mut NumberGraphUiContext,
        _data: NumberObjectUiData<()>,
        _number_graph: &mut ExpressionGraph,
    ) {
        NumberSourceUi::new_named(
            handle.id(),
            "ExponentialApproach".to_string(),
            DisplayStyle::Framed,
        )
        .show(ui, ctx, ui_state);
    }

    fn summon_names(&self) -> &'static [&'static str] {
        &["exponentialapproach"]
    }

    fn make_ui_state(
        &self,
        _handle: &Self::HandleType,
        _init: UiInitialization,
    ) -> (Self::StateType, NumberSourceLayout) {
        ((), NumberSourceLayout::Function)
    }
}

#[derive(Default)]
pub struct IntegratorUi {}

impl ObjectUi for IntegratorUi {
    type GraphUi = NumberGraphUi;
    type HandleType = StatefulExpressionNodeHandle<Integrator>;
    type StateType = ();

    fn ui<'a, 'b>(
        &self,
        handle: StatefulExpressionNodeHandle<Integrator>,
        ui_state: &mut NumberGraphUiState,
        ui: &mut eframe::egui::Ui,
        ctx: &mut NumberGraphUiContext,
        _data: NumberObjectUiData<()>,
        _number_graph: &mut ExpressionGraph,
    ) {
        NumberSourceUi::new_named(handle.id(), "Integrator".to_string(), DisplayStyle::Framed)
            .show(ui, ctx, ui_state);
    }

    fn summon_names(&self) -> &'static [&'static str] {
        &["integrator"]
    }

    fn make_ui_state(
        &self,
        _handle: &Self::HandleType,
        _init: UiInitialization,
    ) -> (Self::StateType, NumberSourceLayout) {
        ((), NumberSourceLayout::Function)
    }
}

#[derive(Default)]
pub struct WrappingIntegratorUi {}

impl ObjectUi for WrappingIntegratorUi {
    type GraphUi = NumberGraphUi;
    type HandleType = StatefulExpressionNodeHandle<WrappingIntegrator>;
    type StateType = ();

    fn ui<'a, 'b>(
        &self,
        handle: StatefulExpressionNodeHandle<WrappingIntegrator>,
        ui_state: &mut NumberGraphUiState,
        ui: &mut eframe::egui::Ui,
        ctx: &mut NumberGraphUiContext,
        _data: NumberObjectUiData<()>,
        _number_graph: &mut ExpressionGraph,
    ) {
        NumberSourceUi::new_named(
            handle.id(),
            "WrappingIntegrator".to_string(),
            DisplayStyle::Framed,
        )
        .show(ui, ctx, ui_state);
    }

    fn summon_names(&self) -> &'static [&'static str] {
        &["wrappingintegrator"]
    }

    fn make_ui_state(
        &self,
        _handle: &Self::HandleType,
        _init: UiInitialization,
    ) -> (Self::StateType, NumberSourceLayout) {
        ((), NumberSourceLayout::Function)
    }
}
