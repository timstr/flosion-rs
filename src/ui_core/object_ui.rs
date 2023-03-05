use std::{
    any::{type_name, Any},
    f64::consts::TAU,
};

use eframe::{
    egui::{self},
    epaint::ecolor::{self},
};
use rand::{thread_rng, Rng};

use crate::core::{
    arguments::{ArgumentList, ParsedArguments},
    graphobject::{GraphId, GraphObjectHandle, ObjectHandle, ObjectId, ObjectInitialization},
    numberinput::{NumberInputHandle, NumberInputId},
    numbersource::{
        NumberSourceHandle, NumberSourceId, NumberVisibility, PureNumberSource,
        PureNumberSourceHandle,
    },
    serialization::{Deserializer, Serializable, Serializer},
    soundinput::SoundInputId,
    soundprocessor::SoundProcessorId,
};

use super::{
    diagnostics::DiagnosticRelevance,
    graph_ui_state::GraphUIState,
    object_ui_states::{AnyObjectUiData, AnyObjectUiState},
};

#[derive(Default)]
pub struct NoUIState;

impl Serializable for NoUIState {
    fn serialize(&self, _serializer: &mut Serializer) {
        // Nothing to do
    }

    fn deserialize(_deserializer: &mut Deserializer) -> Result<Self, ()> {
        Ok(Self)
    }
}

pub fn random_object_color() -> egui::Color32 {
    let hue: f32 = thread_rng().gen();
    let color = ecolor::Hsva::new(hue, 1.0, 0.5, 1.0);
    color.into()
}

pub enum UiInitialization<'a> {
    Args(&'a ParsedArguments),
    Default,
}

pub struct ObjectUiData<'a, T: Any + Default + Serializable> {
    pub state: &'a T,
    pub color: egui::Color32,
}

pub trait ObjectUi: 'static + Default {
    type HandleType: ObjectHandle;
    type StateType: Any + Default + Serializable;
    fn ui(
        &self,
        handle: Self::HandleType,
        graph_state: &mut GraphUIState,
        ui: &mut egui::Ui,
        data: ObjectUiData<Self::StateType>,
    );

    fn aliases(&self) -> &'static [&'static str] {
        &[]
    }

    fn arguments(&self) -> ArgumentList {
        ArgumentList::new()
    }

    fn make_ui_state(
        &self,
        _handle: &Self::HandleType,
        _init: UiInitialization,
    ) -> Self::StateType {
        Self::StateType::default()
    }
}

pub trait AnyObjectUi {
    fn apply(
        &self,
        object: &GraphObjectHandle,
        object_ui_state: &AnyObjectUiData,
        graph_state: &mut GraphUIState,
        ui: &mut egui::Ui,
    );

    fn aliases(&self) -> &'static [&'static str];

    fn arguments(&self) -> ArgumentList;

    fn make_ui_state(
        &self,
        object: &GraphObjectHandle,
        init: ObjectInitialization,
    ) -> Result<Box<dyn AnyObjectUiState>, ()>;
}

impl<T: ObjectUi> AnyObjectUi for T {
    fn apply(
        &self,
        object: &GraphObjectHandle,
        object_ui_state: &AnyObjectUiData,
        graph_state: &mut GraphUIState,
        ui: &mut egui::Ui,
    ) {
        let handle = T::HandleType::from_graph_object(object.clone()).unwrap();
        let state_any = object_ui_state.state().as_any();
        debug_assert!(
            state_any.is::<T::StateType>(),
            "AnyObjectUi expected to receive state type {}, but got {:?} instead",
            type_name::<T::StateType>(),
            object_ui_state.state().get_language_type_name()
        );
        let state = state_any.downcast_ref::<T::StateType>().unwrap();
        let data = ObjectUiData {
            state,
            color: object_ui_state.color(),
        };
        self.ui(handle, graph_state, ui, data);
    }

    fn aliases(&self) -> &'static [&'static str] {
        self.aliases()
    }

    fn arguments(&self) -> ArgumentList {
        self.arguments()
    }

    fn make_ui_state(
        &self,
        object: &GraphObjectHandle,
        init: ObjectInitialization,
    ) -> Result<Box<dyn AnyObjectUiState>, ()> {
        // let dc_object = downcast_object_ref::<T>(object.instance());
        let handle = T::HandleType::from_graph_object(object.clone()).unwrap();
        let state: T::StateType = match init {
            ObjectInitialization::Args(a) => self.make_ui_state(&handle, UiInitialization::Args(a)),
            ObjectInitialization::Archive(mut a) => T::StateType::deserialize(&mut a)?,
            ObjectInitialization::Default => self.make_ui_state(&handle, UiInitialization::Default),
        };
        Ok(Box::new(state))
    }
}

