use std::{
    any::{type_name, Any},
    cell::RefCell,
    rc::Rc,
};

use eframe::egui::{self};

use crate::core::{
    arguments::{ArgumentList, ParsedArguments},
    graphobject::{GraphId, GraphObject, ObjectId, ObjectInitialization, TypedGraphObject},
    numberinput::NumberInputId,
    numbersource::NumberSourceId,
    serialization::{Deserializer, Serializable, Serializer},
    soundinput::SoundInputId,
    soundprocessor::SoundProcessorId,
};

use super::graph_ui_state::{GraphUIState, ObjectUiState};

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

pub enum UiInitialization<'a> {
    Args(&'a ParsedArguments),
    Default,
}

pub trait ObjectUi: 'static + Default {
    type WrapperType: TypedGraphObject;
    type StateType: Any + Default + Serializable;
    fn ui(
        &self,
        id: ObjectId,
        wrapper: &Self::WrapperType,
        graph_state: &mut GraphUIState,
        ui: &mut egui::Ui,
        state: &Self::StateType,
    );

    fn aliases(&self) -> &'static [&'static str] {
        &[]
    }

    fn arguments(&self) -> ArgumentList {
        ArgumentList::new()
    }

    fn make_ui_state(
        &self,
        _wrapper: &Self::WrapperType,
        _init: UiInitialization,
    ) -> Self::StateType {
        Self::StateType::default()
    }
}

pub trait AnyObjectUi {
    fn apply(
        &self,
        id: ObjectId,
        object: &dyn GraphObject,
        object_ui_state: &dyn ObjectUiState,
        graph_state: &mut GraphUIState,
        ui: &mut egui::Ui,
    );

    fn aliases(&self) -> &'static [&'static str];

    fn arguments(&self) -> ArgumentList;

    fn make_ui_state(
        &self,
        object: &dyn GraphObject,
        init: ObjectInitialization,
    ) -> Result<Rc<RefCell<dyn ObjectUiState>>, ()>;
}

fn downcast_object<T: ObjectUi>(object: &dyn GraphObject) -> &T::WrapperType {
    let any = object.as_any();
    debug_assert!(
        any.is::<T::WrapperType>(),
        "AnyObjectUi expected to receive type {}, but got {} instead",
        type_name::<T::WrapperType>(),
        object.get_language_type_name()
    );
    any.downcast_ref::<T::WrapperType>().unwrap()
}

impl<T: ObjectUi> AnyObjectUi for T {
    fn apply(
        &self,
        id: ObjectId,
        object: &dyn GraphObject,
        object_ui_state: &dyn ObjectUiState,
        graph_state: &mut GraphUIState,
        ui: &mut egui::Ui,
    ) {
        let dc_object = downcast_object::<T>(object);
        let state_any = object_ui_state.as_any();
        debug_assert!(
            state_any.is::<T::StateType>(),
            "AnyObjectUi expected to receive state type {}, but got {:?} instead",
            type_name::<T::StateType>(),
            object_ui_state.get_language_type_name()
        );
        let state = state_any.downcast_ref::<T::StateType>().unwrap();
        self.ui(id, dc_object, graph_state, ui, state);
    }

    fn aliases(&self) -> &'static [&'static str] {
        self.aliases()
    }

    fn arguments(&self) -> ArgumentList {
        self.arguments()
    }

    fn make_ui_state(
        &self,
        object: &dyn GraphObject,
        init: ObjectInitialization,
    ) -> Result<Rc<RefCell<dyn ObjectUiState>>, ()> {
        let dc_object = downcast_object::<T>(object);
        let state: T::StateType = match init {
            ObjectInitialization::Args(a) => {
                self.make_ui_state(dc_object, UiInitialization::Args(a))
            }
            ObjectInitialization::Archive(mut a) => T::StateType::deserialize(&mut a)?,
            ObjectInitialization::Default => {
                self.make_ui_state(dc_object, UiInitialization::Default)
            }
        };
        Ok(Rc::new(RefCell::new(state)))
    }
}

pub struct ObjectWindow {
    object_id: ObjectId,
}

impl ObjectWindow {
    pub fn new_sound_processor(id: SoundProcessorId) -> ObjectWindow {
        ObjectWindow {
            object_id: ObjectId::Sound(id),
        }
    }

    pub fn new_number_source(id: NumberSourceId) -> ObjectWindow {
        ObjectWindow {
            object_id: ObjectId::Number(id),
        }
    }

