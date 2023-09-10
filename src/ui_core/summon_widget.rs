use std::cmp::Ordering;

use eframe::egui;

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

#[derive(Eq, PartialEq)]
enum SummonRule<T> {
    BasicName(String, T),
    Pattern(String, fn(&str) -> Option<T>),
}

impl<T: Copy> SummonRule<T> {
    fn display_name(&self) -> &str {
        match self {
            SummonRule::BasicName(name, _) => name,
            SummonRule::Pattern(name, _) => name,
        }
    }

    fn evaluate(&self, prompt: &str) -> Option<T> {
        match self {
            SummonRule::BasicName(_, value) => Some(*value),
            SummonRule::Pattern(_, f) => f(prompt),
        }
    }

    fn default_value(&self) -> Option<T> {
        match self {
            SummonRule::BasicName(_, value) => Some(*value),
            SummonRule::Pattern(_, _) => None,
        }
    }
}

struct ScoredRule<T> {
    rule: SummonRule<T>,
    score: f32,
    value: Option<T>,
}

impl<T: Copy> ScoredRule<T> {
    fn update(&mut self, prompt: &str) {
        self.value = self.rule.evaluate(prompt);
        self.score = match &self.rule {
            SummonRule::BasicName(name, _) => score_match(prompt, name),
            SummonRule::Pattern(_, _) => 0.0,
        }
    }
}

impl<T: Copy> PartialEq for ScoredRule<T> {
    fn eq(&self, other: &Self) -> bool {
        self.rule.display_name() == other.rule.display_name() && self.score == other.score
    }
}

impl<T: Copy> Eq for ScoredRule<T> {}

impl<T: Copy> PartialOrd for ScoredRule<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<T: Copy> Ord for ScoredRule<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        // matching patterns come before basic names.
        // Non-matching patterns come after.
        let rate = |scored_rule: &ScoredRule<T>| -> u8 {
            match scored_rule.rule {
                SummonRule::BasicName(_, _) => 1,
                SummonRule::Pattern(_, _) => {
                    if scored_rule.value.is_some() {
                        0
                    } else {
                        2
                    }
                }
            }
        };

        match rate(self).cmp(&rate(other)) {
            Ordering::Equal => (),
            cmp => return cmp,
        };

        // higher scores come before higher scores
        match self
            .score
            .partial_cmp(&other.score)
            .unwrap_or(Ordering::Equal)
        {
            Ordering::Equal => (),
            cmp => return cmp.reverse(),
        };
        // break any remaining ties with names
        self.rule.display_name().cmp(other.rule.display_name())
    }
}

pub(super) struct SummonWidgetState<T> {
    position: egui::Pos2,
    text: String,
    finalized: bool,
    current_choice: Option<T>,
    rules: Vec<ScoredRule<T>>,
    focus_index: Option<usize>,
}

// TYPES OF THINGS THAT COULD BE SUMMONED
// simple names of graph objects ("dac", "whitenoise", "plus", "sqrt")
//  -> each of these corresponds to an ObjectType
// graph object names with arguments ("slider 0 1", "constant 44", "audioclip ~/sounds/bleepbloop.wav")
//  -> each of these corresponds to an ObjectType plus a list of arguments.
//     Admittedly, the lack of structure of these arguments is currently a pain point.
//     Rather than just having to compare string names, it would be better to have a
//     unique type per object and object ui. But arguments are used literally only by
//     constant and slider currently so I'll put this off for now.
// aliases graph objects ("+", "-")
//  -> each of these corresponds to an ObjectType
// patterns ("1", "0.1", "2e5")
//  -> in this case, this corresponds to an ObjectType (e.g constant) plus a custom value that requires parsing
// names of number sources ("wavegenerator.phase", "ensemble.notefrequency", "dac.time")
//  -> these correspond to SoundNumberSource instances and are unique to number graphs
//  -> This also makes more sense if sound sources can be given custom names for disambiguation
//  -> This also makes more sense if sound number sources are given names in their respective UIs

// names fuzzy-match
// rules like "12564" for constants either match or don't, but this one is distinct enough to merit putting on top if it matches at all
// arguments are not considered during matching
//  -> also who cares about arguments if I'm about to rewrite this to be more general?
//

pub(super) struct SummonWidgetStateBuilder<T> {
    position: egui::Pos2,
    rules: Vec<SummonRule<T>>,
}

impl<T: Copy> SummonWidgetStateBuilder<T> {
    pub(super) fn new(position: egui::Pos2) -> SummonWidgetStateBuilder<T> {
        SummonWidgetStateBuilder {
            position,
            rules: Vec::new(),
        }
    }

