use std::ops::BitAnd;

use eframe::egui::{self};
use hashstash::{Stashable, Stasher};

use crate::{
    core::{
        jit::cache::JitCache,
        sound::{
            soundgraph::SoundGraph, soundinput::InputOptions, soundobject::SoundGraphObject,
            soundprocessor::SoundProcessorId,
        },
    },
    ui_core::{
        flosion_ui::Factories, graph_properties::GraphProperties,
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

    /// The on-screen location of the stack, or None if it has
    /// not been drawn yet.
    rect: egui::Rect,
}

impl StackedGroup {
    const SOCKET_HEIGHT: f32 = 10.0;
    const PLUG_HEIGHT: f32 = 10.0;
    const TAB_HEIGHT: f32 = 20.0;
    const TAB_WIDTH: f32 = 50.0;
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

        // Find the position of the first processor in the group.
        // if not found, just put rect at 0,0
        let first_processor_top_left = positions
            .find_processor(*processors.first().unwrap())
            .map_or(egui::Pos2::ZERO, |pp| pp.rect.left_top());

        // Experimentally determine the offset between the top-left
        // of the highest processor and the top-left of the stacked
        // group by finding the minimum among the positions
        let group_to_first_processor_offset = positions
            .processors()
            .iter()
            .map(|p| p.rect.left_top() - p.group_origin)
            // NOTE: using reduce() because min() and friends do not support f32 >:(
            .reduce(|smallest, v| if v.y < smallest.y { v } else { smallest })
            .unwrap_or(egui::Vec2::ZERO);

        // Move the rect to the position of the first processor
        // minus that offset.
        let group_origin = first_processor_top_left - group_to_first_processor_offset;
        let rect = egui::Rect::from_min_size(group_origin, egui::Vec2::ZERO);

        StackedGroup {
            width_pixels: StackedLayout::DEFAULT_WIDTH,
            time_axis: TimeAxis {
                time_per_x_pixel: (StackedLayout::DEFAULT_DURATION as f32)
                    / (StackedLayout::DEFAULT_WIDTH as f32),
            },
            processors,
            rect,
        }
    }

    pub(crate) fn time_axis(&self) -> TimeAxis {
        self.time_axis
    }

    pub(crate) fn rect(&self) -> egui::Rect {
        self.rect
    }

    pub(crate) fn set_rect(&mut self, rect: egui::Rect) {
        self.rect = rect;
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
        properties: &GraphProperties,
    ) {
        // For a unique id for egui, hash the processor ids in the group
        let area_id = egui::Id::new(&self.processors);

        let area = egui::Area::new(area_id)
            .constrain(false)
            .movable(false)
            .fixed_pos(self.rect.left_top());

        let r = area.show(ui.ctx(), |ui| {
            let group_origin = ui.cursor().left_top();

            let frame = egui::Frame::default()
                .stroke(egui::Stroke::new(1.0, egui::Color32::from_white_alpha(128)))
                .rounding(10.0)
                .inner_margin(5.0);
            frame.show(ui, |ui| {
                ui.horizontal(|ui| {
                    // Allocate some width for the sidebar (full height is not known until
                    // the after the processors are laid out)
                    let (initial_sidebar_rect, _) =
                        ui.allocate_exact_size(egui::vec2(15.0, 15.0), egui::Sense::hover());

                    let processors_response = ui.vertical(|ui| {
                        // Tighten the spacing
                        ui.spacing_mut().item_spacing.y = 0.0;

                        let mut top_of_stack = true;

                        for spid in &self.processors {
                            let processor_data = graph.sound_processor_mut(*spid).unwrap();

                            let inputs = processor_data.input_locations();

                            let processor_color =
                                ui_state.object_states().get_object_color(spid.into());

                            if inputs.is_empty() {
                                self.draw_barrier(ui);
                            } else {
                                for input_loc in inputs {
                                    let (input_socket, target) = processor_data
                                        .with_input(input_loc.input(), |input| {
                                            (
                                                InputSocket::from_input_data(
                                                    input_loc.processor(),
                                                    input,
                                                ),
                                                input.target(),
                                            )
                                        })
                                        .unwrap();
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

                            top_of_stack = false;

                            let object: &mut dyn SoundGraphObject =
                                processor_data.as_graph_object_mut();
                            let ctx = SoundGraphUiContext::new(
                                factories,
                                self.time_axis,
                                self.width_pixels as f32,
                                group_origin,
                                properties,
                                jit_cache,
                            );

                            show_sound_object_ui(factories.sound_uis(), object, ui_state, ui, &ctx);

                            let processor_data = graph.sound_processor(*spid).unwrap();
                            self.draw_processor_plug(
                                ui,
                                ui_state,
                                ProcessorPlug::from_processor_data(processor_data),
                                processor_color,
                            );
                        }
                    });

                    let combined_height = processors_response.response.rect.height();

                    // Left sidebar, for dragging the entire group
                    let sidebar_rect = initial_sidebar_rect
                        .with_max_y(initial_sidebar_rect.min.y + combined_height);

                    let sidebar_response = ui
                        .interact(
                            sidebar_rect,
                            ui.id().with("sidebar"),
                            egui::Sense::click_and_drag(),
                        )
                        .on_hover_and_drag_cursor(egui::CursorIcon::Grab);

                    if sidebar_response.drag_started() {
                        ui_state.interactions_mut().start_dragging(
                            DragDropSubject::Group {
                                top_processor: self.processors.first().unwrap().clone(),
                            },
                            self.rect,
                        );
                    }

                    if sidebar_response.dragged() {
                        self.rect = self.rect.translate(sidebar_response.drag_delta());
                        ui_state.interactions_mut().continue_drag_move_to(self.rect);
                    }

                    if sidebar_response.drag_stopped() {
                        ui_state.interactions_mut().drop_dragging();
                    }

                    ui.painter().rect_filled(
                        sidebar_rect,
                        egui::Rounding::same(5.0),
                        egui::Color32::from_white_alpha(32),
                    );

                    let dot_spacing = 15.0;
                    let dot_radius = 4.0;

                    let dot_x = sidebar_rect.center().x;
                    let top_dot_y = sidebar_rect.top() + 0.5 * sidebar_rect.width();
                    let bottom_dot_y = sidebar_rect.bottom() - 0.5 * sidebar_rect.width();

                    let num_dots = ((bottom_dot_y - top_dot_y) / dot_spacing).floor() as usize;

                    for i in 0..=num_dots {
                        let dot_y = top_dot_y + (i as f32) * dot_spacing;

                        ui.painter().circle_filled(
                            egui::pos2(dot_x, dot_y),
                            dot_radius,
                            egui::Color32::from_black_alpha(64),
                        );
                    }

                    // Right sidebar, for widening and slimming the group
                    let (resize_rect, resize_response) = ui.allocate_exact_size(
                        egui::vec2(5.0, combined_height),
                        egui::Sense::click_and_drag(),
                    );

                    let resize_response =
                        resize_response.on_hover_and_drag_cursor(egui::CursorIcon::ResizeColumn);

                    if resize_response.dragged() {
                        // TODO: what should the minimum width be? What should happen when
                        // UI things have no room?
                        self.width_pixels =
                            (self.width_pixels + resize_response.drag_delta().x).max(50.0);
                    }

                    ui.painter().rect_filled(
                        resize_rect,
                        egui::Rounding::same(5.0),
                        egui::Color32::from_white_alpha(32),
                    );
                });
            });
        });

        self.rect
            .set_right(self.rect.left() + r.response.rect.width());
        self.rect
            .set_bottom(self.rect.top() + r.response.rect.height());
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

        // Add a tab sticking up, biased to the left
        let screen_rect = ui.input(|i| i.screen_rect());
        let tab_left = (bar_rect.center().x - Self::TAB_WIDTH)
            .clamp(screen_rect.left(), screen_rect.right() - Self::TAB_WIDTH)
            .clamp(bar_rect.left(), bar_rect.right() - Self::TAB_WIDTH);

        let tab_rect = egui::Rect::from_x_y_ranges(
            tab_left..=(tab_left + Self::TAB_WIDTH),
            (bar_rect.bottom() - Self::TAB_HEIGHT)..=bar_rect.bottom(),
        );

        let bevel = 5.0;
        ui.painter().add(egui::Shape::convex_polygon(
            vec![
                tab_rect.left_bottom(),
                tab_rect.left_top() + egui::vec2(0.0, bevel),
                tab_rect.left_top() + egui::vec2(bevel, 0.0),
                tab_rect.right_top() + egui::vec2(-bevel, 0.0),
                tab_rect.right_top() + egui::vec2(0.0, bevel),
                tab_rect.right_bottom(),
            ],
            color,
            egui::Stroke::NONE,
        ));

        let tab_response = ui.interact(
            tab_rect,
            ui.id().with("socket").with(socket.location),
            egui::Sense::click_and_drag(),
        );

        let response = bar_response.union(tab_response);

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

        ui_state
            .positions_mut()
            .record_socket(socket, bar_rect, tab_rect);

        ui.painter()
            .rect_filled(bar_rect, egui::Rounding::ZERO, color.gamma_multiply(0.5));

        match socket.options {
            InputOptions::Synchronous => self.draw_even_stripes(ui, bar_rect, socket.branches),
            InputOptions::NonSynchronous => self.draw_uneven_stripes(ui, bar_rect, socket.branches),
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

        // Add a tab sticking down, biased to the right
        // TODO: this gets drawn over top of by any input sockets below.
        // Consider drawing tabs in a separate order or removing them entirely.
        let screen_rect = ui.input(|i| i.screen_rect());
        let tab_left = bar_rect
            .center()
            .x
            .clamp(screen_rect.left() + Self::TAB_WIDTH, screen_rect.right())
            .clamp(bar_rect.left() + Self::TAB_WIDTH, bar_rect.right());

        let tab_rect = egui::Rect::from_x_y_ranges(
            tab_left..=(tab_left + Self::TAB_WIDTH),
            bar_rect.top()..=(bar_rect.top() + Self::TAB_HEIGHT),
        );

        let bevel = 5.0;
        ui.painter().add(egui::Shape::convex_polygon(
            vec![
                tab_rect.left_top(),
                tab_rect.right_top(),
                tab_rect.right_bottom() + egui::vec2(0.0, -bevel),
                tab_rect.right_bottom() + egui::vec2(-bevel, 0.0),
                tab_rect.left_bottom() + egui::vec2(bevel, 0.0),
                tab_rect.left_bottom() + egui::vec2(0.0, -bevel),
            ],
            color,
            egui::Stroke::NONE,
        ));

        let tab_response = ui.interact(
            tab_rect,
            ui.id().with("plug").with(plug.processor),
            egui::Sense::click_and_drag(),
        );

        let response = bar_response.union(tab_response);

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

        ui_state
            .positions_mut()
            .record_plug(plug, bar_rect, tab_rect);

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
        stasher.u32(self.width_pixels.to_bits());
        stasher.u32(self.time_axis.time_per_x_pixel.to_bits());
        stasher.array_of_u64_iter(self.processors.iter().map(|p| p.value() as u64));
        stasher.u32(self.rect.left().to_bits());
        stasher.u32(self.rect.right().to_bits());
        stasher.u32(self.rect.top().to_bits());
        stasher.u32(self.rect.bottom().to_bits());
    }
}
