use std::{
    any::{type_name, Any},
    cell::RefCell,
};

use eframe::{
    egui::{self},
    epaint::ecolor::{self},
};
use rand::{thread_rng, Rng};

use crate::core::{
    arguments::{ArgumentList, ParsedArguments},
    graphobject::{GraphObjectHandle, ObjectHandle, ObjectInitialization},
    numberinput::NumberInputId,
    serialization::{Deserializer, Serializable, Serializer},
    soundinput::SoundInputId,
    soundprocessor::SoundProcessorId,
    uniqueid::UniqueId,
};

use super::{
    graph_ui_state::GraphUIState,
    object_ui_states::{AnyObjectUiData, AnyObjectUiState},
    ui_context::UiContext,
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
    pub state: &'a mut T,
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
        ctx: &UiContext,
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
        ctx: &UiContext,
    );

    fn aliases(&self) -> &'static [&'static str];

    fn arguments(&self) -> ArgumentList;

    fn make_ui_state(
        &self,
        object: &GraphObjectHandle,
        init: ObjectInitialization,
    ) -> Result<Box<RefCell<dyn AnyObjectUiState>>, ()>;
}

impl<T: ObjectUi> AnyObjectUi for T {
    fn apply(
        &self,
        object: &GraphObjectHandle,
        object_ui_state: &AnyObjectUiData,
        graph_state: &mut GraphUIState,
        ui: &mut egui::Ui,
        ctx: &UiContext,
    ) {
        let handle = T::HandleType::from_graph_object(object.clone()).unwrap();
        let color = object_ui_state.color();
        let mut state_mut = object_ui_state.state_mut();
        let state_any = state_mut.as_mut_any();
        debug_assert!(
            state_any.is::<T::StateType>(),
            "AnyObjectUi expected to receive state type {}, but got {:?} instead",
            type_name::<T::StateType>(),
            state_mut.get_language_type_name()
        );
        let state = state_any.downcast_mut::<T::StateType>().unwrap();
        let data = ObjectUiData { state, color };
        self.ui(handle, graph_state, ui, ctx, data);
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
    ) -> Result<Box<RefCell<dyn AnyObjectUiState>>, ()> {
        // let dc_object = downcast_object_ref::<T>(object.instance());
        let handle = T::HandleType::from_graph_object(object.clone()).unwrap();
        let state: T::StateType = match init {
            ObjectInitialization::Args(a) => self.make_ui_state(&handle, UiInitialization::Args(a)),
            ObjectInitialization::Archive(mut a) => T::StateType::deserialize(&mut a)?,
            ObjectInitialization::Default => self.make_ui_state(&handle, UiInitialization::Default),
        };
        Ok(Box::new(RefCell::new(state)))
    }
}

// TODO
pub struct NumberSourceUi {}

// TODO
struct ProcessorNumberInputUi {}

// TODO: split into synchronous and non-synchronous
struct SoundInputUi {}

pub struct ProcessorUi {
    processor_id: SoundProcessorId,
    label: &'static str,
    color: egui::Color32,
    number_inputs: Vec<NumberInputId>,
    sound_inputs: Vec<SoundInputId>,
}

impl ProcessorUi {
    pub fn new(id: SoundProcessorId, label: &'static str, color: egui::Color32) -> ProcessorUi {
        ProcessorUi {
            processor_id: id,
            label,
            color,
            number_inputs: Vec::new(),
            sound_inputs: Vec::new(),
        }
    }

    pub fn add_synchronous_sound_input(mut self, input_id: SoundInputId) -> Self {
        self.sound_inputs.push(input_id);
        self
    }

    pub fn show(self, ui: &mut egui::Ui, ctx: &UiContext, graph_tools: &mut GraphUIState) {
        self.show_with(ui, ctx, graph_tools, |_ui, _tools| {});
    }