    pub(super) fn add_basic_name(&mut self, name: String, value: T) -> &mut Self {
        self.rules.push(SummonRule::BasicName(name, value));
        self
    }

    pub(super) fn add_pattern(&mut self, name: String, f: fn(&str) -> Option<T>) -> &mut Self {
        self.rules.push(SummonRule::Pattern(name, f));
        self
    }

    pub(super) fn build(self) -> SummonWidgetState<T> {
        let mut rules: Vec<ScoredRule<T>> = self
            .rules
            .into_iter()
            .map(|rule| {
                let value = rule.default_value();
                ScoredRule {
                    rule,
                    score: 0.0,
                    value,
                }
            })
            .collect();
        rules.sort();
        SummonWidgetState {
            position: self.position,
            text: String::new(),
            finalized: false,
            current_choice: None,
            rules,
            focus_index: None,
        }
    }
}

impl<T: Copy> SummonWidgetState<T> {
    pub(super) fn final_choice(&self) -> Option<T> {
        if self.finalized {
            self.current_choice
        } else {
            None
        }
    }

    pub(super) fn set_text(&mut self, s: String) {
        self.text = s;
        self.update_matches();
    }

    fn update_matches(&mut self) {
        for rule in &mut self.rules {
            rule.update(&self.text);
        }
        self.rules.sort();
    }

    pub(super) fn position(&self) -> egui::Pos2 {
        self.position
    }
}

pub(super) struct SummonWidget<'a, T> {
    state: &'a mut SummonWidgetState<T>,
}

impl<'a, T> SummonWidget<'a, T> {
    pub(super) fn new(state: &'a mut SummonWidgetState<T>) -> SummonWidget<'a, T> {
        SummonWidget { state }
    }
}

impl<'a, T: Copy> egui::Widget for SummonWidget<'a, T> {
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
                    let mut new_focus_index = self.state.focus_index;
                    let num_rules = self.state.rules.len();
                    if num_rules == 0 {
                        new_focus_index = None;
                    } else {
                        if ui.input_mut(|i| {
                            i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown)
                        }) {
                            new_focus_index = match self.state.focus_index {
                                None => Some(0),
                                Some(i) => Some((i + 1).min(num_rules - 1)),
                            };
                        }
                        if ui
                            .input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp))
                        {
                            new_focus_index = match self.state.focus_index {
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
                    focus_changed = new_focus_index == self.state.focus_index;
                    self.state.focus_index = new_focus_index;
                }

                let textedit = egui::TextEdit::singleline(&mut self.state.text)
                    .cursor_at_end(true)
                    .lock_focus(true);
                let t = textedit.ui(ui);
                t.request_focus();
                if t.changed() {
                    self.state.update_matches();
                }
                if ui.input_mut(|i| {
                    i.consume_key(egui::Modifiers::NONE, egui::Key::Enter)
                        || i.consume_key(egui::Modifiers::NONE, egui::Key::Tab)
                }) {
                    self.state.finalized = true;
                    self.state.current_choice = self.state.rules.get(0).map(|x| x.value).flatten();
                }
                if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
                    self.state.finalized = true;
                    self.state.current_choice = None;
                }
                if t.gained_focus() {
                    self.state.focus_index = None;
                } else if focus_changed && self.state.focus_index == None {
                    t.request_focus();
                }

                egui::ScrollArea::vertical().show(ui, |ui| {
                    for (index, scored_rule) in self.state.rules.iter().enumerate() {
                        let mut layout_job = egui::text::LayoutJob::default();
                        layout_job.append(
                            scored_rule.rule.display_name(),
                            0.0,
                            egui::TextFormat {
                                color: egui::Color32::WHITE,
                                ..Default::default()
                            },
                        );

                        // TESTING displaying rule type and score
                        {
                            layout_job.append(
                                &format!("s={}", scored_rule.score),
                                5.0,
                                egui::TextFormat {
                                    color: egui::Color32::GREEN,
                                    ..Default::default()
                                },
                            );
                            layout_job.append(
                                match scored_rule.rule {
                                    SummonRule::BasicName(_, _) => "name",
                                    SummonRule::Pattern(_, _) => "pattern",
                                },
                                5.0,
                                egui::TextFormat {
                                    color: egui::Color32::GREEN,
                                    ..Default::default()
                                },
                            );
                        }

                        let r = ui.add(egui::Label::new(layout_job).sense(egui::Sense::click()));

                        if r.clicked() {
                            self.state.current_choice = scored_rule.value;
                            self.state.finalized = true;
                        }
                        if r.hovered() {
                            ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::PointingHand);
                        }
                        if r.gained_focus() {
                            self.state.focus_index = Some(index);
                        } else {
                            if let Some(i) = self.state.focus_index {
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
