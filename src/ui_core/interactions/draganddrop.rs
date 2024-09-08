use std::{collections::HashMap, hash::Hasher};

use eframe::egui;
use hashrevise::{Revisable, RevisedProperty, RevisionHash, RevisionHasher};

use crate::{
    core::sound::{
        soundgraph::SoundGraph, soundinput::SoundInputId, soundprocessor::SoundProcessorId,
    },
    ui_core::{
        soundobjectpositions::SoundObjectPositions, soundobjectuistate::SoundObjectUiStates,
        stackedlayout::stackedlayout::StackedLayout,
    },
};

#[derive(Clone, Copy, Eq, PartialEq, Hash)]
pub enum DragDropSubject {
    Processor(SoundProcessorId),
    Plug(SoundProcessorId),
    Socket(SoundInputId),
    Group { top_processor: SoundProcessorId },
}

impl DragDropSubject {
    fn as_processor(&self) -> Option<SoundProcessorId> {
        match self {
            DragDropSubject::Processor(spid) => Some(*spid),
            DragDropSubject::Plug(spid) => Some(*spid),
            _ => None,
        }
    }

    fn as_input(&self) -> Option<SoundInputId> {
        match self {
            DragDropSubject::Socket(siid) => Some(*siid),
            _ => None,
        }
    }

    fn parent_processor(&self, graph: &SoundGraph) -> Option<SoundProcessorId> {
        match self {
            DragDropSubject::Processor(spid) => Some(*spid),
            DragDropSubject::Plug(spid) => Some(*spid),
            DragDropSubject::Socket(siid) => Some(graph.sound_input(*siid).unwrap().owner()),
            DragDropSubject::Group { top_processor: _ } => None,
        }
    }

    fn is_valid(&self, graph: &SoundGraph) -> bool {
        match self {
            DragDropSubject::Processor(spid) => graph.contains(spid),
            DragDropSubject::Plug(spid) => graph.contains(spid),
            DragDropSubject::Socket(siid) => graph.contains(siid),
            DragDropSubject::Group { top_processor } => graph.contains(top_processor),
        }
    }
}

impl Revisable for DragDropSubject {
    fn get_revision(&self) -> RevisionHash {
        let mut hasher = RevisionHasher::new();
        match self {
            DragDropSubject::Processor(spid) => {
                hasher.write_u8(0);
                hasher.write_revisable(spid);
            }
            DragDropSubject::Plug(spid) => {
                hasher.write_u8(1);
                hasher.write_revisable(spid);
            }
            DragDropSubject::Socket(siid) => {
                hasher.write_u8(2);
                hasher.write_revisable(siid);
            }
            DragDropSubject::Group { top_processor } => {
                hasher.write_u8(3);
                hasher.write_revisable(top_processor);
            }
        }
        hasher.into_revision()
    }
}

#[derive(Copy, Clone, Eq, PartialEq)]
pub enum DragDropLegality {
    Legal,
    LegalButInvisible,
    Illegal,
    Irrelevant,
}

pub enum SelectionChange {
    Replace,
    Add,
    Subtract,
}

fn drop_and_drop_should_be_ignored(
    graph: &mut SoundGraph,
    layout: &StackedLayout,
    drag_from: DragDropSubject,
    drop_onto: DragDropSubject,
) -> bool {
    // Ignore processor plugs for dropping onto except
    // at the bottom of a group
    if let DragDropSubject::Plug(spid) = drop_onto {
        let group = layout.find_group(spid).unwrap();
        if group.processors().last().unwrap().clone() != spid {
            return true;
        }
    }

    match drag_from {
        DragDropSubject::Processor(spid) => {
            // If dragging a processor, ignore that processor's own things
            drop_onto.parent_processor(graph) == Some(spid)
        }
        DragDropSubject::Plug(spid) => {
            // If dragging a processor's plug, ignore its own things
            drop_onto.parent_processor(graph) == Some(spid)
        }
        DragDropSubject::Socket(siid) => {
            // If dragging an input socket, ignore its own processor
            let owner = graph.sound_input(siid).unwrap().owner();
            drop_onto.parent_processor(graph) == Some(owner)
        }
        DragDropSubject::Group { top_processor } => {
            // If dragging a group, ignore anything in the group
            if let Some(parent) = drop_onto.parent_processor(graph) {
                let parents_top_proc = layout
                    .find_group(parent)
                    .unwrap()
                    .processors()
                    .first()
                    .unwrap()
                    .clone();
                parents_top_proc == top_processor
            } else {
                false
            }
        }
    }
}

