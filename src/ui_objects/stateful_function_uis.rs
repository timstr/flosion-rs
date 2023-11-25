use crate::{
    core::number::{numbergraph::NumberGraph, numbersource::StatefulNumberSourceHandle},
    objects::statefulfunctions::ExponentialApproach,
    ui_core::{
        numbergraphui::NumberGraphUi,
        numbergraphuicontext::NumberGraphUiContext,
        numbergraphuistate::{NumberGraphUiState, NumberObjectUiData},
        numbersourceui::{DisplayStyle, NumberSourceUi},
        object_ui::ObjectUi,
    },
};

#[derive(Default)]
pub struct ExponentialApproachUi {}

impl ObjectUi for ExponentialApproachUi {
    type GraphUi = NumberGraphUi;
    type HandleType = StatefulNumberSourceHandle<ExponentialApproach>;
    type StateType = ();

    fn ui<'a, 'b>(
        &self,
        handle: StatefulNumberSourceHandle<ExponentialApproach>,
        ui_state: &mut NumberGraphUiState,
        ui: &mut eframe::egui::Ui,
        ctx: &mut NumberGraphUiContext,
        _data: NumberObjectUiData<()>,
        _number_graph: &mut NumberGraph,
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
}
