use eframe::egui;

use crate::{
    core::number::{numbergraph::NumberGraph, numbersource::StatefulNumberSourceHandle},
    objects::sampler1d::Sampler1d,
    ui_core::{
        numbergraphui::NumberGraphUi,
        numbergraphuicontext::NumberGraphUiContext,
        numbergraphuistate::{NumberGraphUiState, NumberObjectUiData},
        numbersourceui::{DisplayStyle, NumberSourceUi},
        object_ui::ObjectUi,
    },
};

#[derive(Default)]
pub struct Sampler1dUi {}

impl ObjectUi for Sampler1dUi {
    type GraphUi = NumberGraphUi;
    type HandleType = StatefulNumberSourceHandle<Sampler1d>;
    type StateType = ();

    fn ui<'a, 'b>(
        &self,
        sampler1d: StatefulNumberSourceHandle<Sampler1d>,
        ui_state: &mut NumberGraphUiState,
        ui: &mut eframe::egui::Ui,
        ctx: &mut NumberGraphUiContext,
        _data: NumberObjectUiData<()>,
        _number_graph: &mut NumberGraph,
    ) {
        NumberSourceUi::new_named(
            sampler1d.id(),
            "Sampler1d".to_string(),
            DisplayStyle::Framed,
        )
        .show_with(ui, ctx, ui_state, |ui, ui_state| {
            // TODO: render samples as a plot, edit the samples when clicked and dragged
            let mut values = sampler1d.value().read().to_vec();

            let r = ui.add(egui::Slider::new(&mut values[0], -1.0..=1.0));

            if r.changed() {
                sampler1d.value().write(&values);
            }
        });
    }

    fn summon_names(&self) -> &'static [&'static str] {
        &["sampler1d"]
    }
}
