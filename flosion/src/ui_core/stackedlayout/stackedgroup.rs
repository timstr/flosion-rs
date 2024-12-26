use std::ops::BitAnd;

use eframe::{
    egui::{self},
    emath::TSTransform,
};
use hashstash::{Stash, Stashable, Stasher, UnstashError, Unstashable, Unstasher};

use crate::{
    core::{
        jit::cache::JitCache,
        sound::{
            soundgraph::SoundGraph, soundinput::Chronicity, soundobject::SoundGraphObject,
            soundprocessor::SoundProcessorId,
        },
    },
    ui_core::{
        factories::Factories, graph_properties::GraphProperties, history::SnapshotFlag,
        interactions::draganddrop::DragDropSubject, soundgraphuicontext::SoundGraphUiContext,
        soundgraphuistate::SoundGraphUiState, soundobjectpositions::SoundObjectPositions,
        soundobjectui::show_sound_object_ui, stackedlayout::stackedlayout::StackedLayout,
    },
};

use super::{
    interconnect::{InputSocket, ProcessorPlug},
    timeaxis::TimeAxis,
};

/// The visual representation of a sequence of sound processors,
/// connected end-to-end in a linear fashion. Each processor in
/// the group must have exactly one sound input, with the exception
/// of the top/leaf processor, which may have any number.
pub struct StackedGroup {
    width_pixels: f32,
    time_axis: TimeAxis,

    /// The processors in the stacked group, ordered with the
    /// deepest dependency first. The root/bottom processor is
    /// thus the last in the vec.
    processors: Vec<SoundProcessorId>,

    /// The top-left corner of the bottom sound processor
    origin: egui::Pos2,
}

impl StackedGroup {
    const SOCKET_HEIGHT: f32 = 10.0;
    const PLUG_HEIGHT: f32 = 10.0;
    const STRIPE_WIDTH: f32 = 3.0;
    const STRIPE_SPACING: f32 = 10.0;

    /// Creates a new stacked group consisting of the given list of sound processors and
    /// positions them according to the previously-known positions of thos processors.
    /// This is intended to minimize movement of processors on-screen when processors
    /// are added and removed from groups.
    pub(crate) fn new_at_top_processor(
        processors: Vec<SoundProcessorId>,
        positions: &SoundObjectPositions,
    ) -> StackedGroup {
        assert!(processors.len() > 0);

        // Find the position of the bottom processor in the group.
        // if not found, just put rect at 0,0
        let bottom_proc_top_left = positions
            .find_processor(*processors.last().unwrap())
            .map_or(egui::Pos2::ZERO, |pp| pp.body_rect.left_top());

        StackedGroup {
            width_pixels: StackedLayout::DEFAULT_WIDTH,
            time_axis: TimeAxis {
                time_per_x_pixel: (StackedLayout::DEFAULT_DURATION as f32)
                    / (StackedLayout::DEFAULT_WIDTH as f32),
            },
            processors,
            origin: bottom_proc_top_left,
        }
    }

    pub(crate) fn time_axis(&self) -> TimeAxis {
        self.time_axis
    }

    pub(crate) fn origin(&self) -> egui::Pos2 {
        self.origin
    }

    pub(crate) fn set_origin(&mut self, origin: egui::Pos2) {
        self.origin = origin
    }

    pub(crate) fn processors(&self) -> &[SoundProcessorId] {
        &self.processors
    }

    pub(crate) fn processor_is_at_top(&self, processor: SoundProcessorId) -> bool {
        self.processors.first() == Some(&processor)
    }

    pub(crate) fn insert_processor_at_bottom(&mut self, processor: SoundProcessorId) {
        self.processors.push(processor);
    }

    pub(crate) fn insert_processor_above(
        &mut self,
        processor: SoundProcessorId,
        other_processor: SoundProcessorId,
    ) {
        let i = self
            .processors
            .iter()
            .position(|p| *p == other_processor)
            .unwrap();
        self.processors.insert(i, processor);
    }

