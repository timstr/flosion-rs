use crate::{
    core::{graphobject::ObjectId, soundprocessor::WrappedDynamicSoundProcessor},
    objects::mixer::Mixer,
    ui_core::{
        graph_ui_tools::GraphUITools,
        object_ui::{ObjectUi, ObjectWindow, SoundInputWidget, SoundOutputWidget},
    },
};

#[derive(Default)]
pub struct MixerUi {}

impl ObjectUi for MixerUi {
    type WrapperType = WrappedDynamicSoundProcessor<Mixer>;

    fn ui(
        &self,
        id: ObjectId,
        wrapper: &WrappedDynamicSoundProcessor<Mixer>,
        graph_tools: &mut GraphUITools,
        ui: &mut eframe::egui::Ui,
    ) {
        let object = wrapper.instance();
        let id = id.as_sound_processor_id().unwrap();
        ObjectWindow::new_sound_processor(id).show(ui.ctx(), |ui| {
            ui.label("Mixer");
            ui.add(SoundOutputWidget::new(id, graph_tools));
            for i in object.get_input_ids() {
                ui.add(SoundInputWidget::new(i, graph_tools));
                if ui.button("x").clicked() {
                    let w = wrapper.clone();
                    graph_tools.make_change(move |sg| {
                        w.instance()
                            .remove_input(i, &mut sg.make_tools_for_dynamic_processor(&w));
                    });
                }
            }
            if ui.button("+").clicked() {
                let w = wrapper.clone();
                graph_tools.make_change(move |sg| {
                    w.instance()
                        .add_input(&mut sg.make_tools_for_dynamic_processor(&w));
                });
            }
        });
    }
}
