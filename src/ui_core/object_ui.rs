use std::any::type_name;

use eframe::egui;

use crate::core::{
    graphobject::{GraphId, GraphObject, ObjectId, TypedGraphObject},
    numberinput::NumberInputId,
    numbersource::NumberSourceId,
    soundinput::SoundInputId,
    soundprocessor::SoundProcessorId,
};

use super::{
    arguments::{ArgumentList, ParsedArguments},
    graph_ui_state::GraphUIState,
};

pub trait ObjectUi: 'static + Default {
    type ObjectType: TypedGraphObject;
    fn ui(
        &self,
        id: ObjectId,
        object: &Self::ObjectType,
        graph_state: &mut GraphUIState,
        ui: &mut egui::Ui,
    );

    fn aliases(&self) -> &'static [&'static str] {
        &[]
    }

    fn arguments(&self) -> ArgumentList {
        ArgumentList::new()
    }

    fn init_object(&self, object: &Self::ObjectType, args: ParsedArguments) {}
}

pub trait AnyObjectUi {
    fn apply(
        &self,
        id: ObjectId,
        object: &dyn GraphObject,
        graph_state: &mut GraphUIState,
        ui: &mut egui::Ui,
    );

    fn aliases(&self) -> &'static [&'static str];

    fn arguments(&self) -> ArgumentList;

    fn init_object(&self, object: &dyn GraphObject, args: ParsedArguments);
}

impl<T: ObjectUi> AnyObjectUi for T {
    fn apply(
        &self,
        id: ObjectId,
        object: &dyn GraphObject,
        graph_state: &mut GraphUIState,
        ui: &mut egui::Ui,
    ) {
        let any = object.as_any();
        debug_assert!(
            any.is::<T::ObjectType>(),
            "AnyObjectUi expected to receive type {}, but got {} instead",
            type_name::<T::ObjectType>(),
            object.get_language_type_name()
        );
        let dc_object = any.downcast_ref::<T::ObjectType>().unwrap();
        self.ui(id, dc_object, graph_state, ui);
    }

    fn init_object(&self, object: &dyn GraphObject, args: ParsedArguments) {
        let any = object.as_any();
        debug_assert!(
            any.is::<T::ObjectType>(),
            "AnyObjectUi expected to receive type {}, but got {} instead",
            type_name::<T::ObjectType>(),
            object.get_language_type_name()
        );
        let dc_object = any.downcast_ref::<T::ObjectType>().unwrap();
        self.init_object(dc_object, args);
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

    pub fn show<F: FnOnce(&mut egui::Ui)>(self, ctx: &egui::CtxRef, add_contents: F) {
        let s = match self.object_id {
            ObjectId::Sound(id) => format!("SoundObjectWindow {:?}", id),
            ObjectId::Number(id) => format!("NumberObjectWindow {:?}", id),
        };
        let id = egui::Id::new(s);
        egui::Window::new("")
            .id(id)
            .title_bar(false)
            .resizable(false)
            .frame(
                egui::Frame::none()
                    .fill(egui::Color32::DARK_BLUE)
                    .stroke(egui::Stroke::new(2.0, egui::Color32::WHITE))
                    .margin(egui::Vec2::splat(10.0)),
            )
            .show(ctx, add_contents)
            .unwrap();
    }
}

fn peg_ui(
    id: GraphId,
    color: egui::Color32,
    graph_state: &mut GraphUIState,
    ui: &mut egui::Ui,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::Vec2::new(20.0, 20.0), egui::Sense::drag());
    graph_state.track_peg(id, rect, response.layer_id);
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
    response
}

pub struct SoundInputWidget<'a> {
    sound_input_id: SoundInputId,
    graph_state: &'a mut GraphUIState,
}

impl<'a> SoundInputWidget<'a> {
    pub fn new(
        sound_input_id: SoundInputId,
        graph_state: &'a mut GraphUIState,
    ) -> SoundInputWidget<'a> {
        SoundInputWidget {
            sound_input_id,
            graph_state,
        }
    }
}

impl<'a> egui::Widget for SoundInputWidget<'a> {
    fn ui(self, ui: &mut egui::Ui) -> eframe::egui::Response {
        peg_ui(
            self.sound_input_id.into(),
            egui::Color32::from_rgb(0, 255, 0),
            self.graph_state,
            ui,
        )
    }
}

pub struct SoundOutputWidget<'a> {
    sound_processor_id: SoundProcessorId,
    graph_state: &'a mut GraphUIState,
}

impl<'a> SoundOutputWidget<'a> {
    pub fn new(
        sound_processor_id: SoundProcessorId,
        graph_state: &'a mut GraphUIState,
    ) -> SoundOutputWidget<'a> {
        SoundOutputWidget {
            sound_processor_id,
            graph_state,
        }
    }
}

impl<'a> egui::Widget for SoundOutputWidget<'a> {
    fn ui(self, ui: &mut egui::Ui) -> eframe::egui::Response {
        peg_ui(
            self.sound_processor_id.into(),
            egui::Color32::from_rgb(0, 128, 0),
            self.graph_state,
            ui,
        )
    }
}

pub struct NumberInputWidget<'a> {
    number_input_id: NumberInputId,
    graph_state: &'a mut GraphUIState,
}

impl<'a> NumberInputWidget<'a> {
    pub fn new(
        number_input_id: NumberInputId,
        graph_state: &mut GraphUIState,
    ) -> NumberInputWidget {
        NumberInputWidget {
            number_input_id,
            graph_state,
        }
    }
}

impl<'a> egui::Widget for NumberInputWidget<'a> {
    fn ui(self, ui: &mut egui::Ui) -> eframe::egui::Response {
        peg_ui(
            self.number_input_id.into(),
            egui::Color32::from_rgb(0, 0, 255),
            self.graph_state,
            ui,
        )
    }
}

pub struct NumberOutputWidget<'a> {
    number_source_id: NumberSourceId,
    graph_state: &'a mut GraphUIState,
}

impl<'a> NumberOutputWidget<'a> {
    pub fn new(
        number_source_id: NumberSourceId,
        graph_state: &'a mut GraphUIState,
    ) -> NumberOutputWidget {
        NumberOutputWidget {
            number_source_id,
            graph_state,
        }
    }
}

impl<'a> egui::Widget for NumberOutputWidget<'a> {
    fn ui(self, ui: &mut egui::Ui) -> eframe::egui::Response {
        peg_ui(
            self.number_source_id.into(),
            egui::Color32::from_rgb(0, 0, 128),
            self.graph_state,
            ui,
        )
    }
}
