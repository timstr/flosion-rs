use eframe::egui;

use crate::{
    core::sound::soundprocessor::DynamicSoundProcessorHandle,
    objects::mixer::Mixer,
    ui_core::{
        object_ui::ObjectUi, soundgraphui::SoundGraphUi, soundgraphuicontext::SoundGraphUiContext,
        soundgraphuistate::SoundGraphUiState, soundobjectuistate::SoundObjectUiData,
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
        ui_state: &mut SoundGraphUiState,
        ui: &mut egui::Ui,
        ctx: &mut SoundGraphUiContext,
        data: SoundObjectUiData<()>,
    ) {
        let mut objwin = ProcessorUi::new(mixer.id(), "Mixer", data.color);

        for siid in mixer.get_input_ids().into_iter() {
            objwin = objwin.add_sound_input(siid);
        }

        objwin.show_with(ui, ctx, ui_state, |ui, ui_state| {
            ui.horizontal(|ui| {
                let last_input = mixer.get_input_ids().into_iter().last();

                if ui.button("+").clicked() {
                    let w = mixer.clone();
                    ui_state.make_change(move |sg, _| {
                        sg.apply_processor_tools(w.id(), |mut tools| {
                            w.add_input(&mut tools);
                        })
                        .unwrap();
                    });
                }

                if let Some(siid) = last_input {
                    if ui.button("-").clicked() {
                        let w = mixer.clone();
                        ui_state.make_change(move |sg, _| {
                            sg.apply_processor_tools(w.id(), |mut tools| {
                                w.remove_input(siid, &mut tools);
                            })
                            .unwrap();
                        });
                    }
                }
            });
        });
    }
}