    pub(crate) fn insert_processor_below(
        &mut self,
        processor: SoundProcessorId,
        other_processor: SoundProcessorId,
    ) {
        let i = self
            .processors
            .iter()
            .position(|p| *p == other_processor)
            .unwrap();
        self.processors.insert(i + 1, processor);
    }

    pub(crate) fn remove_processor(&mut self, processor: SoundProcessorId) {
        self.processors.retain(|i| *i != processor);
    }

    pub(crate) fn split_off_processor_and_everything_below(
        &mut self,
        processor: SoundProcessorId,
    ) -> Vec<SoundProcessorId> {
        let i = self
            .processors
            .iter()
            .position(|p| *p == processor)
            .unwrap();

        self.processors.split_off(i)
    }

    pub(crate) fn split_off_everything_below_processor(
        &mut self,
        processor: SoundProcessorId,
    ) -> Vec<SoundProcessorId> {
        let i = self
            .processors
            .iter()
            .position(|p| *p == processor)
            .unwrap();

        self.processors.split_off(i + 1)
    }

    pub(crate) fn remove_dangling_processor_ids(&mut self, graph: &SoundGraph) {
        self.processors.retain(|i| graph.contains(i));
    }

    pub(crate) fn draw(
        &mut self,
        ui: &mut egui::Ui,
        factories: &Factories,
        ui_state: &mut SoundGraphUiState,
        graph: &mut SoundGraph,
        jit_cache: &JitCache,
        stash: &Stash,
        properties: &GraphProperties,
        snapshot_flag: &SnapshotFlag,
    ) {
        let mut bottom_left_of_next_proc: Option<egui::Pos2> = None;

        for (i, spid) in self.processors.iter().cloned().rev().enumerate() {
            // Start a new ui in a new layer from the top-left corner at the estimated origin
            let layer_id = egui::LayerId::new(egui::Order::Middle, ui.id().with(spid));
            let estimated_top_left = ui_state
                .positions()
                .find_processor(spid)
                .map(|pp| pp.outer_rect.left_top())
                .unwrap_or(egui::pos2(0.0, 0.0));
            let ui_builder = egui::UiBuilder::new().max_rect(egui::Rect::from_x_y_ranges(
                estimated_top_left.x..,
                estimated_top_left.y..,
            ));

            let mut body_rect = None;

            let outer_res = ui.allocate_new_ui(ui_builder, |ui| {
                ui.with_layer_id(layer_id, |ui| {
                    // Tighten the spacing
                    ui.spacing_mut().item_spacing.y = 0.0;

                    let processor_data = graph.sound_processor_mut(spid).unwrap();

                    let processor_color = ui_state.object_states().get_object_color(spid.into());

                    // Draw input sockets
                    let inputs = processor_data.input_locations();
                    if inputs.is_empty() {
                        self.draw_barrier(ui);
                    } else {
                        for input_loc in inputs {
                            let (input_socket, target) = processor_data
                                .with_input(input_loc.input(), |input| {
                                    (
                                        InputSocket::from_input_data(input_loc.processor(), input),
                                        input.target(),
                                    )
                                })
                                .unwrap();
                            let top_of_stack = spid == *self.processors.first().unwrap();
                            self.draw_input_socket(
                                ui,
                                ui_state,
                                target,
                                input_socket,
                                processor_color,
                                top_of_stack,
                            );
                        }
                    }

                    // Draw the sound processor ui
                    let object: &mut dyn SoundGraphObject = processor_data.as_graph_object_mut();
                    let ctx = SoundGraphUiContext::new(
                        factories,
                        self.time_axis,
                        self.width_pixels as f32,
                        properties,
                        jit_cache,
                        stash,
                        snapshot_flag,
                    );
                    let body_res = ui.vertical(|ui| {
                        show_sound_object_ui(factories.sound_uis(), object, ui_state, ui, &ctx);
                    });
                    body_rect = Some(body_res.response.rect);

                    // Draw the final processor's output plug
                    let plug = ProcessorPlug::from_processor_data(processor_data);
                    if i == 0 {
                        self.draw_processor_plug(ui, ui_state, plug, processor_color);
                    } else {
                        ui_state.positions_mut().clear_plug(plug);
                    }
                });
            });
            let proc_size = outer_res.response.rect.size();

            let exact_top_left = if let Some(pos) = bottom_left_of_next_proc {
                egui::pos2(pos.x, pos.y - proc_size.y)
            } else {
                self.origin
            };

            // Translate the processor ui to be above the previous processor
            let translation = exact_top_left - estimated_top_left;
            ui.ctx()
                .transform_layer_shapes(layer_id, TSTransform::from_translation(translation));

            bottom_left_of_next_proc = Some(exact_top_left);

            let body_rect = body_rect.unwrap();
            let outer_rect = outer_res.response.rect.translate(translation);

            ui_state
                .positions_mut()
                .record_processor(spid, body_rect, outer_rect);
        }
    }

