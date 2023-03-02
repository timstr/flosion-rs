use crate::{
    core::soundprocessor::DynamicSoundProcessorHandle,
    objects::mixer::Mixer,
    ui_core::{
        graph_ui_state::GraphUIState,
        object_ui::{NoUIState, ObjectUi, ObjectWindow},
    },
};

#[derive(Default)]
pub struct MixerUi {}

impl ObjectUi for MixerUi {
    type HandleType = DynamicSoundProcessorHandle<Mixer>;
    type StateType = NoUIState;

    fn ui(
        &self,
        mixer: DynamicSoundProcessorHandle<Mixer>,
        graph_tools: &mut GraphUIState,
        ui: &mut eframe::egui::Ui,
        _state: &NoUIState,
    ) {
        let mut objwin =
            ObjectWindow::new_sound_processor(mixer.id()).add_right_peg(mixer.id(), "Output");

        for (i, siid) in mixer.get_input_ids().into_iter().enumerate() {
            objwin = objwin.add_left_peg(siid, "Input ???"); // TODO: allow String, then use format!("Input {}", i + 1));
        }

        objwin.show(ui.ctx(), graph_tools, |ui, graph_tools| {
            ui.label("Mixer");
            let last_input = mixer.get_input_ids().into_iter().last();

            if ui.button("+").clicked() {
                let w = mixer.clone();
                graph_tools.make_change(move |sg| {
                    sg.apply_processor_tools(w.id(), |mut tools| {
                        w.add_input(&mut tools);
                    })
                    .unwrap();
                });
            }

            if let Some(siid) = last_input {
                if ui.button("-").clicked() {
                    let w = mixer.clone();
                    graph_tools.make_change(move |sg| {
                        sg.apply_processor_tools(w.id(), |mut tools| {
                            w.remove_input(siid, &mut tools);
                        })
                        .unwrap();
                    });
                }
            }
        });
    }
}