fn drag_and_drop_in_graph(
    graph: &mut SoundGraph,
    layout: &StackedLayout,
    drag_from: DragDropSubject,
    drop_onto: DragDropSubject,
) -> DragDropLegality {
    // ignore some basic things first
    if drop_and_drop_should_be_ignored(graph, layout, drag_from, drop_onto) {
        return DragDropLegality::Irrelevant;
    }

    // Whether dragging a processor or a stacked group of
    // processors, look at just their end tops and bottoms
    // and otherwise treat them the same.
    let drag_proc_ends = match drag_from {
        DragDropSubject::Processor(spid) => Some((spid, spid)),
        DragDropSubject::Group {
            top_processor: top_spid,
        } => {
            let bottom_spid = layout
                .find_group(top_spid)
                .unwrap()
                .processors()
                .last()
                .unwrap()
                .clone();
            Some((top_spid, bottom_spid))
        }
        _ => None,
    };

    if let (Some((drag_top_proc, drag_bottom_proc)), DragDropSubject::Socket(onto_input)) =
        (drag_proc_ends, drop_onto)
    {
        // If dragging an entire processor/group onto a socket, splice it in

        // Disconnect the top processor's inputs
        disconnect_all_inputs_of_processor(drag_top_proc, graph);

        // Disconnect the bottom processor's outputs
        disconnect_processor_from_all_inputs(drag_bottom_proc, graph);

        // Note the target of the input being dropped onto, if any,
        // before disconnecting it
        let input_target = graph.sound_input(onto_input).unwrap().target();
        if input_target.is_some() {
            graph.disconnect_sound_input(onto_input).unwrap();
        }

        // Connect the socket to the bottom processor
        graph
            .connect_sound_input(onto_input, drag_bottom_proc)
            .unwrap();

        // If there was something connected to the input, and the top
        // processor has exactly one input, reconnect it
        if let (Some(input_target), [top_input]) = (
            input_target,
            graph.sound_processor(drag_top_proc).unwrap().sound_inputs(),
        ) {
            graph.connect_sound_input(*top_input, input_target).unwrap();
        }

        DragDropLegality::Legal
    } else if let (Some((drag_top_proc, drag_bottom_proc)), DragDropSubject::Plug(onto_plug)) =
        (drag_proc_ends, drop_onto)
    {
        // If dragging an entire processor/group onto a plug, splice it in

        // Only allowed if dropping a top processor with one input
        let [top_input] = graph.sound_processor(drag_top_proc).unwrap().sound_inputs() else {
            return DragDropLegality::Irrelevant;
        };
        let top_input: SoundInputId = *top_input;

        // Disconnect the top processor's inputs
        disconnect_all_inputs_of_processor(drag_top_proc, graph);

        // Disconnect the bottom processor's outputs
        disconnect_processor_from_all_inputs(drag_bottom_proc, graph);

        // Note the inputs the plug is connected to, and disconnect them
        let plug_targets: Vec<SoundInputId> = graph.sound_processor_targets(onto_plug).collect();
        for target in &plug_targets {
            graph.disconnect_sound_input(*target).unwrap();
        }

        // Connect the top processor to the plug
        graph.connect_sound_input(top_input, onto_plug).unwrap();

        // Reconnect the plug targets to the bottom of the group
        for target in plug_targets {
            graph.connect_sound_input(target, drag_bottom_proc).unwrap();
        }

        DragDropLegality::Legal
    } else if let (DragDropSubject::Socket(drag_input), Some(onto_proc)) =
        (drag_from, drop_onto.as_processor())
    {
        // If dragging an input socket onto a processor or its plug, connect it

        // Disconnect the input first
        if graph.sound_input(drag_input).unwrap().target().is_some() {
            graph.disconnect_sound_input(drag_input).unwrap();
        }

        graph.connect_sound_input(drag_input, onto_proc).unwrap();

        DragDropLegality::Legal
    } else if let (DragDropSubject::Plug(drag_proc), Some(onto_input)) =
        (drag_from, drop_onto.as_input())
    {
        // If dragging a processor plug onto an input socket, connect it

        // Disconnect the input first
        if graph.sound_input(onto_input).unwrap().target().is_some() {
            graph.disconnect_sound_input(onto_input).unwrap();
        }

        graph.connect_sound_input(onto_input, drag_proc).unwrap();

        DragDropLegality::Legal
    } else {
        DragDropLegality::Irrelevant
    }
}