    fn draw_input_socket(
        &self,
        ui: &mut egui::Ui,
        ui_state: &mut SoundGraphUiState,
        target: Option<SoundProcessorId>,
        socket: InputSocket,
        color: egui::Color32,
        top_of_stack: bool,
    ) {
        if top_of_stack {
            // If the input is at the top of the stack, draw an extra field
            // to hold end of a jumper cable to the target processor, if any
            let (jumper_rect, _) = ui.allocate_exact_size(
                egui::vec2(self.width_pixels as f32, Self::PLUG_HEIGHT),
                egui::Sense::hover(),
            );
            if let Some(target_spid) = target {
                let jumper_color = ui_state
                    .object_states()
                    .get_object_color(target_spid.into());
                ui.painter()
                    .rect_filled(jumper_rect, egui::Rounding::ZERO, jumper_color);
            }
            ui_state
                .positions_mut()
                .record_socket_jumper(socket.location, jumper_rect);
        }

        let (bar_rect, bar_response) = ui.allocate_exact_size(
            egui::vec2(self.width_pixels as f32, Self::SOCKET_HEIGHT),
            egui::Sense::click_and_drag(),
        );

        let response = bar_response;

        if response.drag_started() {
            ui_state
                .interactions_mut()
                .start_dragging(DragDropSubject::Socket(socket.location), bar_rect);
        }

        if response.dragged() {
            ui_state
                .interactions_mut()
                .continue_drag_move_by(response.drag_delta());
        }

        if response.drag_stopped() {
            ui_state.interactions_mut().drop_dragging();
        }

        ui_state.positions_mut().record_socket(socket, bar_rect);

        ui.painter()
            .rect_filled(bar_rect, egui::Rounding::ZERO, color.gamma_multiply(0.5));

        match socket.chronicity {
            Chronicity::Iso => self.draw_even_stripes(ui, bar_rect, socket.branches),
            Chronicity::Aniso => self.draw_uneven_stripes(ui, bar_rect, socket.branches),
        }
    }

    fn draw_processor_plug(
        &self,
        ui: &mut egui::Ui,
        ui_state: &mut SoundGraphUiState,
        plug: ProcessorPlug,
        color: egui::Color32,
    ) {
        let (bar_rect, bar_response) = ui.allocate_exact_size(
            egui::vec2(self.width_pixels as f32, Self::PLUG_HEIGHT),
            egui::Sense::click_and_drag(),
        );

        let response = bar_response;

        if response.drag_started() {
            ui_state
                .interactions_mut()
                .start_dragging(DragDropSubject::Plug(plug.processor), bar_rect);
        }

        if response.dragged() {
            ui_state
                .interactions_mut()
                .continue_drag_move_by(response.drag_delta());
        }

        if response.drag_stopped() {
            ui_state.interactions_mut().drop_dragging();
        }

        ui_state.positions_mut().record_plug(plug, bar_rect);

        ui.painter()
            .rect_filled(bar_rect, egui::Rounding::ZERO, color.gamma_multiply(0.5));

        if plug.is_static {
            self.draw_even_stripes(ui, bar_rect, 1);
        } else {
            let y_middle = bar_rect.center().y;
            let half_dot_height = Self::STRIPE_WIDTH * 0.5;
            self.draw_even_stripes(
                ui,
                egui::Rect::from_x_y_ranges(
                    bar_rect.left()..=bar_rect.right(),
                    (y_middle - half_dot_height)..=(y_middle + half_dot_height),
                ),
                1,
            );
        }
    }

