use eframe::egui;

use crate::{core::graphobject::ObjectType, ui_objects::all_objects::AllObjects};

pub(super) struct SummonWidgetState {
    position: egui::Pos2,
    text: String,
    newly_created: bool,
    should_close: bool,
    selected_type: Option<ObjectType>,
}

impl SummonWidgetState {
    pub(super) fn new(position: egui::Pos2) -> SummonWidgetState {
        SummonWidgetState {
            position,
            text: String::new(),
            newly_created: true,
            should_close: false,
            selected_type: None,
        }
    }

    pub(super) fn should_close(&self) -> bool {
        self.should_close
    }

    pub(super) fn selected_type(&self) -> Option<ObjectType> {
        self.selected_type
    }
}

pub(super) struct SummonWidget<'a> {
    all_objects: &'a AllObjects,
    state: &'a mut SummonWidgetState,
}

impl<'a> SummonWidget<'a> {
    pub(super) fn new(
        all_objects: &'a AllObjects,
        state: &'a mut SummonWidgetState,
    ) -> SummonWidget<'a> {
        SummonWidget { all_objects, state }
    }
}

impl<'a> egui::Widget for SummonWidget<'a> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let r = egui::Window::new("")
            .id(egui::Id::new("Summon"))
            .title_bar(false)
            .frame(
                egui::Frame::none()
                    .fill(egui::Color32::DARK_GRAY)
                    .stroke(egui::Stroke::new(2.0, egui::Color32::WHITE))
                    .margin(egui::Vec2::splat(5.0)),
            )
            .resizable(false)
            .fixed_pos(self.state.position)
            .show(ui.ctx(), |ui| {
                let t = ui.text_edit_singleline(&mut self.state.text);
                if self.state.newly_created {
                    t.request_focus();
                    self.state.newly_created = false;
                }
                // TODO: use t.changed() to efficiently search
                if ui.input().key_down(egui::Key::Enter) {
                    self.state.should_close = true;
                    println!(
                        "TODO: choose the best matching type for \"{}\"",
                        self.state.text
                    );
                }
                if ui.input().key_down(egui::Key::Escape) {
                    self.state.should_close = true;
                }
                egui::ScrollArea::vertical().show(ui, |ui| {
                    let mut all_types = self.all_objects.all_object_types();
                    all_types.sort_by_key(|t| t.name());
                    for t in all_types {
                        let r = ui.add(egui::Label::new(t.name()).sense(egui::Sense::click()));
                        if r.clicked() {
                            self.state.selected_type = Some(t);
                            self.state.should_close = true;
                        }
                    }
                });
            })
            .unwrap();
        r.response
    }
}
