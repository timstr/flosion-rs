use std::collections::HashMap;

use eframe::egui;
use hashstash::{Stashable, Stasher};

use crate::{
    core::{
        jit::cache::JitCache,
        sound::{soundgraph::SoundGraph, soundprocessor::SoundProcessorId},
    },
    ui_core::{
        flosion_ui::Factories, graph_properties::GraphProperties,
        interactions::draganddrop::DragDropSubject, soundgraphuistate::SoundGraphUiState,
        soundobjectpositions::SoundObjectPositions,
    },
};

use super::stackedgroup::StackedGroup;

/// Visual layout of all processor groups and the connections between them.
/// Intended to be the entry point of the main UI for all things pertaining
/// to sound processors, their inputs, connections, and numeric UIs.
pub struct StackedLayout {
    /// The set of top-level stacked groups of sound processors
    groups: Vec<StackedGroup>,
}

impl StackedLayout {
    /// The default on-screen width of a stacked group, in pixels
    pub(crate) const DEFAULT_WIDTH: f32 = 600.0;

    /// The default temporal duration of a stacked group, in seconds
    pub(crate) const DEFAULT_DURATION: f32 = 4.0;

    /// Construct a new, empty SoundGraphLayout. See `renegerate` below for
    /// how to populate a SoundGraphLayout automatically from an existing
    /// SoundGraph instance.
    pub(crate) fn new() -> StackedLayout {
        StackedLayout { groups: Vec::new() }
    }

    pub(crate) fn groups(&self) -> &[StackedGroup] {
        &self.groups
    }

    /// Find the stacked group that a sound processor belongs to, if any.
    pub(crate) fn find_group(&self, id: SoundProcessorId) -> Option<&StackedGroup> {
        for g in &self.groups {
            if g.processors().contains(&id) {
                return Some(g);
            }
        }
        None
    }

    /// Find the stacked group that a sound processor belongs to, if any.
    pub(crate) fn find_group_mut(&mut self, id: SoundProcessorId) -> Option<&mut StackedGroup> {
        for g in &mut self.groups {
            if g.processors().contains(&id) {
                return Some(g);
            }
        }
        None
    }

    pub(crate) fn is_processor_alone(&self, id: SoundProcessorId) -> bool {
        if let Some(g) = self.find_group(id) {
            g.processors() == &[id]
        } else {
            false
        }
    }

    pub(crate) fn processor_above(&self, id: SoundProcessorId) -> Option<SoundProcessorId> {
        if let Some(g) = self.find_group(id) {
            let procs = g.processors();
            let i = procs.iter().position(|x| *x == id).unwrap();
            if i > 0 {
                Some(procs[i - 1])
            } else {
                None
            }
        } else {
            None
        }
    }

    pub(crate) fn processor_below(&self, id: SoundProcessorId) -> Option<SoundProcessorId> {
        if let Some(g) = self.find_group(id) {
            let procs = g.processors();
            let i = procs.iter().position(|x| *x == id).unwrap();
            if i + 1 == procs.len() {
                None
            } else {
                Some(procs[i + 1])
            }
        } else {
            None
        }
    }

    /// Returns true if the given sound processor belongs to a group
    /// and is the very first/top processor in that group.
    pub(crate) fn is_top_of_group(&self, id: SoundProcessorId) -> bool {
        let Some(group) = self.find_group(id) else {
            return false;
        };

        let top_id: SoundProcessorId = *group.processors().first().unwrap();
        top_id == id
    }

    /// Returns true if the given sound processor belongs to a group
    /// and is the very last/bottom in that group.
    pub(crate) fn is_bottom_of_group(&self, id: SoundProcessorId) -> bool {
        let Some(group) = self.find_group(id) else {
            return false;
        };

        let bottom_id: SoundProcessorId = *group.processors().last().unwrap();
        bottom_id == id
    }