    fn draw_even_stripes(&self, ui: &mut egui::Ui, rect: egui::Rect, num_branches: usize) {
        let old_clip_rect = ui.clip_rect();

        // clip rendered things to the allocated area to gracefully
        // overflow contents. This needs to be undone below.
        ui.set_clip_rect(rect);

        let stripe_total_width = Self::STRIPE_SPACING + Self::STRIPE_WIDTH;

        let num_stripes = (self.width_pixels as f32 / stripe_total_width as f32).ceil() as usize;

        for i in 0..num_stripes {
            let xmin = rect.min.x + (i as f32) * stripe_total_width;
            let ymin = rect.min.y;
            let ymax = rect.max.y;

            let top_left = egui::pos2(xmin, ymin);
            let bottom_left = egui::pos2(xmin, ymax);

            self.draw_single_stripe(ui.painter(), top_left, bottom_left, num_branches);
        }

        // Restore the previous clip rect
        ui.set_clip_rect(old_clip_rect);

        // Write branch amount
        if num_branches != 1 {
            self.draw_bubbled_text(format!("×{}", num_branches), rect.left_center(), ui);
        }
    }

    fn draw_uneven_stripes(&self, ui: &mut egui::Ui, rect: egui::Rect, num_branches: usize) {
        let old_clip_rect = ui.clip_rect();

        // clip rendered things to the allocated area to gracefully
        // overflow contents. This needs to be undone below.
        ui.set_clip_rect(rect);

        let stripe_total_width = Self::STRIPE_SPACING + Self::STRIPE_WIDTH;

        let num_stripes = (self.width_pixels as f32 / stripe_total_width as f32).ceil() as usize;

        for i in 0..num_stripes {
            let xmin = rect.min.x + (i as f32) * stripe_total_width;
            let ymin = rect.min.y;
            let ymax = rect.max.y;

            let wonkiness = match i.bitand(3) {
                0 => 0.0,
                1 => 1.0,
                2 => 0.0,
                3 => -1.0,
                _ => unreachable!(),
            };

            let wonkiness = stripe_total_width * 0.25 * wonkiness;

            let top_left = egui::pos2(xmin + wonkiness, ymin);
            let bottom_left = egui::pos2(xmin, ymax);
            self.draw_single_stripe(ui.painter(), top_left, bottom_left, num_branches);
        }

        // Restore the previous clip rect
        ui.set_clip_rect(old_clip_rect);

        // Write branch amount
        if num_branches != 1 {
            self.draw_bubbled_text(format!("×{}", num_branches), rect.left_center(), ui);
        }
    }

    fn draw_barrier(&self, ui: &mut egui::Ui) {
        let (rect, _) = ui.allocate_exact_size(
            egui::vec2(self.width_pixels as f32, Self::SOCKET_HEIGHT),
            egui::Sense::hover(),
        );

        let old_clip_rect = ui.clip_rect();

        // clip rendered things to the allocated area to gracefully
        // overflow contents. This needs to be undone below.
        ui.set_clip_rect(rect);

        let stripe_width = 50.0;
        let stripe_spacing = 10.0;

        let stripe_total_width = stripe_spacing + stripe_width;

        let num_stripes =
            (self.width_pixels as f32 / stripe_total_width as f32).ceil() as usize + 1;

        for i in 0..num_stripes {
            let xmin = rect.min.x + (i as f32) * stripe_total_width;
            let xmax = xmin + stripe_width;
            let ymin = rect.min.y;
            let ymax = rect.max.y;
            let ymiddle = 0.5 * (ymin + ymax);
            ui.painter().rect_filled(
                egui::Rect::from_min_max(egui::pos2(xmin, ymin), egui::pos2(xmax, ymiddle - 2.0)),
                egui::Rounding::ZERO,
                egui::Color32::from_white_alpha(16),
            );
            ui.painter().rect_filled(
                egui::Rect::from_min_max(
                    egui::pos2(xmin - 0.5 * stripe_total_width, ymiddle + 2.0),
                    egui::pos2(xmax - 0.5 * stripe_total_width, ymax),
                ),
                egui::Rounding::ZERO,
                egui::Color32::from_white_alpha(16),
            );
        }

        // Restore the previous clip rect
        ui.set_clip_rect(old_clip_rect);
    }

