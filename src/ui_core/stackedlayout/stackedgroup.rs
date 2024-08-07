use std::{
    collections::{HashMap, HashSet},
    ops::BitAnd,
};

use eframe::egui::{self};

use crate::{
    core::sound::{
        expression::SoundExpressionId, expressionargument::SoundExpressionArgumentId,
        soundgraph::SoundGraph, soundgraphtopology::SoundGraphTopology, soundinput::InputOptions,
        soundprocessor::SoundProcessorId,
    },
    ui_core::{
        flosion_ui::Factories, soundgraphuicontext::SoundGraphUiContext,
        soundgraphuistate::SoundGraphUiState, soundobjectpositions::SoundObjectPositions,
        stackedlayout::stackedlayout::SoundGraphLayout,
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
    width_pixels: usize,
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
            width_pixels: SoundGraphLayout::DEFAULT_WIDTH,
            time_axis: TimeAxis {
                time_per_x_pixel: (SoundGraphLayout::DEFAULT_DURATION as f32)
                    / (SoundGraphLayout::DEFAULT_WIDTH as f32),
            },
            processors,
            rect,
        }
    }

    pub fn rect(&self) -> egui::Rect {
        self.rect
    }

    pub fn set_rect(&mut self, rect: egui::Rect) {
        self.rect = rect;
    }

    pub fn processors(&self) -> &[SoundProcessorId] {
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

    pub(crate) fn remove_dangling_processor_ids(&mut self, topo: &SoundGraphTopology) {
        self.processors.retain(|i| topo.contains(i));
    }

    pub(crate) fn draw(
        &mut self,
        ui: &mut egui::Ui,
        factories: &Factories,
        ui_state: &mut SoundGraphUiState,
        graph: &mut SoundGraph,
        available_arguments: &HashMap<SoundExpressionId, HashSet<SoundExpressionArgumentId>>,
    ) {
        // For a unique id for egui, hash the processor ids in the group
        let area_id = egui::Id::new(&self.processors);

        let area = egui::Area::new(area_id)
            .constrain(false)
            .movable(true)
            .current_pos(self.rect.left_top());

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
                        ui.allocate_exact_size(egui::vec2(30.0, 30.0), egui::Sense::hover());

                    let processors_response = ui.vertical(|ui| {
                        // Tighten the spacing
                        ui.spacing_mut().item_spacing.y = 0.0;

                        for spid in &self.processors {
                            let processor_data = graph.topology().sound_processor(*spid).unwrap();

                            let inputs = processor_data.sound_inputs();

                            let processor_color =
                                ui_state.object_states().get_object_color(spid.into());

                            if inputs.is_empty() {
                                self.draw_barrier(ui);
                            } else {
                                for input_id in inputs {
                                    let input_data =
                                        graph.topology().sound_input(*input_id).unwrap();
                                    self.draw_input_socket(
                                        ui,
                                        ui_state,
                                        InputSocket::from_input_data(input_data),
                                        processor_color,
                                    );
                                }
                            }

                            let object = processor_data.instance_arc().as_graph_object();
                            let mut ctx = SoundGraphUiContext::new(
                                factories,
                                self.time_axis,
                                self.width_pixels as f32,
                                group_origin,
                                available_arguments,
                            );
                            factories
                                .sound_uis()
                                .ui(&object, ui_state, ui, &mut ctx, graph);

                            let processor_data = graph.topology().sound_processor(*spid).unwrap();
                            self.draw_processor_plug(
                                ui,
                                ui_state,
                                ProcessorPlug::from_processor_data(processor_data),
                                processor_color,
                            );
                        }
                    });

                    let sidebar_rect =
                        initial_sidebar_rect.with_max_y(processors_response.response.rect.max.y);

                    ui.painter().rect_filled(
                        sidebar_rect,
                        egui::Rounding::same(5.0),
                        egui::Color32::from_white_alpha(32),
                    );
                });
            });
        });

        self.rect = r.response.rect;
    }

    fn draw_input_socket(
        &self,
        ui: &mut egui::Ui,
        ui_state: &mut SoundGraphUiState,
        socket: InputSocket,
        color: egui::Color32,
    ) {
        let (rect, _) = ui.allocate_exact_size(
            egui::vec2(self.width_pixels as f32, Self::SOCKET_HEIGHT),
            egui::Sense::hover(),
        );

        ui_state.positions_mut().record_socket(socket, rect);

        // Draw a background gradient fading from transparent
        // to the processor's colour
        {
            let mut mesh = egui::Mesh::default();

            mesh.colored_vertex(rect.left_top(), egui::Color32::TRANSPARENT);
            mesh.colored_vertex(rect.right_top(), egui::Color32::TRANSPARENT);
            mesh.colored_vertex(rect.left_bottom(), color);
            mesh.colored_vertex(rect.right_bottom(), color);

            mesh.add_triangle(0, 1, 2);
            mesh.add_triangle(1, 3, 2);

            ui.painter().add(mesh);
        }

        if let Some(sockets) = ui_state
            .interactions()
            .legal_sockets_to_drop_processor_onto()
        {
            let legal = sockets.contains(&socket);
            ui.painter().rect_filled(
                rect,
                egui::Rounding::same(5.0),
                if legal {
                    egui::Color32::from_white_alpha(64)
                } else {
                    egui::Color32::from_rgba_unmultiplied(255, 0, 0, 64)
                },
            );
        } else {
        }

        match socket.options {
            InputOptions::Synchronous => self.draw_even_stripes(ui, rect, socket.branches),
            InputOptions::NonSynchronous => self.draw_uneven_stripes(ui, rect, socket.branches),
        }
    }

    fn draw_processor_plug(
        &self,
        ui: &mut egui::Ui,
        ui_state: &mut SoundGraphUiState,
        plug: ProcessorPlug,
        color: egui::Color32,
    ) {
        let (rect, _) = ui.allocate_exact_size(
            egui::vec2(self.width_pixels as f32, Self::PLUG_HEIGHT),
            egui::Sense::hover(),
        );

        ui_state.positions_mut().record_plug(plug, rect);

        // Draw a background gradient fading from the processor's colour
        // to transparent
        {
            let mut mesh = egui::Mesh::default();

            mesh.colored_vertex(rect.left_top(), color);
            mesh.colored_vertex(rect.right_top(), color);
            mesh.colored_vertex(rect.left_bottom(), egui::Color32::TRANSPARENT);
            mesh.colored_vertex(rect.right_bottom(), egui::Color32::TRANSPARENT);

            mesh.add_triangle(0, 1, 2);
            mesh.add_triangle(1, 3, 2);

            ui.painter().add(mesh);
        }

        // TODO: highlight if dragging something compatible

        if plug.is_static {
            self.draw_even_stripes(ui, rect, 1);
        } else {
            let y_middle = rect.center().y;
            let half_dot_height = Self::STRIPE_WIDTH * 0.5;
            self.draw_even_stripes(
                ui,
                egui::Rect::from_x_y_ranges(
                    rect.left()..=rect.right(),
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
            self.draw_bubbled_text(format!("×{}", num_branches), rect.center(), ui);
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
            self.draw_bubbled_text(format!("×{}", num_branches), rect.center(), ui);
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
            .translate(position.to_vec2() - 0.5 * galley.rect.size());
        ui.painter().rect_filled(
            rect.expand(3.0),
            egui::Rounding::same(3.0),
            egui::Color32::from_black_alpha(128),
        );
        ui.painter()
            .galley(rect.left_top(), galley, egui::Color32::WHITE);
    }
}