    pub fn show_with<F: FnOnce(&mut egui::Ui, &mut GraphUIState)>(
        self,
        ui: &mut egui::Ui,
        ctx: &UiContext,
        graph_tools: &mut GraphUIState,
        add_contents: F,
    ) {
        let s = format!("SoundProcessorUi {:?}", self.processor_id);
        let id = egui::Id::new(s);

        let mut area = egui::Area::new(id)
            .movable(true)
            .constrain(false)
            .drag_bounds(egui::Rect::EVERYTHING);

        if ctx.is_top_level() {
            // If the object is top-level, draw it in a new egui::Area,
            // which can be independently clicked and dragged and moved
            // in front of other objects
            if let Some(state) = graph_tools
                .object_positions()
                .get_object_location(self.processor_id.into())
            {
                let pos = state.rect.left_top();
                area = area.current_pos(pos);
            }

            let r = area.show(ui.ctx(), |ui| {
                self.show_with_impl(ui, ctx, graph_tools, add_contents);
            });

            if r.response.drag_started() {
                if !graph_tools.is_object_selected(self.processor_id.into()) {
                    graph_tools.clear_selection();
                    graph_tools.select_object(self.processor_id.into());
                }
            }

            if r.response.dragged() {
                graph_tools.move_selection(r.response.drag_delta(), Some(self.processor_id.into()));
                // Correct for the current object being moved twice
                // graph_tools
                //     .object_positions_mut()
                //     .track_object_location(self.processor_id.into(), r.response.rect);
            }

            if r.response.clicked() {
                if !graph_tools.is_object_selected(self.processor_id.into()) {
                    graph_tools.clear_selection();
                    graph_tools.select_object(self.processor_id.into());
                }
            }
        } else {
            // Otherwise, if the object isn't top-level, nest it within the
            // current egui::Ui
            self.show_with_impl(ui, ctx, graph_tools, add_contents);
        }

        // TODO:
        // if the processor is free-floating (not nested):
        //  - it should be easily draggable
        //  - its temporal axis should be freely adjustable
        // otherwise, if the processor is nested inside the input of another sound processor:
        //  - it should be possible to drag the processor out of the input, thereby detaching
        //    it and turning it into a free-floating processor
        //     -> also, for processors with a single (synchronous?) input, it should be possible
        //        to extract the processor only and have its input take its place
        // in both cases, the processor UI is predominantly a rectangle, whose width is
        // given by the temporal axis. Synchronous sound inputs appear stacked at the top and
        // span the full temporal width. Ignore non-synchronous sound inputs for now.
        // Number inputs are also stacked vertically and (may) span the full width so that later
        // they can be graphed against time if they draw upon a time-dependant input.
        // Once that has been implemented, add rails/nest guards to the left side of each sound
        // processor that precisely reaches around the full vertical extent of its inputs.
    }

    fn show_with_impl<F: FnOnce(&mut egui::Ui, &mut GraphUIState)>(
        &self,
        ui: &mut egui::Ui,
        ctx: &UiContext,
        graph_tools: &mut GraphUIState,
        add_contents: F,
    ) {
        let mut fill = self.color;

        if graph_tools.is_object_selected(self.processor_id.into()) {
            let mut hsva = ecolor::Hsva::from(fill);
            hsva.v = 0.5 * (1.0 + hsva.a);
            fill = hsva.into();
        }

        let outer_frame = egui::Frame::default()
            .fill(egui::Color32::from_rgb(
                (fill.r() as u16 * 3 / 4) as u8,
                (fill.g() as u16 * 3 / 4) as u8,
                (fill.b() as u16 * 3 / 4) as u8,
            ))
            .inner_margin(egui::vec2(0.0, 5.0))
            .stroke(egui::Stroke::new(2.0, egui::Color32::from_black_alpha(128)));

        let input_frame = egui::Frame::default()
            .fill(fill)
            .inner_margin(egui::vec2(0.0, 5.0))
            .stroke(egui::Stroke::new(2.0, egui::Color32::from_black_alpha(128)));

        let content_frame = egui::Frame::default()
            .fill(fill)
            .inner_margin(egui::vec2(0.0, 5.0))
            .stroke(egui::Stroke::new(2.0, egui::Color32::from_black_alpha(128)));

        let r = outer_frame.show(ui, |ui| {
            // Clip to the entire screen, not just outside the area
            // ui.set_clip_rect(ctx.egui_context().input(|i| i.screen_rect()));

            let desired_width = ctx.width() as f32;
            ui.set_width(desired_width);

            if !self.sound_inputs.is_empty() {
                input_frame.show(ui, |ui| {
                    ui.vertical(|ui| {
                        ui.set_width(desired_width);
                        for input_id in &self.sound_inputs {
                            let input_data = ctx.topology().sound_input(*input_id).unwrap();

                            // TODO: check sychronous vs non-synchronous

                            let target = input_data.target();

                            match target {
                                Some(spid) => {
                                    // TODO: draw the processor right above
                                    let target_processor =
                                        ctx.topology().sound_processor(spid).unwrap();
                                    let target_graph_object =
                                        target_processor.instance_arc().as_graph_object();

                                    graph_tools
                                        .object_positions_mut()
                                        .track_object_location(spid.into(), ui.cursor());

                                    let inner_ctx = ctx.nest();

                                    ctx.ui_factory().ui(
                                        &target_graph_object,
                                        graph_tools,
                                        ui,
                                        &inner_ctx,
                                    );
                                }
                                None => {
                                    // TODO: draw an empty field onto which things can be dragged
                                    ui.label(format!("Sound Input {} (empty)", input_id.value()));
                                }
                            }
                        }
                    });
                });
            }

            content_frame.show(ui, |ui| {
                let ir = ui.vertical(|ui| {
                    ui.set_width(desired_width);
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new(self.label)
                                .color(egui::Color32::BLACK)
                                .strong(),
                        )
                        .wrap(false),
                    );
                    add_contents(ui, graph_tools);
                });
                let content_width = ir.response.rect.width();
                if content_width < desired_width {
                    ui.allocate_space(egui::vec2(desired_width - content_width, 0.0));
                }
            });
        });

        graph_tools
            .object_positions_mut()
            .track_object_location(self.processor_id.into(), r.response.rect);
    }
}
