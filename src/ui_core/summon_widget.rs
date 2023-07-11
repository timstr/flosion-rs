use eframe::egui;

use crate::core::arguments::{ArgumentList, ParsedArguments};

use super::{soundgraphui::SoundGraphUi, ui_factory::UiFactory};

fn score_match(query: &str, content: &str) -> f32 {
    let mut score: f32 = 0.0;
    let mut qi = query.chars();
    let mut qc = qi.next();
    let mut first = true;
    for cc in content.chars() {
        if let Some(c) = qc {
            if cc == c {
                score += if first { 2.0 } else { 1.0 };
                qc = qi.next();
            } else {
                score -= 0.2;
                first = false;
            }
        } else {
            score -= 0.1;
            first = false;
        }
    }
    score
}

struct MatchingObject {
    object_type_str: String,
    display_str: String,
    arguments: ArgumentList,
}

// TODO: consider making this shareable between sound and number graphs
pub(super) struct SummonWidgetState {
    position: egui::Pos2,
    text: String,
    newly_created: bool,
    ready: bool,
    selected_type: Option<String>,
    object_scores: Vec<(MatchingObject, f32)>,
    focus_object_index: Option<usize>,
}

impl SummonWidgetState {
    pub(super) fn new(
        position: egui::Pos2,
        all_objects: &UiFactory<SoundGraphUi>,
    ) -> SummonWidgetState {
        let mut object_scores: Vec<(MatchingObject, f32)> = Vec::new();
        for t in all_objects.all_object_types() {
            let ui = all_objects.get_object_ui(t);
            object_scores.push((
                MatchingObject {
                    object_type_str: t.to_string(),
                    display_str: t.to_string(),
                    arguments: ui.arguments(),
                },
                0.0,
            ));
            for alias in ui.aliases() {
                object_scores.push((
                    MatchingObject {
                        object_type_str: t.to_string(),
                        display_str: alias.to_string(),
                        arguments: ui.arguments(),
                    },
                    0.0,
                ))
            }
        }

        object_scores.sort_by(|a, b| a.0.display_str.cmp(&b.0.display_str));

        SummonWidgetState {
            position,
            text: String::new(),
            newly_created: true,
            ready: false,
            selected_type: None,
            object_scores,
            focus_object_index: None,
        }
    }

    pub(super) fn ready(&self) -> bool {
        self.ready
    }

    pub(super) fn selected_type(&self) -> Option<&str> {
        self.selected_type.as_ref().map(|s| &**s)
    }

    pub(super) fn parse_selected(&self) -> (&str, ParsedArguments) {
        let selected: &str = &*self.selected_type.as_ref().unwrap();
        let object = &self
            .object_scores
            .iter()
            .find(|(o, _)| o.object_type_str == selected)
            .unwrap()
            .0;
        debug_assert!(selected == object.object_type_str);
        let args_str: Vec<&str> = self.text.split_whitespace().collect();
        let args = object.arguments.parse(if args_str.len() >= 1 {
            &args_str[1..]
        } else {
            &[]
        });
        (&object.object_type_str, args)
    }

    fn update_matches(&mut self) {
        for (o, s) in self.object_scores.iter_mut() {
            *s = score_match(&self.text, &o.display_str);
        }
        self.object_scores
            .sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap().reverse());
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
                    .inner_margin(egui::Vec2::splat(5.0)),
            )
            .resizable(false)
            .fixed_pos(self.state.position)
            .show(ui.ctx(), |ui| {
                let focus_changed;
                {
                    let mut new_focus_object_index = self.state.focus_object_index;
                    let num_objects = self.state.object_scores.len();
                    if num_objects == 0 {
                        new_focus_object_index = None;
                    } else {
                        if ui.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
                            new_focus_object_index = match self.state.focus_object_index {
                                None => Some(0),
                                Some(i) => Some((i + 1).min(self.state.object_scores.len() - 1)),
                            };
                        }
                        if ui.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
                            new_focus_object_index = match self.state.focus_object_index {
                                None => None,
                                Some(i) => {
                                    if i > 0 {
                                        Some(i - 1)
                                    } else {
                                        None
                                    }
                                }
                            };
                        }
                    }
                    focus_changed = new_focus_object_index == self.state.focus_object_index;
                    self.state.focus_object_index = new_focus_object_index;
                }

                let t = ui.text_edit_singleline(&mut self.state.text);
                if self.state.newly_created {
                    t.request_focus();
                    self.state.newly_created = false;
                }
                if t.changed() {
                    self.state.update_matches();
                }
                if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    self.state.ready = true;
                    self.state.selected_type = self
                        .state
                        .object_scores
                        .get(0)
                        .map(|x| x.0.object_type_str.clone());
                }
                if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    self.state.ready = true;
                }
                if t.gained_focus() {
                    self.state.focus_object_index = None;
                } else if focus_changed && self.state.focus_object_index == None {
                    t.request_focus();
                }

                egui::ScrollArea::vertical().show(ui, |ui| {
                    let max_score = self.state.object_scores.last().unwrap().1;
                    let min_score = self.state.object_scores.first().unwrap().1;
                    let k = if max_score == min_score {
                        1.0
                    } else {
                        1.0 / (max_score - min_score)
                    };
                    for (index, (object, score)) in self.state.object_scores.iter().enumerate() {
                        let t = 1.0 - (score - min_score) * k;
                        debug_assert!(t >= 0.0 && t <= 1.0);
                        let c = 64_u8 + ((255 - 64) as f32 * t) as u8;
                        let mut layout_job = egui::text::LayoutJob::default();
                        layout_job.append(
                            &object.display_str,
                            0.0,
                            egui::TextFormat {
                                color: egui::Color32::from_rgb(c, c, c),
                                ..Default::default()
                            },
                        );
                        if object.display_str != object.object_type_str {
                            layout_job.append(
                                &format!("={}", object.object_type_str),
                                5.0,
                                egui::TextFormat {
                                    color: egui::Color32::from_rgba_unmultiplied(c, c, c, 128),
                                    italics: true,
                                    ..Default::default()
                                },
                            );
                        }
                        for arg in object.arguments.items() {
                            layout_job.append(
                                arg.name(),
                                5.0,
                                egui::TextFormat {
                                    color: egui::Color32::from_rgba_unmultiplied(0, c, 0, 128),
                                    italics: true,
                                    ..Default::default()
                                },
                            );
                        }
                        let r = ui.add(egui::Label::new(layout_job).sense(egui::Sense::click()));
                        if r.clicked() {
                            self.state.selected_type = Some(object.object_type_str.clone());
                            self.state.ready = true;
                        }
                        if r.hovered() {
                            ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::PointingHand);
                        }
                        if r.gained_focus() {
                            self.state.focus_object_index = Some(index);
                        } else {
                            if let Some(i) = self.state.focus_object_index {
                                if focus_changed && i == index {
                                    r.request_focus();
                                }
                            }
                        }
                    }
                });
            })
            .unwrap();
        r.response
    }
}
