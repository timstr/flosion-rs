use eframe::egui;

use crate::{
    core::sound::{soundgraph::SoundGraph, soundprocessor::DynamicSoundProcessorHandle},
    objects::mixer::Mixer,
    ui_core::{
        arguments::ParsedArguments, object_ui::ObjectUi, soundgraphui::SoundGraphUi,
        soundgraphuicontext::SoundGraphUiContext, soundgraphuistate::SoundGraphUiState,
        soundprocessorui::ProcessorUi,
    },
};

#[derive(Default)]
pub struct MixerUi {}

impl ObjectUi for MixerUi {
    type GraphUi = SoundGraphUi;
    type HandleType = DynamicSoundProcessorHandle<Mixer>;
    type StateType = ();

    fn ui(
        &self,
        mixer: DynamicSoundProcessorHandle<Mixer>,
        graph_ui_state: &mut SoundGraphUiState,
        ui: &mut egui::Ui,
        ctx: &SoundGraphUiContext,
        _state: &mut (),
        sound_graph: &mut SoundGraph,
    ) {
        let mut objwin = ProcessorUi::new(&mixer, "Mixer");

        for (i, siid) in mixer.get_input_ids().into_iter().enumerate() {
            objwin = objwin.add_sound_input(siid, &format!("input{}", i + 1), sound_graph);
        }

        objwin.show_with(
            ui,
            ctx,
            graph_ui_state,
            sound_graph,
            |ui, _ui_state, sound_graph| {
                ui.horizontal(|ui| {
                    let last_input = mixer.get_input_ids().into_iter().last();

                    if ui.button("+").clicked() {
                        let w = mixer.clone();

                        sound_graph
                            .with_processor_tools(w.id(), |mut tools| {
                                w.add_input(&mut tools);
                                Ok(())
                            })
                            .unwrap();
                    }

                    if let Some(siid) = last_input {
                        if ui.button("-").clicked() {
                            let w = mixer.clone();
                            sound_graph
                                .with_processor_tools(w.id(), |mut tools| {
                                    w.remove_input(siid, &mut tools);
                                    Ok(())
                                })
                                .unwrap();
                        }
                    }
                });
            },
        );
    }

    fn summon_names(&self) -> &'static [&'static str] {
        &["mixer"]
    }

    fn make_properties(&self) -> () {
        ()
    }

    fn make_ui_state(&self, _handle: &Self::HandleType, _args: ParsedArguments) -> Result<(), ()> {
        Ok(())
    }
}