    fn draw_single_stripe(
        &self,
        painter: &egui::Painter,
        top_left: egui::Pos2,
        bottom_left: egui::Pos2,
        num_branches: usize,
    ) {
        let width = Self::STRIPE_WIDTH;
        match num_branches {
            0 => {
                // no branches: taper to a point at top middle
                let points = vec![
                    egui::pos2(top_left.x + 0.5 * width, top_left.y),
                    egui::pos2(bottom_left.x + width, bottom_left.y),
                    bottom_left,
                ];

                painter.add(egui::Shape::convex_polygon(
                    points,
                    egui::Color32::from_white_alpha(32),
                    egui::Stroke::NONE,
                ));
            }
            1 => {
                // 1 branch: basic single parallelogram
                let points = vec![
                    top_left,
                    egui::pos2(top_left.x + width, top_left.y),
                    egui::pos2(bottom_left.x + width, bottom_left.y),
                    bottom_left,
                ];

                painter.add(egui::Shape::convex_polygon(
                    points,
                    egui::Color32::from_white_alpha(32),
                    egui::Stroke::NONE,
                ));
            }
            _ => {
                // 2 or more branches: draw a trapezoid
                let splay = width * 2.0; // hmmm
                let points = vec![
                    egui::pos2(top_left.x - 0.5 * splay, top_left.y),
                    egui::pos2(top_left.x + width + 0.5 * splay, top_left.y),
                    egui::pos2(bottom_left.x + width, bottom_left.y),
                    egui::pos2(bottom_left.x, bottom_left.y),
                ];
                painter.add(egui::Shape::convex_polygon(
                    points,
                    egui::Color32::from_white_alpha(32),
                    egui::Stroke::NONE,
                ));
            }
        }
    }

    fn draw_bubbled_text(&self, text: String, position: egui::Pos2, ui: &mut egui::Ui) {
        let galley = ui
            .fonts(|f| f.layout_no_wrap(text, egui::FontId::monospace(10.0), egui::Color32::WHITE));
        let rect = galley
            .rect
            .translate(position.to_vec2() - egui::vec2(0.0, galley.rect.height()));
        ui.painter().rect_filled(
            rect.expand(3.0),
            egui::Rounding::same(3.0),
            egui::Color32::from_black_alpha(128),
        );
        ui.painter()
            .galley(rect.left_top(), galley, egui::Color32::WHITE);
    }
}

impl Stashable for StackedGroup {
    fn stash(&self, stasher: &mut Stasher) {
        stasher.f32(self.width_pixels);
        stasher.f32(self.time_axis.time_per_x_pixel);
        stasher.array_of_u64_iter(self.processors.iter().map(|p| p.value() as u64));
        stasher.f32(self.origin.x);
        stasher.f32(self.origin.y);
    }
}

impl Unstashable for StackedGroup {
    fn unstash(unstasher: &mut Unstasher) -> Result<StackedGroup, UnstashError> {
        let width_pixels = unstasher.f32()?;
        let time_axis = TimeAxis {
            time_per_x_pixel: unstasher.f32()?,
        };

        let processors = unstasher
            .array_of_u64_iter()?
            .map(|i| SoundProcessorId::new(i as _))
            .collect();

        let origin = egui::pos2(unstasher.f32()?, unstasher.f32()?);

        Ok(StackedGroup {
            width_pixels,
            time_axis,
            processors,
            origin,
        })
    }
}
