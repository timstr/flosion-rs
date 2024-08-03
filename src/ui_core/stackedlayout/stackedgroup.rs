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
    interconnect::{InterconnectInput, ProcessorInterconnect},
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
    const INTERCONNECT_HEIGHT: f32 = 10.0;

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
                // .fill(egui::Color32::from_gray(64))
                .stroke(egui::Stroke::new(1.0, egui::Color32::from_white_alpha(128)))
                .rounding(10.0)
                .inner_margin(10.0);
            frame.show(ui, |ui| {
                ui.horizontal(|ui| {
                    // Allocate some width for the sidebar (full height is not known until
                    // the after the processors are laid out)
                    let (initial_sidebar_rect, _) =
                        ui.allocate_exact_size(egui::vec2(30.0, 30.0), egui::Sense::hover());

                    let processors_response = ui.vertical(|ui| {
                        let top_processor = self.processors[0];
                        let top_inputs: Vec<InterconnectInput> = graph
                            .topology()
                            .sound_processor(top_processor)
                            .unwrap()
                            .sound_inputs()
                            .iter()
                            .map(|siid| {
                                InterconnectInput::from_input_data(
                                    graph.topology().sound_input(*siid).unwrap(),
                                )
                            })
                            .collect();

                        if top_inputs.len() == 0 {
                            let (rect, _) = ui.allocate_exact_size(
                                egui::vec2(self.width_pixels as f32, Self::INTERCONNECT_HEIGHT),
                                egui::Sense::hover(),
                            );
                            self.draw_barrier(ui, rect);
                        } else {
                            for input in top_inputs {
                                // let (rect, _) = ui.allocate_exact_size(
                                //     egui::vec2(self.width_pixels as f32, 5.0),
                                //     egui::Sense::hover(),
                                // );
                                // ui.painter().rect_filled(
                                //     rect,
                                //     egui::Rounding::ZERO,
                                //     egui::Color32::WHITE,
                                // );
                                self.draw_processor_interconnect(
                                    ui,
                                    ui_state,
                                    ProcessorInterconnect::TopOfStack(top_processor, input),
                                );
                            }
                        }

                        for i in 0..self.processors.len() {
                            let spid = self.processors[i];

                            let object = graph
                                .topology()
                                .sound_processor(spid)
                                .unwrap()
                                .instance_arc()
                                .as_graph_object();
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

                            if let Some(next_spid) = self.processors.get(i + 1) {
                                let next_inputs = graph
                                    .topology()
                                    .sound_processor(*next_spid)
                                    .unwrap()
                                    .sound_inputs();
                                debug_assert_eq!(next_inputs.len(), 1);
                                let siid = next_inputs[0];
                                let input = graph.topology().sound_input(siid).unwrap();
                                let interconnect = ProcessorInterconnect::BetweenTwoProcessors {
                                    bottom: *next_spid,
                                    top: spid,
                                    input: InterconnectInput::from_input_data(input),
                                };
                                self.draw_processor_interconnect(ui, ui_state, interconnect);
                            }
                        }

                        self.draw_processor_interconnect(
                            ui,
                            ui_state,
                            ProcessorInterconnect::BottomOfStack(*self.processors.last().unwrap()),
                        );
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

    fn draw_processor_interconnect(
        &self,
        ui: &mut egui::Ui,
        ui_state: &mut SoundGraphUiState,
        interconnect: ProcessorInterconnect,
    ) {
        // TODO: make clickable to e.g. spawn summon widget, insert new (matching) processor
        let (rect, _) = ui.allocate_exact_size(
            egui::vec2(self.width_pixels as f32, Self::INTERCONNECT_HEIGHT),
            egui::Sense::hover(),
        );

        ui_state
            .positions_mut()
            .record_interconnect(interconnect, rect);

        ui_state.record_interconnect(interconnect);

        if let Some(legal_interconnects) = ui_state.interactions().legal_processors_to_drop_onto() {
            if legal_interconnects.contains(&interconnect) {
                // If the interconnect is legal to drop a processor onto, highlight it
                ui.painter().rect_filled(
                    rect,
                    egui::Rounding::same(5.0),
                    egui::Color32::from_white_alpha(64),
                );
            } else if !interconnect
                .includes_processor(ui_state.interactions().processor_being_dragged().unwrap())
            {
                // Otherwise, if the interconnect isn't immediately next to the processor,
                // colour it red to show that the processor can't be dropped there
                ui.painter().rect_filled(
                    rect,
                    egui::Rounding::same(5.0),
                    egui::Color32::from_rgba_unmultiplied(255, 0, 0, 64),
                );
            }
        }

        match interconnect {
            ProcessorInterconnect::TopOfStack(_, input) => {
                self.draw_stripes(ui, rect, input.branches, input.options);
            }
            ProcessorInterconnect::BetweenTwoProcessors {
                bottom: _,
                top: _,
                input,
            } => {
                self.draw_stripes(ui, rect, input.branches, input.options);
            }
            ProcessorInterconnect::BottomOfStack(_) => self.draw_even_stripes(ui, rect, 1),
        }
    }

    fn draw_stripes(
        &self,
        ui: &mut egui::Ui,
        rect: egui::Rect,
        num_branches: usize,
        options: InputOptions,
    ) {
        match options {
            InputOptions::Synchronous => self.draw_even_stripes(ui, rect, num_branches),
            InputOptions::NonSynchronous => self.draw_uneven_stripes(ui, rect, num_branches),
        }
    }

    fn draw_even_stripes(&self, ui: &mut egui::Ui, rect: egui::Rect, num_branches: usize) {
        let old_clip_rect = ui.clip_rect();

        // clip rendered things to the allocated area to gracefully
        // overflow contents. This needs to be undone below.
        ui.set_clip_rect(rect);

        let stripe_width = 3.0;
        let stripe_spacing = 10.0;

        let stripe_total_width = stripe_spacing + stripe_width;

        let num_stripes = (self.width_pixels as f32 / stripe_total_width as f32).ceil() as usize;

        for i in 0..num_stripes {
            let xmin = rect.min.x + (i as f32) * stripe_total_width;
            let ymin = rect.min.y;
            let ymax = rect.max.y;

            let top_left = egui::pos2(xmin, ymin);
            let bottom_left = egui::pos2(xmin, ymax);

            self.draw_single_stripe(
                ui.painter(),
                top_left,
                bottom_left,
                stripe_width,
                num_branches,
            );
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

        let stripe_width = 3.0;
        let stripe_spacing = 10.0;

        let stripe_total_width = stripe_spacing + stripe_width;

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
            self.draw_single_stripe(
                ui.painter(),
                top_left,
                bottom_left,
                stripe_width,
                num_branches,
            );
        }

        // Restore the previous clip rect
        ui.set_clip_rect(old_clip_rect);

        // Write branch amount
        if num_branches != 1 {
            self.draw_bubbled_text(format!("×{}", num_branches), rect.center(), ui);
        }
    }

    fn draw_barrier(&self, ui: &mut egui::Ui, rect: egui::Rect) {
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
        width: f32,
        num_branches: usize,
    ) {
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
