use eframe::egui;

use crate::core::graphobject::ObjectType;

fn score_match(query: &str, content: &str) -> f32 {
    if query.len() == 0 {
        return 0.0;
    }
    let mut rate = 2;
    let mut score: u32 = 0;
    let mut q = query.chars();
    let mut qc = q.next().unwrap();
    for c in content.chars() {
        if qc == c {
            rate = 1;
            qc = match q.next() {
                Some(qc) => qc,
                None => break,
            }
        } else {
            score += rate;
        }
    }
    (score as f32) / (content.len() as f32)
}

pub(super) struct SummonWidgetState {
    position: egui::Pos2,
    text: String,
    newly_created: bool,
    should_close: bool,
    selected_type: Option<ObjectType>,
    object_scores: Vec<(ObjectType, f32)>,
}

impl SummonWidgetState {
    pub(super) fn new(
        position: egui::Pos2,
        mut available_objects: Vec<ObjectType>,
    ) -> SummonWidgetState {
        available_objects.sort_by_key(|t| t.name());
        SummonWidgetState {
            position,
            text: String::new(),
            newly_created: true,
            should_close: false,
            selected_type: None,
            object_scores: available_objects.iter().map(|t| (*t, 0.0)).collect(),
        }
    }

    pub(super) fn should_close(&self) -> bool {
        self.should_close
    }

    pub(super) fn selected_type(&self) -> Option<ObjectType> {
        self.selected_type
    }

    fn update_matches(&mut self) {
        for (t, s) in self.object_scores.iter_mut() {
            *s = score_match(&self.text, t.name());
        }
        self.object_scores
            .sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
    }
}

pub(super) struct SummonWidget<'a> {
    state: &'a mut SummonWidgetState,
}

impl<'a> SummonWidget<'a> {
    pub(super) fn new(state: &'a mut SummonWidgetState) -> SummonWidget<'a> {
        SummonWidget { state }
    }
}

impl<'a> egui::Widget for SummonWidget<'a> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let r = egui::Window::new("")
            .id(egui::Id::new("Summon"))
            .title_bar(false)
            .frame(
                egui::Frame::none()
                    .fill(egui::Color32::BLACK)
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
                if t.changed() {
                    self.state.update_matches();
                }
                if ui.input().key_down(egui::Key::Enter) {
                    self.state.should_close = true;
                    self.state.selected_type = self.state.object_scores.get(0).map(|x| x.0);
                }
                if ui.input().key_down(egui::Key::Escape) {
                    self.state.should_close = true;
                }
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for (otype, score) in &self.state.object_scores {
                        let c = (255.0 * (1.0 / (1.0 + score))) as u8;
                        let r = ui.add(
                            egui::Label::new(
                                egui::RichText::new(otype.name())
                                    .color(egui::Color32::from_rgb(c, c, c)),
                            )
                            .sense(egui::Sense::click()),
                        );
                        if r.clicked() {
                            self.state.selected_type = Some(*otype);
                            self.state.should_close = true;
                        }
                    }
                });
            })
            .unwrap();
        r.response
    }
}