    /// Update the layout to remove any deleted processors, include
    /// any newly-added sound processors, and generally keep things
    /// tidy and valid as the graph changes programmatically and
    /// without simultaneous changes to the layout.
    pub(crate) fn regenerate(&mut self, graph: &SoundGraph, positions: &SoundObjectPositions) {
        self.remove_dangling_processor_ids(graph, positions);

        // precompute the number of sound inputs that each processor
        // is connected to
        let mut dependent_counts: HashMap<SoundProcessorId, usize> =
            graph.sound_processors().keys().map(|k| (*k, 0)).collect();

        for proc_data in graph.sound_processors().values() {
            proc_data.foreach_input(|input, _| {
                if let Some(spid) = input.target() {
                    *dependent_counts.entry(spid).or_insert(0) += 1;
                }
            });
        }

        // Every processor except those with a single connected sound
        // input must be at the top of a group
        for proc in graph.sound_processors().values() {
            let inputs = proc.input_locations();
            if inputs.len() == 1 {
                if proc
                    .with_input(inputs[0].input(), |i| i.target().is_some())
                    .unwrap()
                {
                    continue;
                }
            }

            // If the processor is already in a group, split it.
            // Otherwise, add a new group for it. Other newly-added
            // processors which should belong to the same group
            // will be added below.
            if self.find_group(proc.id()).is_some() {
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
                if self.find_group(spid).is_some() {
                    self.split_group_below_processor(spid, positions);
                }
            }
        }

        // Finally, add every remaining processor to a group.
        // Because of the above steps, every remaining processor
        // in the graph which is not yet in a group has exactly
        // one connected sound input. Repeatedly search for remaining
        // processors which are connected to the bottom processor of
        // an existing group, and add them.
        let mut remaining_processors: Vec<SoundProcessorId> = graph
            .sound_processors()
            .keys()
            .cloned()
            .filter(|i| self.find_group(*i).is_none())
            .collect();

        while !remaining_processors.is_empty() {
            let mut added_processor = None;
            for spid in &remaining_processors {
                let proc_data = graph.sound_processor(*spid).unwrap();
                let inputs = proc_data.input_locations();
                debug_assert_eq!(inputs.len(), 1);
                let target = proc_data
                    .with_input(inputs[0].input(), |i| i.target())
                    .unwrap()
                    .unwrap();

                if self.is_bottom_of_group(target) {
                    let existing_group = self.find_group_mut(target).unwrap();
                    existing_group.insert_processor_at_bottom(*spid);
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
        properties: &GraphProperties,
        jit_cache: &JitCache,
    ) {
        // Draw each stacked group
        for group in &mut self.groups {
            group.draw(ui, factories, ui_state, graph, jit_cache, properties);
        }

        // draw wires between connected groups
        // TODO: make this prettier
        for (jumper_input, jumper_pos) in ui_state.positions().socket_jumpers().items() {
            let Some(target_spid) = graph
                .with_sound_input(*jumper_input, |i| i.target())
                .unwrap()
            else {
                continue;
            };

            let plug_pos = ui_state
                .positions()
                .drag_drop_subjects()
                .position(&DragDropSubject::Plug(target_spid))
                .unwrap();

            let color = ui_state
                .object_states()
                .get_object_color(target_spid.into());

            let src_pos = jumper_pos.left_center();
            let dst_pos = plug_pos.left_center();

            let via_x = src_pos.x.min(dst_pos.x) - 30.0;

            let via_pos_1 = egui::pos2(via_x, src_pos.y);
            let via_pos_2 = egui::pos2(via_x, dst_pos.y);

            let stroke = egui::Stroke::new(5.0, color);

            ui.painter().line_segment([src_pos, via_pos_1], stroke);
            ui.painter().line_segment([via_pos_1, via_pos_2], stroke);
            ui.painter().line_segment([via_pos_2, dst_pos], stroke);
        }
    }

    #[cfg(debug_assertions)]
    pub(crate) fn check_invariants(&self, graph: &SoundGraph) -> bool {
        // every sound processor in the graph must appear exactly once

        for spid in graph.sound_processors().keys().cloned() {
            let number_of_appearances: usize = self
                .groups
                .iter()
                .map(|group| group.processors().iter().filter(|i| **i == spid).count())
                .sum();

            if number_of_appearances != 1 {
                if number_of_appearances == 0 {
                    println!(
                        "The sound processor {} does not appear any groups",
                        graph
                            .sound_processor(spid)
                            .unwrap()
                            .as_graph_object()
                            .friendly_name()
                    );
                } else {
                    println!(
                        "The sound processor {} appears in more than one group",
                        graph
                            .sound_processor(spid)
                            .unwrap()
                            .as_graph_object()
                            .friendly_name()
                    );
                }
                return false;
            }
        }

        // every sound processor in the layout must exist in the graph
        for group in &self.groups {
            for spid in group.processors() {
                if !graph.contains(spid) {
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
            for (top_proc, bottom_proc) in group
                .processors()
                .iter()
                .zip(group.processors().iter().skip(1))
            {
                if !Self::connection_is_unique(*top_proc, *bottom_proc, graph) {
                    println!(
                        "Processor {} is above processor {} in a group but the two do not have a unique connection",
                        graph.sound_processor(*top_proc).unwrap().as_graph_object().friendly_name(),
                        graph.sound_processor(*bottom_proc).unwrap().as_graph_object().friendly_name()
                    );
                    return false;
                }
            }
        }

        true
    }

    /// Remove sound processors which no longer exist from any groups they
    /// appear in, splitting groups where connections are broken and
    /// removing any empty groups that result.
    fn remove_dangling_processor_ids(
        &mut self,
        graph: &SoundGraph,
        positions: &SoundObjectPositions,
    ) {
        // delete any removed processor ids
        for group in &mut self.groups {
            group.remove_dangling_processor_ids(graph);
        }

        // Remove any empty groups
        self.groups.retain(|g| !g.processors().is_empty());

        // split any groups where they imply connections that no longer exist
        let mut new_groups = Vec::new();
        for group in &mut self.groups {
            // Iterate over the group connections in reverse so that
            // we can repeatedly split off the remaining end of the vector

            for i in (1..group.processors().len()).rev() {
                let top_proc = group.processors()[i - 1];
                let bottom_proc = group.processors()[i];

                // If the connection isn't unique, split off the remainder
                // of the stack into a separate group
                if !Self::connection_is_unique(top_proc, bottom_proc, graph) {
                    let procs = group.split_off_everything_below_processor(top_proc);
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
        if group.processor_is_at_top(processor_id) {
            return;
        }

        let rest = group.split_off_processor_and_everything_below(processor_id);

        group.set_rect(group.rect().translate(egui::vec2(0.0, -50.0)));

        debug_assert!(!group.processors().is_empty());

        self.groups
            .push(StackedGroup::new_at_top_processor(rest, positions));
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
        let rest_exclusive = group.split_off_everything_below_processor(processor_id);
        if !rest_exclusive.is_empty() {
            let mut new_group = StackedGroup::new_at_top_processor(rest_exclusive, positions);
            new_group.set_rect(new_group.rect().translate(egui::vec2(0.0, 50.0)));
            self.groups.push(new_group);
        }
    }

    pub(crate) fn split_processor_into_own_group(
        &mut self,
        processor_id: SoundProcessorId,
        positions: &SoundObjectPositions,
    ) {
        let group = self.find_group_mut(processor_id).unwrap();
        let rest_exclusive = group.split_off_everything_below_processor(processor_id);

        // remove the processor at the split point as well
        group.remove_processor(processor_id);

        if group.processors().is_empty() {
            // if there are no processors before the split, we can just put it back
            group.insert_processor_at_bottom(processor_id);
        } else {
            // otherwise, create a new group for the lone processor
            let procs = vec![processor_id];
            self.groups
                .push(StackedGroup::new_at_top_processor(procs, positions));
        }

        // if any processors exist after the split point, move them into their own new group
        if !rest_exclusive.is_empty() {
            let mut rest_group = StackedGroup::new_at_top_processor(rest_exclusive, positions);
            let magic_offset = -10.0;
            rest_group.set_rect(rest_group.rect().translate(egui::vec2(0.0, magic_offset)));
            self.groups.push(rest_group);
        }
    }

    pub(crate) fn remove_processor(&mut self, processor_id: SoundProcessorId) {
        self.groups.retain_mut(|group| {
            group.remove_processor(processor_id);
            !group.processors().is_empty()
        });
    }

    pub(crate) fn insert_processor_above(
        &mut self,
        processor_to_insert: SoundProcessorId,
        other_processor: SoundProcessorId,
    ) {
        self.remove_processor(processor_to_insert);
        self.find_group_mut(other_processor)
            .unwrap()
            .insert_processor_above(processor_to_insert, other_processor);
    }

    pub(crate) fn insert_processor_below(
        &mut self,
        processor_to_insert: SoundProcessorId,
        other_processor: SoundProcessorId,
    ) {
        self.remove_processor(processor_to_insert);
        self.find_group_mut(other_processor)
            .unwrap()
            .insert_processor_below(processor_to_insert, other_processor);
    }

    /// Returns true if and only if:
    ///  - the bottom processor has exactly one sound input
    ///  - the top processor is connected to that sound input
    ///  - the top processor not connected to any other sound inputs
    /// Thinking of the sound graph in terms of a directed
    /// acyclic graph, this corresponds to there being exactly one
    /// outbound edge from the top processor which itself is the only
    /// inbound edge to the bottom processor.
    fn connection_is_unique(
        top_processor: SoundProcessorId,
        bottom_processor: SoundProcessorId,
        graph: &SoundGraph,
    ) -> bool {
        let inputs = graph
            .sound_processor(bottom_processor)
            .unwrap()
            .input_locations();

        // check that the bottom processor has 1 input
        if inputs.len() != 1 {
            return false;
        }

        // check that the top processor is connected to that input
        let input_id = inputs[0];
        let input_target = graph.with_sound_input(input_id, |i| i.target()).unwrap();
        if input_target != Some(top_processor) {
            return false;
        }

        // check that no other inputs are connected to the top processor
        let all_connected_inputs = graph.sound_processor_targets(top_processor);
        if all_connected_inputs != [input_id] {
            return false;
        }

        true
    }
}

impl Stashable for StackedLayout {
    fn stash(&self, stasher: &mut Stasher) {
        stasher.array_of_objects_slice(&self.groups, hashstash::Order::Unordered);
    }
}