fn drag_and_drop_in_layout(
    layout: &mut StackedLayout,
    graph: &SoundGraph,
    drag_from: DragDropSubject,
    drop_onto: DragDropSubject,
    positions: &SoundObjectPositions,
) {
    let processors_being_dragged: Option<Vec<SoundProcessorId>> = match drag_from {
        DragDropSubject::Processor(spid) => Some(vec![spid]),
        DragDropSubject::Group { top_processor } => Some(
            layout
                .find_group(top_processor)
                .unwrap()
                .processors()
                .to_vec(),
        ),
        _ => None,
    };

    match (processors_being_dragged, drop_onto) {
        (Some(processors), DragDropSubject::Socket(input)) => {
            // Dragging a processor onto an input. Move the processor
            // and insert it at the group under the input.
            let input_data = graph.sound_input(input).unwrap();
            let proc_below = input_data.owner();

            if layout.is_top_of_group(proc_below) {
                for proc in processors {
                    layout.insert_processor_above(proc, proc_below);
                    let group = layout.find_group_mut(proc).unwrap();
                    let proc_pos = positions.find_processor(proc).unwrap();
                    let magic_offset = -5.0;
                    let delta = proc_pos.rect.bottom() - proc_pos.group_origin.y + magic_offset;
                    let rect = group.rect().translate(egui::vec2(0.0, -delta));
                    group.set_rect(rect);
                }
            } else {
                for proc in processors {
                    layout.remove_processor(proc);
                    layout.insert_processor_above(proc, proc_below);
                }
            }
        }
        (Some(processors), DragDropSubject::Plug(plug)) => {
            // Dragging a processor onto the bottom plug of a stacked group
            // (assuming that this is only being called after drag_and_drop_in_graph
            // has deemed it legal). Move the processor to the bottom of the group.
            for proc in processors.into_iter().rev() {
                layout.remove_processor(proc);
                layout.insert_processor_below(proc, plug);
            }
        }
        _ => {
            // Otherwise, only plugs/sockets are being dragged, no layout changes
            // are needed that can't be resolved normally by SoundGraphLayout::regenerate
        }
    }

    layout.regenerate(graph, positions);
}

fn compute_legal_drop_sites(
    graph: &SoundGraph,
    layout: &StackedLayout,
    drag_subject: DragDropSubject,
    drop_sites: &[DragDropSubject],
) -> HashMap<DragDropSubject, DragDropLegality> {
    debug_assert_eq!(graph.validate(), Ok(()));
    let mut site_statuses = HashMap::new();
    for drop_site in drop_sites {
        let mut graph_clone = graph.clone();
        // drag_and_drop_in_graph only does superficial error
        // detection, here we additionally check whether the
        // resulting graph is valid.
        let status =
            match drag_and_drop_in_graph(&mut graph_clone, layout, drag_subject, *drop_site) {
                DragDropLegality::Legal => {
                    if graph_clone.validate().is_err() {
                        DragDropLegality::Illegal
                    } else {
                        DragDropLegality::Legal
                    }
                }
                DragDropLegality::LegalButInvisible => {
                    if graph_clone.validate().is_err() {
                        DragDropLegality::Irrelevant
                    } else {
                        DragDropLegality::LegalButInvisible
                    }
                }
                DragDropLegality::Illegal => DragDropLegality::Illegal,
                DragDropLegality::Irrelevant => DragDropLegality::Irrelevant,
            };
        site_statuses.insert(*drop_site, status);
    }
    site_statuses
}

