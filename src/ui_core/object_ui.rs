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
    soundinput::{InputOptions, SoundInputId},
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

pub struct ProcessorUi {
    processor_id: SoundProcessorId,
    label: &'static str,
    color: egui::Color32,
    number_inputs: Vec<NumberInputId>,
    sound_inputs: Vec<SoundInputId>,
}

#[derive(Clone, Copy)]
struct ProcessorUiProps {
    origin: egui::Pos2,
    indentation: f32,
    fill: egui::Color32,
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

    pub fn add_sound_input(mut self, input_id: SoundInputId) -> Self {
        self.sound_inputs.push(input_id);
        self
    }

    pub fn add_number_input(mut self, input_id: NumberInputId) -> Self {
        self.number_inputs.push(input_id);
        self
    }

    const RAIL_WIDTH: f32 = 15.0;

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
        if ctx.is_top_level() {
            // If the object is top-level, draw it in a new egui::Area,
            // which can be independently clicked and dragged and moved
            // in front of other objects

            let s = format!("SoundProcessorUi {:?}", self.processor_id);
            let id = egui::Id::new(s);

            let mut area = egui::Area::new(id)
                .movable(true)
                .constrain(false)
                .drag_bounds(egui::Rect::EVERYTHING);

            // TODO: offset position according to cumulative rail thickness

            if let Some(state) = graph_tools
                .object_positions()
                .get_object_location(self.processor_id.into())
            {
                let pos = state.rect.left_top();
                area = area.current_pos(pos);
            }

            let r = area.show(ui.ctx(), |ui| {
                self.show_with_impl(ui, ctx, graph_tools, add_contents)
            });

            let r = r.response.union(r.inner);

            if r.drag_started() {
                if !graph_tools.is_object_selected(self.processor_id.into()) {
                    graph_tools.clear_selection();
                    graph_tools.select_object(self.processor_id.into());
                }
            }

            if r.dragged() {
                graph_tools.move_selection(r.drag_delta(), Some(self.processor_id.into()));
            }

            if r.clicked() {
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
    }

    fn show_with_impl<F: FnOnce(&mut egui::Ui, &mut GraphUIState)>(
        &self,
        ui: &mut egui::Ui,
        ctx: &UiContext,
        graph_tools: &mut GraphUIState,
        add_contents: F,
    ) -> egui::Response {
        // Clip to the entire screen, not just outside the area
        ui.set_clip_rect(ui.ctx().input(|i| i.screen_rect()));

        let mut fill = self.color;

        let selected = graph_tools.is_object_selected(self.processor_id.into());

        if selected {
            let mut hsva = ecolor::Hsva::from(fill);
            hsva.v = 0.5 * (1.0 + hsva.a);
            fill = hsva.into();
        }

        let darkish_stroke = egui::Stroke::new(2.0, egui::Color32::from_black_alpha(128));
        let bright_yellow_stroke = egui::Stroke::new(2.0, egui::Color32::YELLOW);

        let outer_frame = egui::Frame::default()
            .fill(egui::Color32::from_rgb(
                (fill.r() as u16 * 3 / 4) as u8,
                (fill.g() as u16 * 3 / 4) as u8,
                (fill.b() as u16 * 3 / 4) as u8,
            ))
            .inner_margin(egui::vec2(0.0, 5.0))
            .stroke(darkish_stroke);

        let content_frame = egui::Frame::default()
            .fill(fill)
            .inner_margin(egui::vec2(0.0, 5.0))
            .stroke(if selected {
                bright_yellow_stroke
            } else {
                darkish_stroke
            });

        let props = ProcessorUiProps {
            origin: ui.cursor().left_top(),
            indentation: (ctx.nesting_depth() + 1) as f32 * Self::RAIL_WIDTH,
            fill,
        };

        let left_of_body = props.origin.x + props.indentation;

        let desired_width = ctx.width();

        let r = outer_frame.show(ui, |ui| {
            if !self.sound_inputs.is_empty() {
                ui.set_width(desired_width);
                for input_id in &self.sound_inputs {
                    self.show_sound_input(ui, ctx, *input_id, graph_tools, props);
                }
            }

            let body_rect = egui::Rect::from_x_y_ranges(
                left_of_body..=(left_of_body + desired_width),
                ui.cursor().top()..=f32::INFINITY,
            );

            ui.allocate_ui_at_rect(body_rect, |ui| {
                content_frame.show(ui, |ui| {
                    ui.vertical(|ui| {
                        for input_id in &self.number_inputs {
                            self.show_number_input(ui, ctx, *input_id, graph_tools, props);
                        }
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
                });
            });

            let bottom_of_body = ui.cursor().top();

            let top_rail_rect = egui::Rect::from_x_y_ranges(
                props.origin.x..=(props.origin.x + Self::RAIL_WIDTH - 2.0),
                props.origin.y..=bottom_of_body,
            );

            let rounding = egui::Rounding::same(3.0);

            ui.painter().rect_filled(top_rail_rect, rounding, fill);
            ui.painter()
                .rect_stroke(top_rail_rect, rounding, darkish_stroke)
        });

        graph_tools
            .object_positions_mut()
            .track_object_location(self.processor_id.into(), r.response.rect);

        // r.response.union(r.inner)
        r.response
    }

    fn show_sound_input(
        &self,
        ui: &mut egui::Ui,
        ctx: &UiContext,
        input_id: SoundInputId,
        graph_tools: &mut GraphUIState,
        mut props: ProcessorUiProps,
    ) {
        let input_data = ctx.topology().sound_input(input_id).unwrap();

        let opts = input_data.options();

        let nonsync_shim_width = Self::RAIL_WIDTH * 0.5;

        let original_origin = props.origin;
        let mut desired_width = ctx.width();

        if let InputOptions::NonSynchronous = opts {
            props.origin.x += nonsync_shim_width;
            desired_width -= nonsync_shim_width;
        }

        let left_of_body = props.origin.x + props.indentation;

        let desired_width = desired_width;

        let top_of_input = ui.cursor().top();

        let input_frame = egui::Frame::default()
            .fill(egui::Color32::from_black_alpha(64))
            .inner_margin(egui::vec2(0.0, 5.0))
            .stroke(egui::Stroke::new(2.0, egui::Color32::from_black_alpha(128)));

        let target = input_data.target();
        match target {
            Some(spid) => {
                // draw the processor right above
                let target_processor = ctx.topology().sound_processor(spid).unwrap();
                let target_graph_object = target_processor.instance_arc().as_graph_object();

                let inner_ctx = ctx.nest(desired_width);

                // move the inner UI one rail's width to the right to account for
                // the lesser nesting level and to let the nested object ui find
                // the correct horizontal extent again
                let inner_objectui_rect = egui::Rect::from_x_y_ranges(
                    (props.origin.x + Self::RAIL_WIDTH)..=f32::INFINITY,
                    ui.cursor().top()..=f32::INFINITY,
                );

                ui.allocate_ui_at_rect(inner_objectui_rect, |ui| {
                    ctx.ui_factory()
                        .ui(&target_graph_object, graph_tools, ui, &inner_ctx);
                });
            }
            None => {
                // move the inner UI exactly to the desired horizontal extent,
                // past all rails, where it actually needs to get drawn
                let input_rect = egui::Rect::from_x_y_ranges(
                    left_of_body..=(left_of_body + desired_width),
                    ui.cursor().top()..=f32::INFINITY,
                );

                ui.allocate_ui_at_rect(input_rect, |ui| {
                    // TODO: draw an empty field onto which things can be dragged
                    input_frame.show(ui, |ui| {
                        ui.set_width(desired_width);
                        let label_str = format!("Sound Input {} (empty)", input_id.value());
                        ui.add(
                            egui::Label::new(
                                egui::RichText::new(label_str)
                                    .color(egui::Color32::BLACK)
                                    .strong(),
                            )
                            .wrap(false),
                        );
                    });
                });
            }
        }

        let bottom_of_input = ui.cursor().top();

        if let InputOptions::NonSynchronous = opts {
            let left_of_shim = original_origin.x + Self::RAIL_WIDTH;
            let nonsync_shim_rect = egui::Rect::from_x_y_ranges(
                left_of_shim..=(left_of_shim + nonsync_shim_width),
                top_of_input..=bottom_of_input,
            );
            ui.painter().rect_filled(
                nonsync_shim_rect,
                egui::Rounding::none(),
                egui::Color32::from_black_alpha(64),
            );
        }
    }

    fn show_number_input(
        &self,
        ui: &mut egui::Ui,
        ctx: &UiContext,
        input_id: NumberInputId,
        graph_tools: &mut GraphUIState,
        props: ProcessorUiProps,
    ) {
        let fill = egui::Color32::from_black_alpha(64);

        let input_frame = egui::Frame::default()
            .fill(fill)
            .inner_margin(egui::vec2(0.0, 5.0))
            .stroke(egui::Stroke::new(2.0, egui::Color32::from_black_alpha(128)));

        input_frame.show(ui, |ui| {
            ui.set_width(ctx.width());
            let label_str = format!("Number Input {}", input_id.value());
            ui.add(
                egui::Label::new(
                    egui::RichText::new(label_str)
                        .color(egui::Color32::BLACK)
                        .strong(),
                )
                .wrap(false),
            );
        });
    }
}
