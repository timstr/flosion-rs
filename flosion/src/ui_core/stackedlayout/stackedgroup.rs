use std::{collections::HashMap, ops::BitAnd};

use eframe::{
    egui::{self, UiBuilder},
    emath::TSTransform,
};
use hashstash::{Stash, Stashable, Stasher, UnstashError, Unstashable, Unstasher};

use crate::{
    core::{
        engine::soundenginereport::SoundEngineReport,
        jit::cache::JitCache,
        samplefrequency::SAMPLE_FREQUENCY,
        sound::{
            inputtypes::scheduledinput::{InputTimeSpan, InputTimeSpanId, SoundInputSchedule},
            soundgraph::SoundGraph,
            soundinput::{AnyProcessorInput, SoundInputCategory},
            soundobject::SoundGraphObject,
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

/// A modification to a single time span in a scheduled input
#[derive(Copy, Clone)]
enum TimeSpanEdit {
    Move { delta_samples: isize },
    ChangeBegin { delta_samples: isize },
    ChangeEnd { delta_samples: isize },
    Delete,
}

impl TimeSpanEdit {
    /// Get the corresponding time span (if any) as it would appear after
    /// applying the edit to the schedule
    fn apply(&self, span: InputTimeSpan) -> Option<InputTimeSpan> {
        match self.clone() {
            TimeSpanEdit::Move { delta_samples } => {
                let begin = span.start_sample().saturating_add_signed(delta_samples);
                Some(InputTimeSpan::new(span.id(), begin, span.length_samples()))
            }
            TimeSpanEdit::ChangeBegin { delta_samples } => {
                let begin = span.start_sample().saturating_add_signed(delta_samples);
                let end = span.start_sample() + span.length_samples();
                if begin < end {
                    Some(InputTimeSpan::new(span.id(), begin, end - begin))
                } else {
                    None
                }
            }
            TimeSpanEdit::ChangeEnd { delta_samples } => {
                let begin = span.start_sample();
                let end = (span.start_sample() + span.length_samples())
                    .saturating_add_signed(delta_samples);
                if begin <= end {
                    Some(InputTimeSpan::new(span.id(), begin, end - begin))
                } else {
                    None
                }
            }
            TimeSpanEdit::Delete => None,
        }
    }
}

#[derive(Copy, Clone)]
enum ScheduleChange {
    Edit(usize, TimeSpanEdit),
    Add(InputTimeSpan),
}

#[derive(Clone)]
struct ScheduleChangeSet {
    edits: HashMap<usize, TimeSpanEdit>,
    additions: Vec<InputTimeSpan>,
}
impl ScheduleChangeSet {
    fn new() -> Self {
        Self {
            edits: HashMap::new(),
            additions: Vec::new(),
        }
    }

    fn any_changes(&self) -> bool {
        !self.edits.is_empty() || !self.additions.is_empty()
    }
}

fn resolve_schedule_change(
    schedule: &SoundInputSchedule,
    authored_change: ScheduleChange,
) -> ScheduleChangeSet {
    // naive first pass:
    //  - always keep the authored changes
    //  - remove other time spans if they overlap with authored changes

    let mut resolved_changes = ScheduleChangeSet::new();

    match authored_change {
        ScheduleChange::Edit(i, edit) => {
            resolved_changes.edits.insert(i, edit);
        }
        ScheduleChange::Add(span) => {
            resolved_changes.additions.push(span);
        }
    }

    let authored_span = match authored_change {
        ScheduleChange::Edit(i, edit) => edit.apply(schedule.spans()[i]),
        ScheduleChange::Add(span) => Some(span),
    };

    for (i, span) in schedule.spans().iter().enumerate() {
        if let ScheduleChange::Edit(j, edit) = authored_change {
            if i == j {
                continue;
            }
        }
        if let Some(authored_span) = authored_span {
            if authored_span.intersects_with(*span) {
                resolved_changes.edits.insert(i, TimeSpanEdit::Delete);
            }
        }
    }

    resolved_changes
}

fn apply_schedule_change(schedule: &mut SoundInputSchedule, change: &ScheduleChangeSet) {
    let mut new_spans = Vec::new();

    for (i, span) in schedule.spans().iter().enumerate() {
        if let Some(edit) = change.edits.get(&i) {
            if let Some(edited_span) = edit.apply(*span) {
                new_spans.push(edited_span);
            }
        } else {
            new_spans.push(*span);
        }
    }
    for span in &change.additions {
        new_spans.push(*span);
    }

    schedule.replace_spans(new_spans).unwrap();
}

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
                seconds_per_x_pixel: (StackedLayout::DEFAULT_DURATION as f32)
                    / (StackedLayout::DEFAULT_WIDTH as f32),
            },
            processors,
            origin: bottom_proc_top_left,
        }
    }

    pub(crate) fn time_axis(&self) -> TimeAxis {
        self.time_axis
    }

    pub(crate) fn translate(&mut self, delta: egui::Vec2) {
        self.origin = self.origin + delta;
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
        sound_engine_report: &SoundEngineReport,
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

            let ctx = SoundGraphUiContext::new(
                factories,
                self.time_axis,
                self.width_pixels as f32,
                properties,
                jit_cache,
                stash,
                snapshot_flag,
                sound_engine_report,
            );

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
                            processor_data
                                .with_input_mut(input_loc.input(), |input| {
                                    let top_of_stack = spid == *self.processors.first().unwrap();
                                    self.draw_input_socket(
                                        ui,
                                        ui_state,
                                        &ctx,
                                        input_loc.processor(),
                                        input,
                                        processor_color,
                                        top_of_stack,
                                    );
                                })
                                .unwrap();
                        }
                    }

                    // Draw the sound processor ui
                    let object: &mut dyn SoundGraphObject = processor_data.as_graph_object_mut();
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
        ctx: &SoundGraphUiContext,
        processor_id: SoundProcessorId,
        input: &mut dyn AnyProcessorInput,
        color: egui::Color32,
        top_of_stack: bool,
    ) {
        let socket = InputSocket::from_input_data(processor_id, input);

        if top_of_stack {
            // If the input is at the top of the stack, draw an extra field
            // to hold end of a jumper cable to the target processor, if any
            let (jumper_rect, _) = ui.allocate_exact_size(
                egui::vec2(self.width_pixels as f32, Self::PLUG_HEIGHT),
                egui::Sense::hover(),
            );
            if let Some(target_spid) = input.target() {
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

        let height_factor = match socket.category {
            SoundInputCategory::Scheduled => 2.0,
            _ => 1.0,
        };

        let (rect, response) = ui.allocate_exact_size(
            egui::vec2(
                self.width_pixels as f32,
                Self::SOCKET_HEIGHT * height_factor,
            ),
            egui::Sense::click_and_drag(),
        );

        // TODO: re-enable dragging

        // if response.drag_started() {
        //     ui_state
        //         .interactions_mut()
        //         .start_dragging(DragDropSubject::Socket(socket.location), rect);
        // }

        // if response.dragged() {
        //     ui_state
        //         .interactions_mut()
        //         .continue_drag_move_by(response.drag_delta());
        // }

        // if response.drag_stopped() {
        //     ui_state.interactions_mut().drop_dragging();
        // }

        ui_state.positions_mut().record_socket(socket, rect);

        ui.painter()
            .rect_filled(rect, egui::Rounding::ZERO, color.gamma_multiply(0.5));

        match socket.category {
            SoundInputCategory::Anisochronic => self.draw_uneven_stripes(ui, rect, 1),
            SoundInputCategory::Isochronic => self.draw_even_stripes(ui, rect, 1),
            SoundInputCategory::Branched(n) => self.draw_even_stripes(ui, rect, n),
            SoundInputCategory::Scheduled => self.draw_scheduled_input_socket(ui, ctx, input, rect),
        }
    }

    fn draw_scheduled_input_socket(
        &self,
        ui: &mut egui::Ui,
        ctx: &SoundGraphUiContext,
        input: &mut dyn AnyProcessorInput,
        rect: egui::Rect,
    ) {
        // TODO:
        // [ ] taller rect?
        // [x] get access to schedule
        // [x] draw one box per span
        // [ ] method for adding new spans (click and drag in empty space?)
        // [x] make those spans draggable
        // [ ] prevent spans from overlapping
        // [ ] make ends of spans extensible (horizontal arrows, draggable edges)
        // [ ] pass the various horizontal extents of the input to the next processor
        //     being drawn, draw its background along the same extents

        let mut authored_change: Option<ScheduleChange> = None;
        let schedule = input.schedule_mut().unwrap();

        let bg_response = ui.scope_builder(
            UiBuilder::new().sense(egui::Sense::click_and_drag()),
            |ui| {
                for (i_span, span) in schedule.spans().iter().enumerate() {
                    let left = (span.start_sample() as f32 / SAMPLE_FREQUENCY as f32)
                        / ctx.time_axis().seconds_per_x_pixel;
                    let width = span.length_samples() as f32
                        / SAMPLE_FREQUENCY as f32
                        / ctx.time_axis().seconds_per_x_pixel;

                    let span_rect = egui::Rect::from_x_y_ranges(
                        (rect.left() + left)..=(rect.left() + left + width),
                        rect.top()..=rect.bottom(),
                    );

                    ui.painter().rect_filled(
                        span_rect,
                        egui::Rounding::ZERO,
                        egui::Color32::from_white_alpha(64),
                    );

                    let span_response = ui.interact(
                        span_rect,
                        ui.id().with(span.id()),
                        egui::Sense::click_and_drag(),
                    );

                    if span_response.hovered() {
                        ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::ResizeHorizontal);
                    }

                    if span_response.dragged() {
                        let delta_seconds =
                            span_response.drag_delta().x * ctx.time_axis().seconds_per_x_pixel;
                        let delta_samples =
                            (delta_seconds * SAMPLE_FREQUENCY as f32).round() as isize;

                        authored_change = Some(ScheduleChange::Edit(
                            i_span,
                            TimeSpanEdit::Move { delta_samples },
                        ));
                    }

                    if span_response.drag_stopped() {
                        ctx.request_snapshot();
                    }
                }

                self.draw_bubbled_text("TODO: ???".to_string(), rect.center(), ui);

                // Ensure that the UiBuilder takes up the entire rect, making
                // all of it respond to click + drag events
                ui.advance_cursor_after_rect(rect);
            },
        );
        // ui.interact_bg(egui::Sense::click_and_drag());

        let bg_response = bg_response.response;

        if bg_response.drag_started() {
            ui.memory_mut(|m| {
                m.data.insert_temp(
                    ui.id().with("dragstart"),
                    bg_response.interact_pointer_pos().unwrap().x,
                )
            });
        }

        if bg_response.dragged() {
            if let Some(drag_start) =
                ui.memory(|m| m.data.get_temp::<f32>(ui.id().with("dragstart")))
            {
                let drag_end = bg_response.interact_pointer_pos().unwrap().x;
                let span_rect = egui::Rect::from_x_y_ranges(
                    drag_start.min(drag_end)..=drag_start.max(drag_end),
                    rect.top()..=rect.bottom(),
                );

                ui.painter().rect_filled(
                    span_rect,
                    egui::Rounding::ZERO,
                    egui::Color32::from_white_alpha(64),
                );
            }
        }

        if bg_response.drag_stopped() {
            if let Some(drag_start) =
                ui.memory(|m| m.data.get_temp::<f32>(ui.id().with("dragstart")))
            {
                let drag_end = bg_response.interact_pointer_pos().unwrap().x;
                let start_seconds =
                    (drag_start - rect.left()) * ctx.time_axis().seconds_per_x_pixel;
                let end_seconds = (drag_end - rect.left()) * ctx.time_axis().seconds_per_x_pixel;
                let start_samples = (start_seconds * SAMPLE_FREQUENCY as f32).round() as usize;
                let end_samples = (end_seconds * SAMPLE_FREQUENCY as f32).round() as usize;

                let (start_samples, end_samples) = (
                    start_samples.min(end_samples),
                    start_samples.max(end_samples),
                );

                if start_samples != end_samples {
                    authored_change = Some(ScheduleChange::Add(InputTimeSpan::new(
                        InputTimeSpanId::new_unique(),
                        start_samples,
                        end_samples - start_samples,
                    )));
                }
            }
        }

        if let Some(authored_change) = authored_change {
            let resolved_change = resolve_schedule_change(schedule, authored_change);
            apply_schedule_change(schedule, &resolved_change);
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
        stasher.f32(self.time_axis.seconds_per_x_pixel);
        stasher.array_of_u64_iter(self.processors.iter().map(|p| p.value() as u64));
        stasher.f32(self.origin.x);
        stasher.f32(self.origin.y);
    }
}

impl Unstashable for StackedGroup {
    fn unstash(unstasher: &mut Unstasher) -> Result<StackedGroup, UnstashError> {
        let width_pixels = unstasher.f32()?;
        let time_axis = TimeAxis {
            seconds_per_x_pixel: unstasher.f32()?,
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