pub trait PegCreator {
    fn get_id(&self) -> GraphId;
}

impl PegCreator for &NumberInputHandle {
    fn get_id(&self) -> GraphId {
        debug_assert!(
            self.visibility() == NumberVisibility::Public,
            "Attempted to make a peg for a number input which is private"
        );
        self.id().into()
    }
}

impl PegCreator for &NumberSourceHandle {
    fn get_id(&self) -> GraphId {
        debug_assert!(
            self.visibility() == NumberVisibility::Public,
            "Attempted to make a peg for a number source which is private"
        );
        self.id().into()
    }
}

impl<T: PureNumberSource> PegCreator for &PureNumberSourceHandle<T> {
    fn get_id(&self) -> GraphId {
        self.id().into()
    }
}

impl PegCreator for SoundInputId {
    fn get_id(&self) -> GraphId {
        (*self).into()
    }
}

impl PegCreator for SoundProcessorId {
    fn get_id(&self) -> GraphId {
        (*self).into()
    }
}

pub struct ObjectWindow {
    object_id: ObjectId,
    label: &'static str,
    color: egui::Color32,
    left_pegs: Vec<(GraphId, &'static str)>,
    top_pegs: Vec<(GraphId, &'static str)>,
    right_pegs: Vec<(GraphId, &'static str)>,
}

impl ObjectWindow {
    pub fn new_sound_processor(
        id: SoundProcessorId,
        label: &'static str,
        color: egui::Color32,
    ) -> ObjectWindow {
        ObjectWindow {
            object_id: ObjectId::Sound(id),
            label,
            color,
            left_pegs: Vec::new(),
            top_pegs: Vec::new(),
            right_pegs: Vec::new(),
        }
    }

    pub fn new_number_source(
        id: NumberSourceId,
        label: &'static str,
        color: egui::Color32,
    ) -> ObjectWindow {
        ObjectWindow {
            object_id: ObjectId::Number(id),
            label,
            color,
            left_pegs: Vec::new(),
            top_pegs: Vec::new(),
            right_pegs: Vec::new(),
        }
    }

    pub fn add_left_peg<T: PegCreator>(mut self, peg: T, label: &'static str) -> Self {
        self.left_pegs.push((peg.get_id(), label));
        self
    }

    pub fn add_right_peg<T: PegCreator>(mut self, peg: T, label: &'static str) -> Self {
        self.right_pegs.push((peg.get_id(), label));
        self
    }

    pub fn add_top_peg<T: PegCreator>(mut self, peg: T, label: &'static str) -> Self {
        self.top_pegs.push((peg.get_id(), label));
        self
    }

    fn show_pegs(
        ui: &mut egui::Ui,
        pegs: &[(GraphId, &'static str)],
        direction: PegDirection,
        graph_state: &mut GraphUIState,
    ) {
        for (graph_id, label) in pegs {
            match graph_id {
                GraphId::SoundInput(siid) => {
                    ui.add(SoundInputWidget::new(*siid, label, direction, graph_state));
                }
                GraphId::SoundProcessor(spid) => {
                    ui.add(SoundOutputWidget::new(*spid, label, direction, graph_state));
                }
                GraphId::NumberInput(niid) => {
                    ui.add(NumberInputWidget::new(*niid, label, direction, graph_state));
                }
                GraphId::NumberSource(nsid) => {
                    ui.add(NumberOutputWidget::new(
                        *nsid,
                        label,
                        direction,
                        graph_state,
                    ));
                }
            }
        }
    }

    pub fn show(self, ctx: &egui::Context, graph_tools: &mut GraphUIState) {
        self.show_with(ctx, graph_tools, |_ui, _tools| {});
    }

    pub fn show_with<F: FnOnce(&mut egui::Ui, &mut GraphUIState)>(
        self,
        ctx: &egui::Context,
        graph_tools: &mut GraphUIState,
        add_contents: F,
    ) {
        let s = match self.object_id {
            ObjectId::Sound(id) => format!("SoundObjectWindow {:?}", id),
            ObjectId::Number(id) => format!("NumberObjectWindow {:?}", id),
        };
        let id = egui::Id::new(s);

        let mut area = egui::Area::new(id);
        let mut displacement: Option<egui::Vec2> = None;
        let mut fill = self.color;

        if let Some(relevance) = graph_tools.graph_item_has_warning(self.object_id.into()) {
            match relevance {
                DiagnosticRelevance::Primary => {
                    displacement = Some(egui::vec2(
                        5.0 * (ctx.input().time * TAU * 6.0).sin() as f32,
                        0.0,
                    ));
                    fill = egui::Color32::RED
                }
                DiagnosticRelevance::Secondary => fill = egui::Color32::YELLOW,
            };
            ctx.request_repaint();
        }

        if let Some(state) = graph_tools
            .layout_state()
            .get_object_location(self.object_id)
        {
            let mut pos = state.rect.left_top();
            if let Some(d) = displacement {
                pos += d;
            }
            area = area.current_pos(pos);
        } else if let Some(pos) = ctx.input().pointer.interact_pos() {
            area = area.current_pos(pos);
        }

        if graph_tools.object_has_keyboard_focus(self.object_id) {
            // TODO: how to highlight?
        } else if graph_tools.is_object_selected(self.object_id) {
            let mut hsva = ecolor::Hsva::from(fill);
            hsva.v = 0.5 * (1.0 + hsva.a);
            fill = hsva.into();
        }

        let center_frame = egui::Frame::default()
            .fill(fill)
            .inner_margin(egui::Vec2::splat(5.0));

        let side_frame_template = egui::Frame::default()
            .fill(fill)
            .inner_margin(egui::Vec2::splat(5.0));

        let outer_corner_rounding = 10.0;

        let mut top_frame = side_frame_template;
        let mut left_frame = side_frame_template;
        let mut right_frame = side_frame_template;

        top_frame.rounding.nw = outer_corner_rounding;
        top_frame.rounding.ne = outer_corner_rounding;
        left_frame.rounding.nw = outer_corner_rounding;
        left_frame.rounding.sw = outer_corner_rounding;
        right_frame.rounding.ne = outer_corner_rounding;
        right_frame.rounding.se = outer_corner_rounding;

        let top_row = !self.top_pegs.is_empty();
        let left_col = !self.left_pegs.is_empty();
        let right_col = !self.right_pegs.is_empty();

        area = area
            .movable(true)
            .constrain(false)
            .drag_bounds(egui::Rect::EVERYTHING);
        let r = area.show(ctx, |ui| {
            // Clip to the entire screen, not just outside the area
            ui.set_clip_rect(ctx.input().screen_rect());

            egui::Grid::new(id.with("grid"))
                .min_col_width(0.0)
                .min_row_height(0.0)
                .spacing(egui::vec2(0.0, 0.0))
                .show(ui, |ui| {
                    if top_row {
                        if left_col {
                            ui.label(""); // TODO: better way to make empty cell?
                        }
                        top_frame.show(ui, |ui| {
                            ui.horizontal(|ui| {
                                Self::show_pegs(ui, &self.top_pegs, PegDirection::Top, graph_tools)
                            });
                        });
                        ui.end_row();
                    }

                    if left_col {
                        ui.vertical(|ui| {
                            left_frame.show(ui, |ui| {
                                Self::show_pegs(
                                    ui,
                                    &self.left_pegs,
                                    PegDirection::Left,
                                    graph_tools,
                                )
                            })
                        });
                    }
                    center_frame.show(ui, |ui| {
                        ui.vertical(|ui| {
                            ui.add(
                                egui::Label::new(
                                    egui::RichText::new(self.label)
                                        .color(egui::Color32::BLACK)
                                        .strong(),
                                )
                                .wrap(false),
                            );
                            add_contents(ui, graph_tools);
                            ui.allocate_space(ui.available_size());
                        });
                    });
                    if right_col {
                        ui.vertical(|ui| {
                            right_frame.show(ui, |ui| {
                                Self::show_pegs(
                                    ui,
                                    &self.right_pegs,
                                    PegDirection::Right,
                                    graph_tools,
                                )
                            })
                        });
                    }
                    ui.end_row();
                });
        });

        // Track the un-displaced object location so that displacement
        // does not accumulate between frames
        graph_tools.layout_state_mut().track_object_location(
            self.object_id,
            r.response
                .rect
                .translate(-displacement.unwrap_or(egui::Vec2::ZERO)),
            r.response.layer_id,
        );

        if r.response.drag_started() {
            if !graph_tools.is_object_selected(self.object_id) {
                graph_tools.clear_selection();
                graph_tools.select_object(self.object_id);
            }
        }

        if r.response.dragged() {
            graph_tools.move_selection(r.response.drag_delta());
        }

        if r.response.clicked() {
            if !graph_tools.is_object_selected(self.object_id) {
                graph_tools.clear_selection();
                graph_tools.select_object(self.object_id);
            }
        }
    }
}

fn key_to_string(key: egui::Key) -> String {
    match key {
        egui::Key::A => "A".to_string(),
        egui::Key::B => "B".to_string(),
        egui::Key::C => "C".to_string(),
        egui::Key::D => "D".to_string(),
        egui::Key::E => "E".to_string(),
        egui::Key::F => "F".to_string(),
        egui::Key::G => "G".to_string(),
        egui::Key::H => "H".to_string(),
        egui::Key::I => "I".to_string(),
        egui::Key::J => "J".to_string(),
        egui::Key::K => "K".to_string(),
        egui::Key::L => "L".to_string(),
        egui::Key::M => "M".to_string(),
        egui::Key::N => "N".to_string(),
        egui::Key::O => "O".to_string(),
        egui::Key::P => "P".to_string(),
        egui::Key::Q => "Q".to_string(),
        egui::Key::R => "R".to_string(),
        egui::Key::S => "S".to_string(),
        egui::Key::T => "T".to_string(),
        egui::Key::U => "U".to_string(),
        egui::Key::V => "V".to_string(),
        egui::Key::W => "W".to_string(),
        egui::Key::X => "X".to_string(),
        egui::Key::Y => "Y".to_string(),
        egui::Key::Z => "Z".to_string(),
        _ => "???".to_string(),
    }
}

#[derive(Copy, Clone)]
pub enum PegDirection {
    Left,
    Top,
    Right,
}

fn peg_ui(
    id: GraphId,
    color: egui::Color32,
    label: &str,
    direction: PegDirection,
    ui_state: &mut GraphUIState,
    ui: &mut egui::Ui,
) -> egui::Response {
    let (peg_rect, response) =
        ui.allocate_exact_size(egui::Vec2::new(20.0, 20.0), egui::Sense::drag());
    ui_state
        .layout_state_mut()
        .track_peg(id, peg_rect, response.layer_id, direction);
    let display_str;
    let popup_str;
    let size_diff;
    if ui_state.peg_has_keyboard_focus(id) {
        display_str = "*".to_string();
        popup_str = None;
        size_diff = 5.0;
    } else if let Some(key) = ui_state.peg_has_hotkey(id) {
        display_str = key_to_string(key);
        popup_str = Some(label);
        size_diff = 0.0;
    } else {
        display_str = format!("{}", id.as_usize());
        size_diff = 0.0;
        popup_str = if response.hovered() {
            Some(label)
        } else {
            None
        };
        // display_str = "-".to_string();
        // size_diff = -3.0;
    }
    let painter = ui.painter();
    painter.rect(
        peg_rect.expand(size_diff),
        5.0,
        color,
        egui::Stroke::new(2.0, egui::Color32::WHITE),
    );
    painter.text(
        peg_rect.center(),
        egui::Align2::CENTER_CENTER,
        display_str,
        egui::FontId::monospace(16.0),
        egui::Color32::WHITE,
    );
    // TODO: also show label when wires are being dragged
    if let Some(s) = popup_str {
        let galley = painter.layout_no_wrap(
            s.to_string(),
            egui::FontId::monospace(16.0),
            egui::Color32::WHITE,
        );
        let pos = match direction {
            PegDirection::Left => {
                peg_rect.left_center()
                    + egui::vec2(-5.0 - galley.rect.width(), -0.5 * galley.rect.height())
            }
            PegDirection::Top => {
                peg_rect.center_top()
                    + egui::vec2(-0.5 * galley.rect.width(), -5.0 - galley.rect.height())
            }
            PegDirection::Right => {
                peg_rect.right_center() + egui::vec2(5.0, -0.5 * galley.rect.height())
            }
        };
        painter.rect(
            galley.rect.expand(3.0).translate(pos.to_vec2()),
            3.0,
            egui::Color32::BLACK,
            egui::Stroke::new(2.0, egui::Color32::WHITE),
        );
        painter.galley(pos, galley);
    }
    if response.drag_started() {
        ui_state.start_dragging(id);
    }
    if response.drag_released() {
        ui_state.stop_dragging(Some(response.interact_pointer_pos().unwrap()));
    }
    response
}

struct SoundInputWidget<'a> {
    sound_input_id: SoundInputId,
    label: &'a str,
    direction: PegDirection,
    graph_state: &'a mut GraphUIState,
}

impl<'a> SoundInputWidget<'a> {
    fn new(
        sound_input_id: SoundInputId,
        label: &'a str,
        direction: PegDirection,
        graph_state: &'a mut GraphUIState,
    ) -> SoundInputWidget<'a> {
        SoundInputWidget {
            sound_input_id,
            label,
            direction,
            graph_state,
        }
    }
}

impl<'a> egui::Widget for SoundInputWidget<'a> {
    fn ui(self, ui: &mut egui::Ui) -> eframe::egui::Response {
        peg_ui(
            self.sound_input_id.into(),
            egui::Color32::from_rgb(0, 255, 0),
            self.label,
            self.direction,
            self.graph_state,
            ui,
        )
    }
}

struct SoundOutputWidget<'a> {
    sound_processor_id: SoundProcessorId,
    label: &'a str,
    direction: PegDirection,
    graph_state: &'a mut GraphUIState,
}

impl<'a> SoundOutputWidget<'a> {
    fn new(
        sound_processor_id: SoundProcessorId,
        label: &'a str,
        direction: PegDirection,
        graph_state: &'a mut GraphUIState,
    ) -> SoundOutputWidget<'a> {
        SoundOutputWidget {
            sound_processor_id,
            label,
            direction,
            graph_state,
        }
    }
}

impl<'a> egui::Widget for SoundOutputWidget<'a> {
    fn ui(self, ui: &mut egui::Ui) -> eframe::egui::Response {
        peg_ui(
            self.sound_processor_id.into(),
            egui::Color32::from_rgb(0, 128, 0),
            self.label,
            self.direction,
            self.graph_state,
            ui,
        )
    }
}

struct NumberInputWidget<'a> {
    number_input_id: NumberInputId,
    label: &'a str,
    direction: PegDirection,
    graph_state: &'a mut GraphUIState,
}

