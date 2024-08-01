use std::{
    collections::{HashMap, HashSet},
    hash::Hasher,
    ops::BitAnd,
};

use eframe::egui;

use crate::core::{
    revision::revision::{Revisable, RevisionHash},
    sound::{
        expression::SoundExpressionId,
        expressionargument::SoundExpressionArgumentId,
        soundgraph::SoundGraph,
        soundgraphdata::SoundInputData,
        soundgraphtopology::SoundGraphTopology,
        soundinput::{InputOptions, SoundInputId},
        soundprocessor::SoundProcessorId,
    },
};

use super::{
    flosion_ui::Factories, soundgraphuicontext::SoundGraphUiContext,
    soundgraphuistate::SoundGraphUiState, soundobjectpositions::SoundObjectPositions,
};

/// A mapping between a portion of the sound processing timeline
/// and a spatial region on screen.
#[derive(Clone, Copy)]
pub struct TimeAxis {
    /// How many seconds each horizontal pixel corresponds to
    pub time_per_x_pixel: f32,
    // TODO: offset to allow scrolling?
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct InterconnectInput {
    pub id: SoundInputId,
    pub options: InputOptions,
    pub branches: usize,
}

impl InterconnectInput {
    pub(crate) fn from_input_data(data: &SoundInputData) -> InterconnectInput {
        InterconnectInput {
            id: data.id(),
            options: data.options(),
            branches: data.branches().len(),
        }
    }
}

impl Revisable for InterconnectInput {
    fn get_revision(&self) -> RevisionHash {
        let mut hasher = seahash::SeaHasher::new();
        hasher.write_u64(self.id.get_revision().value());
        hasher.write_u8(match self.options {
            InputOptions::Synchronous => 0,
            InputOptions::NonSynchronous => 1,
        });
        hasher.write_usize(self.branches);
        RevisionHash::new(hasher.finish())
    }
}

/// Describes the spaces around and between sound processors in a stacked
/// group, in terms of which processors and which sound input meet at
/// the region of space.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProcessorInterconnect {
    /// The interconnect at the top of a stack and on above the top
    /// sound processor. Note that a processor with no inputs does
    /// not correspond to any interconnects, and similarly, a processor
    /// with multiple inputs will have many interconnects.
    TopOfStack(SoundProcessorId, InterconnectInput),

    /// The interconnect between two sound processors in the interior
    /// of a stacked group, where the bottom processor has exactly one
    /// sound input which is connected to the top sound processor, and
    /// by virtue of the stacked group, the top processor is not connected
    /// to any other inputs.
    BetweenTwoProcessors {
        bottom: SoundProcessorId,
        top: SoundProcessorId,
        input: InterconnectInput,
    },

    /// The space below the lowest sound processor in the stacked group.
    BottomOfStack(SoundProcessorId),
}

impl ProcessorInterconnect {
    pub(crate) fn processor_above(&self) -> Option<SoundProcessorId> {
        match self {
            ProcessorInterconnect::TopOfStack(_, _) => None,
            ProcessorInterconnect::BetweenTwoProcessors {
                bottom: _,
                top,
                input: _,
            } => Some(*top),
            ProcessorInterconnect::BottomOfStack(i) => Some(*i),
        }
    }

    pub(crate) fn processor_below(&self) -> Option<SoundProcessorId> {
        match self {
            ProcessorInterconnect::TopOfStack(i, _) => Some(*i),
            ProcessorInterconnect::BetweenTwoProcessors {
                bottom,
                top: _,
                input: _,
            } => Some(*bottom),
            ProcessorInterconnect::BottomOfStack(_) => None,
        }
    }

    pub(crate) fn unique_input(&self) -> Option<InterconnectInput> {
        match self {
            ProcessorInterconnect::TopOfStack(_, i) => Some(*i),
            ProcessorInterconnect::BetweenTwoProcessors {
                bottom: _,
                top: _,
                input,
            } => Some(*input),
            ProcessorInterconnect::BottomOfStack(_) => None,
        }
    }
}