    pub fn show<F: FnOnce(&mut egui::Ui, &mut GraphUIState)>(
        self,
        ctx: &egui::CtxRef,
        graph_tools: &mut GraphUIState,
        add_contents: F,
    ) {
        let s = match self.object_id {
            ObjectId::Sound(id) => format!("SoundObjectWindow {:?}", id),
            ObjectId::Number(id) => format!("NumberObjectWindow {:?}", id),
        };
        let id = egui::Id::new(s);
        let fill;
        let stroke;
        if graph_tools.object_has_keyboard_focus(self.object_id) {
            fill = egui::Color32::GREEN;
            stroke = egui::Stroke::new(2.0, egui::Color32::YELLOW);
        } else if graph_tools.is_object_selected(self.object_id) {
            fill = egui::Color32::LIGHT_BLUE;
            stroke = egui::Stroke::new(2.0, egui::Color32::YELLOW);
        } else {
            fill = egui::Color32::DARK_BLUE;
            stroke = egui::Stroke::new(2.0, egui::Color32::WHITE);
        }

        let frame = egui::Frame::default()
            .fill(fill)
            .stroke(stroke)
            .margin(egui::Vec2::splat(10.0));

        let mut area = egui::Area::new(id);
        // let layer_id = area.layer();
        if let Some(state) = graph_tools
            .layout_state()
            .get_object_location(self.object_id)
        {
            area = area.current_pos(state.rect.left_top());
        } else if let Some(pos) = ctx.input().pointer.interact_pos() {
            area = area.current_pos(pos);
        }
        area = area.movable(true);
        let r = area.show(ctx, |ui| frame.show(ui, |ui| add_contents(ui, graph_tools)));
        graph_tools.layout_state_mut().track_object_location(
            self.object_id,
            r.response.rect,
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
        if r.response.drag_released() {
            // println!("drag released");
        }
        if r.response.clicked() {
            if !graph_tools.is_object_selected(self.object_id) {
                graph_tools.clear_selection();
                graph_tools.select_object(self.object_id);
            }
        }

        // let r = egui::Window::new("")
        //     .id(id)
        //     .title_bar(false)
        //     .resizable(false)
        //     .frame(
        //         egui::Frame::none()
        //             .fill(fill)
        //             .stroke(stroke)
        //             .margin(egui::Vec2::splat(10.0)),
        //     )
        //     .show(ctx, |ui| add_contents(ui, graph_tools))
        //     .unwrap();
        // if r.response.clicked() {
        //     println!("Yup");
        //     graph_tools.select_object(self.object_id);
        // }
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

fn peg_ui(
    id: GraphId,
    color: egui::Color32,
    label: &str,
    ui_state: &mut GraphUIState,
    ui: &mut egui::Ui,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::Vec2::new(20.0, 20.0), egui::Sense::drag());
    ui_state
        .layout_state_mut()
        .track_peg(id, rect, response.layer_id);
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
        // display_str = format!("{}", id.as_usize());
        display_str = "-".to_string();
        popup_str = None;
        size_diff = -3.0;
    }
    let painter = ui.painter();
    painter.rect(
        rect.expand(size_diff),
        5.0,
        color,
        egui::Stroke::new(2.0, egui::Color32::WHITE),
    );
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        display_str,
        egui::TextStyle::Monospace,
        egui::Color32::WHITE,
    );
    if let Some(s) = popup_str {
        let galley = painter.layout_no_wrap(
            s.to_string(),
            // rect.right_center() + egui::vec2(5.0, 0.0),
            // egui::Align2::LEFT_CENTER,
            egui::TextStyle::Monospace,
            egui::Color32::WHITE,
        );
        let pos = rect.right_center() + egui::vec2(5.0, -0.5 * galley.rect.height());
        painter.rect(
            galley.rect.expand(3.0).translate(pos.to_vec2()),
            3.0,
            // egui::Color32::from_rgba_unmultiplied(0, 0, 0, 64),
            egui::Color32::BLACK,
            egui::Stroke::new(2.0, egui::Color32::WHITE),
        );
        painter.galley(pos, galley);
    }
    if response.clicked() {
        // println!("SoundInputWidget[id={:?}] was clicked", self.sound_input_id);
    }
    if response.drag_started() {
        ui_state.start_dragging(id);
    }
    if response.drag_released() {
        ui_state.stop_dragging(id, response.interact_pointer_pos().unwrap());
    }
    let r = response.clone();
    response.on_hover_ui_at_pointer(|ui| {
        ui.label(label);
    });
    r
}

pub struct SoundInputWidget<'a> {
    sound_input_id: SoundInputId,
    label: &'a str,
    graph_state: &'a mut GraphUIState,
}

impl<'a> SoundInputWidget<'a> {
    pub fn new(
        sound_input_id: SoundInputId,
        label: &'a str,
        graph_state: &'a mut GraphUIState,
    ) -> SoundInputWidget<'a> {
        SoundInputWidget {
            sound_input_id,
            label,
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
            self.graph_state,
            ui,
        )
    }
}

pub struct SoundOutputWidget<'a> {
    sound_processor_id: SoundProcessorId,
    label: &'a str,
    graph_state: &'a mut GraphUIState,
}

impl<'a> SoundOutputWidget<'a> {
    pub fn new(
        sound_processor_id: SoundProcessorId,
        label: &'a str,
        graph_state: &'a mut GraphUIState,
    ) -> SoundOutputWidget<'a> {
        SoundOutputWidget {
            sound_processor_id,
            label,
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
            self.graph_state,
            ui,
        )
    }
}

pub struct NumberInputWidget<'a> {
    number_input_id: NumberInputId,
    label: &'a str,
    graph_state: &'a mut GraphUIState,
}

impl<'a> NumberInputWidget<'a> {
    pub fn new(
        number_input_id: NumberInputId,
        label: &'a str,
        graph_state: &'a mut GraphUIState,
    ) -> NumberInputWidget<'a> {
        NumberInputWidget {
            number_input_id,
            label,
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
            self.graph_state,
            ui,
        )
    }
}

pub struct NumberOutputWidget<'a> {
    number_source_id: NumberSourceId,
    label: &'a str,
    graph_state: &'a mut GraphUIState,
}

impl<'a> NumberOutputWidget<'a> {
    pub fn new(
        number_source_id: NumberSourceId,
        label: &'a str,
        graph_state: &'a mut GraphUIState,
    ) -> NumberOutputWidget<'a> {
        NumberOutputWidget {
            number_source_id,
            label,
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
            self.graph_state,
            ui,
        )
    }
}
