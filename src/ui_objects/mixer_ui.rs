use crate::{
    core::{graphobject::ObjectId, soundprocessor::DynamicSoundProcessorHandle},
    objects::mixer::Mixer,
    ui_core::{
        graph_ui_state::GraphUIState,
        object_ui::{NoUIState, ObjectUi, ObjectWindow, SoundInputWidget, SoundOutputWidget},
    },
};

#[derive(Default)]
pub struct MixerUi {}

impl ObjectUi for MixerUi {
    type WrapperType = DynamicSoundProcessorHandle<Mixer>;
    type StateType = NoUIState;

    fn ui(
        &self,
        id: ObjectId,
        wrapper: &DynamicSoundProcessorHandle<Mixer>,
        graph_tools: &mut GraphUIState,
        ui: &mut eframe::egui::Ui,
        _state: &NoUIState,
    ) {
        let wrapper = wrapper.clone();
        let object = wrapper.instance();
        let id = id.as_sound_processor_id().unwrap();
        ObjectWindow::new_sound_processor(id).show(ui.ctx(), graph_tools, |ui, graph_tools| {
            ui.label("Mixer");
            ui.add(SoundOutputWidget::new(id, "Output", graph_tools));
            for (i, input_id) in object.get_input_ids().into_iter().enumerate() {
                ui.add(SoundInputWidget::new(
                    input_id,
                    &format!("Input {}", i),
                    graph_tools,
                ));
                if ui.button("x").clicked() {
                    let w = wrapper.clone();
                    graph_tools.make_change(move |sg| {
                        let w = w;
                        let input_id = input_id;
                        sg.apply_processor_tools(w.id(), |mut tools| {
                            w.remove_input(input_id, &mut tools);
                        })
                    });
                }
            }
            if ui.button("+").clicked() {
                let w = wrapper.clone();
                graph_tools.make_change(move |sg| {
                    let w = w;
                    sg.apply_processor_tools(w.id(), |mut tools| {
                        w.add_input(&mut tools);
                    })
                });
            }
        });
    }
}