impl<'a> NumberInputWidget<'a> {
    fn new(
        number_input_id: NumberInputId,
        label: &'a str,
        direction: PegDirection,
        graph_state: &'a mut GraphUIState,
    ) -> NumberInputWidget<'a> {
        NumberInputWidget {
            number_input_id,
            label,
            direction,
            graph_state,
        }
    }
}

impl<'a> egui::Widget for NumberInputWidget<'a> {
    fn ui(self, ui: &mut egui::Ui) -> eframe::egui::Response {
        peg_ui(
            self.number_input_id.into(),
            egui::Color32::from_rgb(0, 0, 255),
            self.label,
            self.direction,
            self.graph_state,
            ui,
        )
    }
}

struct NumberOutputWidget<'a> {
    number_source_id: NumberSourceId,
    label: &'a str,
    direction: PegDirection,
    graph_state: &'a mut GraphUIState,
}

impl<'a> NumberOutputWidget<'a> {
    fn new(
        number_source_id: NumberSourceId,
        label: &'a str,
        direction: PegDirection,
        graph_state: &'a mut GraphUIState,
    ) -> NumberOutputWidget<'a> {
        NumberOutputWidget {
            number_source_id,
            label,
            direction,
            graph_state,
        }
    }
}

impl<'a> egui::Widget for NumberOutputWidget<'a> {
    fn ui(self, ui: &mut egui::Ui) -> eframe::egui::Response {
        peg_ui(
            self.number_source_id.into(),
            egui::Color32::from_rgb(0, 0, 128),
            self.label,
            self.direction,
            self.graph_state,
            ui,
        )
    }
}