const MIN_DROP_OVERLAP: f32 = 1000.0;

pub struct DragInteraction {
    subject: DragDropSubject,
    rect: egui::Rect,
    original_rect: egui::Rect,
    legal_drop_sites: RevisedProperty<HashMap<DragDropSubject, DragDropLegality>>,
    closest_legal_site: Option<DragDropSubject>,
}

impl DragInteraction {
    pub(crate) fn new(subject: DragDropSubject, original_rect: egui::Rect) -> DragInteraction {
        DragInteraction {
            subject,
            rect: original_rect,
            original_rect,
            legal_drop_sites: RevisedProperty::new(),
            closest_legal_site: None,
        }
    }

    pub(crate) fn interact_and_draw(
        &mut self,
        ui: &mut egui::Ui,
        graph: &SoundGraph,
        object_states: &SoundObjectUiStates,
        layout: &StackedLayout,
        positions: &SoundObjectPositions,
    ) {
        // Ensure that the legal connections are up to date, since these are used
        // to highlight legal/illegal interconnects to drop onto
        self.legal_drop_sites.refresh4(
            compute_legal_drop_sites,
            graph,
            layout,
            self.subject,
            positions.drag_drop_subjects().values(),
        );
        let site_is_legal = |s: &DragDropSubject| -> bool {
            self.legal_drop_sites.get_cached().unwrap().get(s).cloned()
                == Some(DragDropLegality::Legal)
        };
        self.closest_legal_site = positions
            .drag_drop_subjects()
            .find_closest_where(self.rect, MIN_DROP_OVERLAP, site_is_legal)
            .cloned();

        // Highlight the legal and illegal drop sites
        for (drop_site, legality) in self.legal_drop_sites.get_cached().unwrap() {
            let color = match legality {
                DragDropLegality::Legal => egui::Color32::WHITE,
                DragDropLegality::LegalButInvisible => continue,
                DragDropLegality::Illegal => egui::Color32::RED,
                DragDropLegality::Irrelevant => continue,
            };

            let alpha = if self.closest_legal_site == Some(*drop_site) {
                128
            } else {
                64
            };

            let color =
                egui::Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), alpha);

            ui.painter().rect_filled(
                positions.drag_drop_subjects().position(drop_site).unwrap(),
                egui::Rounding::same(2.0),
                color,
            );
        }
        // Draw a basic coloured rectangle tracking the cursor as a preview of
        // the subject being dragged, without drawing its ui twice
        let drag_subject_processor = match self.subject {
            DragDropSubject::Processor(spid) => Some(spid),
            DragDropSubject::Plug(spid) => Some(spid),
            DragDropSubject::Socket(siid) => Some(graph.sound_input(siid).unwrap().owner()),
            DragDropSubject::Group { top_processor: _ } => {
                // Groups get dragged directly and so don't need a
                // preview. They don't have just one colour anyway.
                None
            }
        };
        if let Some(drag_subject_processor) = drag_subject_processor {
            let color = object_states.get_object_color(drag_subject_processor.into());
            let color = egui::Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 64);
            ui.painter()
                .rect_filled(self.rect, egui::Rounding::same(5.0), color);
        }
    }

    pub(crate) fn rect(&self) -> egui::Rect {
        self.rect
    }

    pub(crate) fn set_rect(&mut self, rect: egui::Rect) {
        self.rect = rect;
    }

    pub(crate) fn is_valid(&self, graph: &SoundGraph) -> bool {
        self.subject.is_valid(graph)
    }
}

#[derive(Clone)]
pub struct DropInteraction {
    subject: DragDropSubject,
    rect: egui::Rect,
    original_rect: egui::Rect,
    legal_sites: HashMap<DragDropSubject, DragDropLegality>,
}

impl DropInteraction {
    pub(crate) fn new_from_drag(drag: &DragInteraction) -> DropInteraction {
        DropInteraction {
            subject: drag.subject,
            rect: drag.rect,
            original_rect: drag.original_rect,
            legal_sites: drag.legal_drop_sites.get_cached().cloned().unwrap(),
        }
    }

