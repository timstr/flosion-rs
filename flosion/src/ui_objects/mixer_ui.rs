use eframe::egui;

use crate::{
    core::sound::soundprocessor::SoundProcessorWithId,
    objects::mixer::Mixer,
    ui_core::{
        arguments::ParsedArguments, object_ui::NoObjectUiState,
        soundgraphuicontext::SoundGraphUiContext, soundgraphuistate::SoundGraphUiState,
        soundobjectui::SoundObjectUi, soundprocessorui::ProcessorUi,
    },
};

#[derive(Default)]
pub struct MixerUi {}

impl SoundObjectUi for MixerUi {
    type ObjectType = SoundProcessorWithId<Mixer>;
    type StateType = NoObjectUiState;

    fn ui(
        &self,
        mixer: &mut SoundProcessorWithId<Mixer>,
        graph_ui_state: &mut SoundGraphUiState,
        ui: &mut egui::Ui,
        ctx: &SoundGraphUiContext,
        _state: &mut NoObjectUiState,
    ) {
        let mut objwin = ProcessorUi::new("Mixer");

        for (i, input) in mixer.inputs().iter().enumerate() {
            objwin = objwin.add_sound_input(input, &format!("input{}", i + 1));
        }

        objwin.show_with(mixer, ui, ctx, graph_ui_state, |mixer, ui, _ui_state| {
            ui.horizontal(|ui| {
                let last_input = mixer.inputs().last().map(|i| i.id());

                if ui.button("+").clicked() {
                    mixer.add_input();
                }

                if let Some(siid) = last_input {
                    if ui.button("-").clicked() {
                        mixer.remove_input(siid);
                    }
                }
            });
        });
    }

    fn summon_names(&self) -> &'static [&'static str] {
        &["mixer"]
    }

    fn make_properties(&self) -> () {
        ()
    }

    fn make_ui_state(
        &self,
        _handle: &Self::ObjectType,
        _args: &ParsedArguments,
    ) -> Result<NoObjectUiState, ()> {
        Ok(NoObjectUiState)
    }
}
