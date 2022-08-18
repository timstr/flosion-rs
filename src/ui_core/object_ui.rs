use std::{
    any::{type_name, Any},
    cell::RefCell,
    rc::Rc,
};

use eframe::egui;

use crate::core::{
    graphobject::{GraphId, GraphObject, ObjectId, TypedGraphObject},
    numberinput::NumberInputId,
    numbersource::NumberSourceId,
    serialization::{Deserializer, Serializable, Serializer},
    soundinput::SoundInputId,
    soundprocessor::SoundProcessorId,
};

use super::{
    arguments::{ArgumentList, ParsedArguments},
    graph_ui_state::{GraphUIState, ObjectUiState},
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

    fn init_from_args(&self, _object: &Self::WrapperType, _args: &ParsedArguments) {}

    fn init_from_archive(&self, _object: &Self::WrapperType, _archive: &mut Deserializer) {}

    fn serialize_object(&self, _object: &Self::WrapperType, _serializer: &mut Serializer) {}

    fn make_ui_state(&self, _args: &ParsedArguments) -> Self::StateType {
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

    fn init_object_from_args(&self, object: &dyn GraphObject, args: &ParsedArguments);

    fn init_object_from_archive(&self, object: &dyn GraphObject, deserializer: &mut Deserializer);

    fn serialize_object(&self, object: &dyn GraphObject, serializer: &mut Serializer);

    fn make_ui_state(&self, args: &ParsedArguments) -> Rc<RefCell<dyn ObjectUiState>>;
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

    fn init_object_from_args(&self, object: &dyn GraphObject, args: &ParsedArguments) {
        self.init_from_args(downcast_object::<T>(object), args);
    }

    fn init_object_from_archive(&self, object: &dyn GraphObject, deserializer: &mut Deserializer) {
        self.init_from_archive(downcast_object::<T>(object), deserializer);
    }

    fn serialize_object(&self, _object: &dyn GraphObject, _serializer: &mut Serializer) {
        todo!();
    }

    fn make_ui_state(&self, args: &ParsedArguments) -> Rc<RefCell<dyn ObjectUiState>> {
        let x: &T = self;
        Rc::new(RefCell::new(x.make_ui_state(args)))
    }

    fn aliases(&self) -> &'static [&'static str] {
        self.aliases()
    }

    fn arguments(&self) -> ArgumentList {
        self.arguments()
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
        if graph_tools.is_object_selected(self.object_id) {
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
        } else {
            area = area.current_pos(ctx.input().pointer.interact_pos().unwrap());
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

fn peg_ui(
    id: GraphId,
    color: egui::Color32,
    label: &str,
    graph_state: &mut GraphUIState,
    ui: &mut egui::Ui,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::Vec2::new(20.0, 20.0), egui::Sense::drag());
    graph_state
        .layout_state_mut()
        .track_peg(id, rect, response.layer_id);
    let painter = ui.painter();
    painter.rect(
        rect,
        5.0,
        color,
        egui::Stroke::new(2.0, egui::Color32::WHITE),
    );
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        format!("{}", id.inner_value()),
        egui::TextStyle::Monospace,
        egui::Color32::WHITE,
    );
    if response.clicked() {
        // println!("SoundInputWidget[id={:?}] was clicked", self.sound_input_id);
    }
    if response.drag_started() {
        graph_state.start_dragging(id);
    }
    if response.drag_released() {
        graph_state.stop_dragging(id, response.interact_pointer_pos().unwrap());
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