impl Revisable for ProcessorInterconnect {
    fn get_revision(&self) -> RevisionHash {
        let mut hasher = seahash::SeaHasher::new();
        match self {
            ProcessorInterconnect::TopOfStack(spid, input) => {
                hasher.write_u8(0);
                hasher.write_u64(spid.get_revision().value());
                hasher.write_u64(input.get_revision().value());
            }
            ProcessorInterconnect::BetweenTwoProcessors { bottom, top, input } => {
                hasher.write_u8(1);
                hasher.write_u64(bottom.get_revision().value());
                hasher.write_u64(top.get_revision().value());
                hasher.write_u64(input.get_revision().value());
            }
            ProcessorInterconnect::BottomOfStack(spid) => {
                hasher.write_u8(2);
                hasher.write_u64(spid.get_revision().value());
            }
        }
        RevisionHash::new(hasher.finish())
    }
}

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
                ui.painter().rect_filled(
                    rect,
                    egui::Rounding::same(5.0),
                    egui::Color32::from_white_alpha(64),
                );
            } else {
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

/// Visual layout of all processor groups and the connections between them.
/// Intended to be the entry point of the main UI for all things pertaining
/// to sound processors, their inputs, connections, and numeric UIs.
pub struct SoundGraphLayout {
    /// The set of top-level stacked groups of sound processors
    groups: Vec<StackedGroup>,
}

/// Public methods
impl SoundGraphLayout {
    /// The default on-screen width of a stacked group, in pixels
    const DEFAULT_WIDTH: usize = 600;

    /// The default temporal duration of a stacked group, in seconds
    const DEFAULT_DURATION: f32 = 4.0;

    /// Construct a new, empty SoundGraphLayout. See `renegerate` below for
    /// how to populate a SoundGraphLayout automatically from an existing
    /// SoundGraphTopology instance.
    pub(crate) fn new() -> SoundGraphLayout {
        SoundGraphLayout { groups: Vec::new() }
    }

    /// Find the stacked group that a sound processor belongs to, if any.
    pub(crate) fn find_group(&self, id: SoundProcessorId) -> Option<&StackedGroup> {
        for g in &self.groups {
            if g.processors.contains(&id) {
                return Some(g);
            }
        }
        None
    }

    /// Find the stacked group that a sound processor belongs to, if any.
    pub(crate) fn find_group_mut(&mut self, id: SoundProcessorId) -> Option<&mut StackedGroup> {
        for g in &mut self.groups {
            if g.processors.contains(&id) {
                return Some(g);
            }
        }
        None
    }

    /// Returns true if the given sound processor belongs to a group
    /// and is the very first/top processor in that group.
    pub(crate) fn is_top_of_group(&self, id: SoundProcessorId) -> bool {
        let Some(group) = self.find_group(id) else {
            return false;
        };

        let top_id: SoundProcessorId = *group.processors.first().unwrap();
        top_id == id
    }

    /// Returns true if the given sound processor belongs to a group
    /// and is the very last/bottom in that group.
    pub(crate) fn is_bottom_of_group(&self, id: SoundProcessorId) -> bool {
        let Some(group) = self.find_group(id) else {
            return false;
        };

        let bottom_id: SoundProcessorId = *group.processors.last().unwrap();
        bottom_id == id
    }

    /// Update the layout to remove any deleted processors, include
    /// any newly-added sound processors, and generally keep things
    /// tidy and valid as the topology changes programmatically and
    /// without simultaneous changes to the layout.
    pub(crate) fn regenerate(
        &mut self,
        topo: &SoundGraphTopology,
        positions: &SoundObjectPositions,
    ) {
        self.remove_dangling_processor_ids(topo, positions);

        // precompute the number of sound inputs that each processor
        // is connected to
        let mut dependent_counts: HashMap<SoundProcessorId, usize> =
            topo.sound_processors().keys().map(|k| (*k, 0)).collect();

        for si in topo.sound_inputs().values() {
            if let Some(spid) = si.target() {
                *dependent_counts.entry(spid).or_insert(0) += 1;
            }
        }

        // Every processor except those with a single connected sound
        // input must be at the top of a group
        for proc in topo.sound_processors().values() {
            let inputs = proc.sound_inputs();
            if inputs.len() == 1 {
                if topo.sound_input(inputs[0]).unwrap().target().is_some() {
                    continue;
                }
            }

            // If the processor is already in a group, split it.
            // Otherwise, add a new group for it. Other newly-added
            // processors which should belong to the same group
            // will be added below.
            if self.find_group(proc.id().into()).is_some() {
                self.split_group_above_processor(proc.id(), positions);
            } else {
                let procs = vec![proc.id()];
                self.groups
                    .push(StackedGroup::new_at_top_processor(procs, positions));
            }
        }

        // every existing processor with zero or more than one dependent must
        // be at the bottom of a group
        for (spid, dep_count) in dependent_counts {
            if dep_count != 1 {
                // If the processor is already in a group, split it.
                // Otherwise, do nothing. It will be appended onto
                // an existing group in the next phase.
                if self.find_group(spid.into()).is_some() {
                    self.split_group_below_processor(spid, positions);
                }
            }
        }

        // Finally, add every remaining processor to a group.
        // Because of the above steps, every remaining processor
        // in the topology which is not yet in a group has exactly
        // one connected sound input. Repeatedly search for remaining
        // processors which are connected to the bottom processor of
        // an existing group, and add them.
        let mut remaining_processors: Vec<SoundProcessorId> = topo
            .sound_processors()
            .keys()
            .cloned()
            .filter(|i| self.find_group(*i).is_none())
            .collect();

        while !remaining_processors.is_empty() {
            let mut added_processor = None;
            for spid in &remaining_processors {
                let inputs = topo.sound_processor(*spid).unwrap().sound_inputs();
                debug_assert_eq!(inputs.len(), 1);
                let input = topo.sound_input(inputs[0]).unwrap();
                let target = input.target().unwrap();

                if self.is_bottom_of_group(target) {
                    let existing_group = self.find_group_mut(target).unwrap();
                    existing_group.processors.push(*spid);
                    added_processor = Some(target);
                    break;
                }
            }

            let added_processor = added_processor.expect(
                "Oops, seems like something went wrong while regenerating the SoundGraphLayout",
            );

            remaining_processors.retain(|i| *i != added_processor);
        }
    }

    /// Draw the layout and every group to the ui
    pub(crate) fn draw(
        &mut self,
        ui: &mut egui::Ui,
        factories: &Factories,
        ui_state: &mut SoundGraphUiState,
        graph: &mut SoundGraph,
        available_arguments: &HashMap<SoundExpressionId, HashSet<SoundExpressionArgumentId>>,
    ) {
        for group in &mut self.groups {
            group.draw(ui, factories, ui_state, graph, available_arguments);
        }

        // TODO: draw wires between connected groups also
    }

    #[cfg(debug_assertions)]
    pub(crate) fn check_invariants(&self, topo: &SoundGraphTopology) -> bool {
        // every sound processor in the topology must appear exactly once

        for spid in topo.sound_processors().keys().cloned() {
            let number_of_appearances: usize = self
                .groups
                .iter()
                .map(|group| group.processors.iter().filter(|i| **i == spid).count())
                .sum();

            if number_of_appearances != 1 {
                if number_of_appearances == 0 {
                    println!(
                        "The sound processor {} does not appear any groups",
                        topo.sound_processor(spid).unwrap().friendly_name()
                    );
                } else {
                    println!(
                        "The sound processor {} appears in more than one group",
                        topo.sound_processor(spid).unwrap().friendly_name()
                    );
                }
                return false;
            }
        }

        // every sound processor in the layout must exist in the topology
        for group in &self.groups {
            for spid in &group.processors {
                if !topo.contains((*spid).into()) {
                    println!(
                        "The layout contains a sound processor #{} which no longer exists",
                        spid.value()
                    );
                    return false;
                }
            }
        }

        // Every connection implied by adjacent processors in a stacked
        // group must exist and be unique (see `connection_is_unique` for
        // a definition)
        for group in &self.groups {
            for (top_proc, bottom_proc) in
                group.processors.iter().zip(group.processors.iter().skip(1))
            {
                if !Self::connection_is_unique(*top_proc, *bottom_proc, topo) {
                    println!(
                        "The processors {} and {} are adjacent in a group but do not have a unique connection",
                        topo.sound_processor(*top_proc).unwrap().friendly_name(),
                        topo.sound_processor(*bottom_proc).unwrap().friendly_name()
                    );
                    return false;
                }
            }
        }

        true
    }
}

/// Internal helper methods
impl SoundGraphLayout {
    /// Remove sound processors which no longer exist from any groups they
    /// appear in, splitting groups where connections are broken and
    /// removing any empty groups that result.
    fn remove_dangling_processor_ids(
        &mut self,
        topo: &SoundGraphTopology,
        positions: &SoundObjectPositions,
    ) {
        // delete any removed processor ids
        for group in &mut self.groups {
            group.processors.retain(|i| topo.contains((*i).into()));
        }

        // Remove any empty groups
        self.groups.retain(|g| !g.processors.is_empty());

        // split any groups where they imply connections that no longer exist
        let mut new_groups = Vec::new();
        for group in &mut self.groups {
            // Iterate over the group connections in reverse so that
            // we can repeatedly split off the remaining end of the vector
            for i in (1..group.processors.len()).rev() {
                let top_proc = group.processors[i - 1];
                let bottom_proc = group.processors[i];

                // If the connection isn't unique, split off the remainder
                // of the stack into a separate group
                if !Self::connection_is_unique(top_proc, bottom_proc, topo) {
                    let procs = group.processors.split_off(i);
                    new_groups.push(StackedGroup::new_at_top_processor(procs, positions));
                }
            }
        }

        self.groups.append(&mut new_groups);
    }

    /// Find the group that the processor belongs to, and if there
    /// are any processors above it, split those off into a separate
    /// group. This would be done for example if the given processor
    /// just gained a new sound input, which breaks uniqueness of the
    /// group's implied connection above the processor.
    pub(crate) fn split_group_above_processor(
        &mut self,
        processor_id: SoundProcessorId,
        positions: &SoundObjectPositions,
    ) {
        let group = self.find_group_mut(processor_id).unwrap();
        let i = group
            .processors
            .iter()
            .position(|p| *p == processor_id)
            .unwrap();

        if i == 0 {
            return;
        }

        let rest_inclusive = group.processors.split_off(i);
        self.groups.push(StackedGroup::new_at_top_processor(
            rest_inclusive,
            positions,
        ));
    }

    /// Find the group that processor belongs to, and if there are
    /// any processors below it, split those off into a separate
    /// group. This would be done for example if the given processor
    /// was just connected to another sound input elsewhere, which
    /// breaks uniqueness of the group's implied connection below the
    /// processor.
    pub(crate) fn split_group_below_processor(
        &mut self,
        processor_id: SoundProcessorId,
        positions: &SoundObjectPositions,
    ) {
        let group = self.find_group_mut(processor_id).unwrap();
        let i = group
            .processors
            .iter()
            .position(|p| *p == processor_id)
            .unwrap();
        let rest_exclusive = group.processors.split_off(i + 1);
        if !rest_exclusive.is_empty() {
            self.groups.push(StackedGroup::new_at_top_processor(
                rest_exclusive,
                positions,
            ));
        }
    }

    pub(crate) fn split_processor_into_own_group(
        &mut self,
        processor_id: SoundProcessorId,
        positions: &SoundObjectPositions,
    ) {
        let group = self.find_group_mut(processor_id).unwrap();
        let i = group
            .processors
            .iter()
            .position(|p| *p == processor_id)
            .unwrap();

        // split the group just after the processor into a separate vector
        let rest_exclusive = group.processors.split_off(i + 1);

        // remove the processor at the split point as well
        let spid = group.processors.pop().unwrap();
        debug_assert_eq!(spid, processor_id);

        if i == 0 {
            // if there are no processors before the split, we can just put it back
            group.processors.push(processor_id);
        } else {
            // otherwise, create a new group for the lone processor
            let procs = vec![processor_id];
            self.groups
                .push(StackedGroup::new_at_top_processor(procs, positions));
        }

        // if any processors exist after the split point, move them into their own new group
        if !rest_exclusive.is_empty() {
            self.groups.push(StackedGroup::new_at_top_processor(
                rest_exclusive,
                positions,
            ));
        }
    }

    fn remove_processor(&mut self, processor_id: SoundProcessorId) {
        let mut occurrences = 0;
        self.groups.retain_mut(|group| {
            group.processors.retain(|spid| {
                if *spid == processor_id {
                    occurrences += 1;
                    false
                } else {
                    true
                }
            });
            !group.processors.is_empty()
        });
        debug_assert_eq!(occurrences, 1);
    }

    pub(crate) fn insert_processor_above(
        &mut self,
        processor_to_insert: SoundProcessorId,
        processor_below: SoundProcessorId,
    ) {
        self.remove_processor(processor_to_insert);
        let group = self.find_group_mut(processor_below).unwrap();
        let i = group
            .processors
            .iter()
            .position(|p| *p == processor_below)
            .unwrap();
        group.processors.insert(i, processor_to_insert);
    }

    pub(crate) fn insert_processor_below(
        &mut self,
        processor_to_insert: SoundProcessorId,
        processor_below: SoundProcessorId,
    ) {
        self.remove_processor(processor_to_insert);
        let group = self.find_group_mut(processor_below).unwrap();
        let i = group
            .processors
            .iter()
            .position(|p| *p == processor_below)
            .unwrap();
        group.processors.insert(i + 1, processor_to_insert);
    }

    /// Returns true if and only if:
    ///  - the bottom processor has exactly one sound input
    ///  - the top processor is connected to that sound input
    ///  - the top processor not connected to any other sound inputs
    /// Thinking of the sound graph topology in terms of a directed
    /// acyclic graph, this corresponds to there being exactly one
    /// outbound edge from the top processor which itself is the only
    /// inbound edge to the bottom processor.
    fn connection_is_unique(
        top_processor: SoundProcessorId,
        bottom_processor: SoundProcessorId,
        topo: &SoundGraphTopology,
    ) -> bool {
        let inputs = topo
            .sound_processor(bottom_processor)
            .unwrap()
            .sound_inputs();

        // check that the bottom processor has 1 input
        if inputs.len() != 1 {
            return false;
        }

        // check that the top processor is connected to that input
        let input_id = inputs[0];
        let input_target = topo.sound_input(input_id).unwrap().target();
        if input_target != Some(top_processor) {
            return false;
        }

        // check that no other inputs are connected to the top processor
        for other_input in topo.sound_inputs().values() {
            if other_input.id() != input_id && other_input.target() == Some(top_processor) {
                return false;
            }
        }

        true
    }
}