    pub(crate) fn handle_drop(
        &self,
        graph: &mut SoundGraph,
        layout: &mut StackedLayout,
        positions: &mut SoundObjectPositions,
    ) {
        let nearest_drop_site = positions.drag_drop_subjects().find_closest_where(
            self.rect,
            MIN_DROP_OVERLAP,
            |site| self.legal_sites.get(site).cloned() == Some(DragDropLegality::Legal),
        );

        if let Some(nearest_drop_site) = nearest_drop_site {
            // No point in checking invariants later if they aren't
            // already upheld
            #[cfg(debug_assertions)]
            assert!(layout.check_invariants(graph));

            let drag_and_drop_result = graph.try_make_change(|graph| {
                Ok(drag_and_drop_in_graph(
                    graph,
                    layout,
                    self.subject,
                    *nearest_drop_site,
                ))
            });

            match drag_and_drop_result {
                Ok(DragDropLegality::Legal) => { /* nice */ }
                Ok(DragDropLegality::LegalButInvisible) => { /* nice */ }
                Ok(DragDropLegality::Illegal) => {
                    println!("Nope, can't drop that there.");
                    return;
                }
                Ok(DragDropLegality::Irrelevant) => {
                    println!("How did you do that???");
                    return;
                }
                Err(e) => {
                    println!("Can't drop that there: {}", e.explain(graph));
                    return;
                }
            }

            drag_and_drop_in_layout(layout, graph, self.subject, *nearest_drop_site, positions);

            #[cfg(debug_assertions)]
            assert!(layout.check_invariants(graph));
        } else {
            // If a processor was dropped far away from anything, split
            // it into its own group
            if let DragDropSubject::Processor(spid) = self.subject {
                if !layout.is_processor_alone(spid) {
                    graph
                        .try_make_change(|graph| {
                            disconnect_processor_in_graph(spid, graph);
                            Ok(())
                        })
                        .unwrap();

                    layout.split_processor_into_own_group(spid, positions);

                    #[cfg(debug_assertions)]
                    assert!(layout.check_invariants(graph));
                }
            }
        }

        // If a processor in a lone group was dropped, move the group to
        // where the processor was dropped
        if let DragDropSubject::Processor(spid) = self.subject {
            let group = layout.find_group_mut(spid).unwrap();
            if group.processors() == &[spid] {
                let rect = group.rect();
                group
                    .set_rect(rect.translate(self.rect.left_top() - self.original_rect.left_top()));
            }
        }
    }

    pub(crate) fn is_valid(&self, graph: &SoundGraph) -> bool {
        self.subject.is_valid(graph)
    }
}

// TODO: move these methods to soundgraph.rs?

fn disconnect_all_inputs_of_processor(processor_id: SoundProcessorId, graph: &mut SoundGraph) {
    let mut inputs_to_disconnect_from: Vec<SoundInputId> = Vec::new();
    for i in graph.sound_processor(processor_id).unwrap().sound_inputs() {
        if graph.sound_input(*i).unwrap().target().is_some() {
            inputs_to_disconnect_from.push(*i);
        }
    }
    for i in inputs_to_disconnect_from {
        graph.disconnect_sound_input(i).unwrap();
    }
}

fn disconnect_processor_from_all_inputs(processor_id: SoundProcessorId, graph: &mut SoundGraph) {
    let inputs_to_disconnect_from: Vec<SoundInputId> =
        graph.sound_processor_targets(processor_id).collect();
    for i in inputs_to_disconnect_from {
        graph.disconnect_sound_input(i).unwrap();
    }
}

fn disconnect_processor_in_graph(processor_id: SoundProcessorId, graph: &mut SoundGraph) {
    let mut inputs_to_disconnect_from: Vec<SoundInputId> =
        graph.sound_processor_targets(processor_id).collect();
    for i in graph.sound_processor(processor_id).unwrap().sound_inputs() {
        if graph.sound_input(*i).unwrap().target().is_some() {
            inputs_to_disconnect_from.push(*i);
        }
    }
    for i in inputs_to_disconnect_from {
        graph.disconnect_sound_input(i).unwrap();
    }
}
