use std::collections::HashMap;

use eframe::egui;
use hashstash::{
    stash_clone_with_context, HashCacheProperty, Order, Stash, Stashable, Stasher, UnstashError,
    Unstashable, Unstasher,
};

use crate::{
    core::{
        sound::{
            soundgraph::SoundGraph, soundinput::SoundInputLocation,
            soundprocessor::SoundProcessorId,
        },
        stashing::{StashingContext, UnstashingContext},
    },
    ui_core::{
        factories::Factories, history::SnapshotFlag, soundobjectpositions::SoundObjectPositions,
        soundobjectuistate::SoundObjectUiStates, stackedlayout::stackedlayout::StackedLayout,
    },
};

#[derive(Clone, Copy, Eq, PartialEq, Hash)]
pub enum DragDropSubject {
    Processor(SoundProcessorId),
    Plug(SoundProcessorId),
    Socket(SoundInputLocation),
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

    fn as_input(&self) -> Option<SoundInputLocation> {
        match self {
            DragDropSubject::Socket(siid) => Some(*siid),
            _ => None,
        }
    }

    fn parent_processor(&self) -> Option<SoundProcessorId> {
        match self {
            DragDropSubject::Processor(spid) => Some(*spid),
            DragDropSubject::Plug(spid) => Some(*spid),
            DragDropSubject::Socket(siid) => Some(siid.processor()),
            DragDropSubject::Group { top_processor: _ } => None,
        }
    }

    pub(crate) fn is_valid(&self, graph: &SoundGraph) -> bool {
        match self {
            DragDropSubject::Processor(spid) => graph.contains(spid),
            DragDropSubject::Plug(spid) => graph.contains(spid),
            DragDropSubject::Socket(siid) => graph.contains(siid),
            DragDropSubject::Group { top_processor } => graph.contains(top_processor),
        }
    }
}

impl Stashable for DragDropSubject {
    fn stash(&self, stasher: &mut Stasher) {
        match self {
            DragDropSubject::Processor(spid) => {
                stasher.u8(0);
                spid.stash(stasher);
            }
            DragDropSubject::Plug(spid) => {
                stasher.u8(1);
                spid.stash(stasher);
            }
            DragDropSubject::Socket(input_loc) => {
                stasher.u8(2);
                input_loc.stash(stasher);
            }
            DragDropSubject::Group { top_processor } => {
                stasher.u8(3);
                top_processor.stash(stasher);
            }
        }
    }
}

impl Unstashable for DragDropSubject {
    fn unstash(unstasher: &mut Unstasher) -> Result<Self, UnstashError> {
        let dds = match unstasher.u8()? {
            0 => DragDropSubject::Processor(SoundProcessorId::unstash(unstasher)?),
            1 => DragDropSubject::Plug(SoundProcessorId::unstash(unstasher)?),
            2 => DragDropSubject::Socket(SoundInputLocation::unstash(unstasher)?),
            3 => DragDropSubject::Group {
                top_processor: SoundProcessorId::unstash(unstasher)?,
            },
            _ => panic!(),
        };
        Ok(dds)
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
            drop_onto.parent_processor() == Some(spid)
        }
        DragDropSubject::Plug(spid) => {
            // If dragging a processor's plug, ignore its own things
            drop_onto.parent_processor() == Some(spid)
        }
        DragDropSubject::Socket(siid) => {
            // If dragging an input socket, ignore its own processor
            let owner = siid.processor();
            drop_onto.parent_processor() == Some(owner)
        }
        DragDropSubject::Group { top_processor } => {
            // If dragging a group, ignore anything in the group
            if let Some(parent) = drop_onto.parent_processor() {
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
    if drop_and_drop_should_be_ignored(layout, drag_from, drop_onto) {
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
        let input_target = graph
            .with_sound_input(onto_input, |input| input.target())
            .unwrap();
        if input_target.is_some() {
            graph.disconnect_sound_input(onto_input).unwrap();
        }

        // Connect the socket to the bottom processor
        graph
            .connect_sound_input(onto_input, drag_bottom_proc)
            .unwrap();

        // If there was something connected to the input, and the top
        // processor has exactly one input, reconnect it
        let sound_inputs = graph
            .sound_processor(drag_top_proc)
            .unwrap()
            .input_locations();
        if let (Some(input_target), [top_input]) = (input_target, &sound_inputs[..]) {
            graph.connect_sound_input(*top_input, input_target).unwrap();
        }

        DragDropLegality::Legal
    } else if let (Some((drag_top_proc, drag_bottom_proc)), DragDropSubject::Plug(onto_plug)) =
        (drag_proc_ends, drop_onto)
    {
        // If dragging an entire processor/group onto a plug, splice it in

        // Only allowed if dropping a top processor with one input
        let [top_input] = &graph
            .sound_processor(drag_top_proc)
            .unwrap()
            .input_locations()[..]
        else {
            return DragDropLegality::Irrelevant;
        };
        let top_input: SoundInputLocation = *top_input;

        // Disconnect the top processor's inputs
        disconnect_all_inputs_of_processor(drag_top_proc, graph);

        // Disconnect the bottom processor's outputs
        disconnect_processor_from_all_inputs(drag_bottom_proc, graph);

        // Note the inputs the plug is connected to, and disconnect them
        let plug_targets = graph.inputs_connected_to(onto_plug);
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
        graph
            .with_sound_input_mut(drag_input, |input| input.set_target(None))
            .unwrap();

        graph.connect_sound_input(drag_input, onto_proc).unwrap();

        DragDropLegality::Legal
    } else if let (DragDropSubject::Plug(drag_proc), Some(onto_input)) =
        (drag_from, drop_onto.as_input())
    {
        // If dragging a processor plug onto an input socket, connect it

        graph
            .with_sound_input_mut(onto_input, |input| input.set_target(Some(drag_proc)))
            .unwrap();

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
            let proc_below = input.processor();

            if layout.is_top_of_group(proc_below) {
                for proc in processors {
                    layout.insert_processor_above(proc, proc_below);
                    // TODO: move the group?
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

// wrapper struct to make keys Stashable
struct AvailableDropSites<'a>(&'a HashMap<DragDropSubject, egui::Rect>);

impl<'a> Stashable for AvailableDropSites<'a> {
    fn stash(&self, stasher: &mut Stasher) {
        stasher.array_of_proxy_objects(
            self.0.keys(),
            |v, stasher| stasher.object(v),
            Order::Unordered,
        );
    }
}

// wrapper struct to stash just the topology of StackedLayout,
// not its positions, to avoid redundant work when changes in
// on-screen positions wouldn't make a difference
struct StackedLayoutWrapper<'a>(&'a StackedLayout);

impl<'a> Stashable for StackedLayoutWrapper<'a> {
    fn stash(&self, stasher: &mut Stasher) {
        stasher.array_of_proxy_objects(
            self.0.groups().iter(),
            |group, stasher| {
                stasher.array_of_u64_iter(group.processors().iter().map(|i| i.value() as u64));
            },
            Order::Unordered,
        );
    }
}

fn compute_legal_drop_sites(
    graph: &SoundGraph,
    layout: &StackedLayoutWrapper,
    drag_subject: DragDropSubject,
    drop_sites: AvailableDropSites,
    stash: &Stash,
    factories: &Factories,
) -> HashMap<DragDropSubject, DragDropLegality> {
    debug_assert_eq!(graph.validate(), Ok(()));
    let mut site_statuses = HashMap::new();
    for drop_site in drop_sites.0.keys() {
        let (mut graph_clone, _) = stash_clone_with_context(
            graph,
            stash,
            StashingContext::new_stashing_normally(),
            UnstashingContext::new(factories.sound_objects(), factories.expression_objects()),
        )
        .unwrap();

        // drag_and_drop_in_graph only does superficial error
        // detection, here we additionally check whether the
        // resulting graph is valid.
        let status =
            match drag_and_drop_in_graph(&mut graph_clone, layout.0, drag_subject, *drop_site) {
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

fn find_closest_legal_drop_site(
    rect: egui::Rect,
    positions: &SoundObjectPositions,
    min_overlap: f32,
    legal_sites: &HashMap<DragDropSubject, DragDropLegality>,
) -> Option<DragDropSubject> {
    let mut best_overlap = min_overlap;
    let mut best_subject = None;

    for (subject, subject_rect) in positions.drag_drop_subjects() {
        if legal_sites.get(subject).unwrap().clone() != DragDropLegality::Legal {
            continue;
        }

        let intersection = subject_rect.intersect(rect);
        if !intersection.is_positive() {
            continue;
        }
        let area = intersection.area();
        if area > best_overlap {
            best_overlap = area;
            best_subject = Some(*subject);
        }
    }

    best_subject
}

const MIN_DROP_OVERLAP: f32 = 1000.0;

pub struct DragInteraction {
    subject: DragDropSubject,
    rect: egui::Rect,
    original_rect: egui::Rect,
    legal_drop_sites: HashCacheProperty<HashMap<DragDropSubject, DragDropLegality>>,
    closest_legal_site: Option<DragDropSubject>,
}

impl DragInteraction {
    pub(crate) fn new(subject: DragDropSubject, original_rect: egui::Rect) -> DragInteraction {
        DragInteraction {
            subject,
            rect: original_rect,
            original_rect,
            legal_drop_sites: HashCacheProperty::new(),
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
        stash: &Stash,
        factories: &Factories,
    ) {
        // Ensure that the legal connections are up to date, since these are used
        // to highlight legal/illegal interconnects to drop onto
        // NOTE: all Stashable implementations in the ui use StashingContext
        // even though it's irrelevant to them, all because this method uses
        // a shared context type
        self.legal_drop_sites.refresh4(
            |a, b, c, d| compute_legal_drop_sites(a, b, c, d, stash, factories),
            graph,
            &StackedLayoutWrapper(layout),
            self.subject,
            AvailableDropSites(positions.drag_drop_subjects()),
        );

        self.closest_legal_site = find_closest_legal_drop_site(
            self.rect,
            positions,
            MIN_DROP_OVERLAP,
            self.legal_drop_sites.get_cached().unwrap(),
        );

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
                positions
                    .drag_drop_subjects()
                    .get(drop_site)
                    .unwrap()
                    .clone(),
                egui::Rounding::same(2.0),
                color,
            );
        }
        // Draw a basic coloured rectangle tracking the cursor as a preview of
        // the subject being dragged, without drawing its ui twice
        let drag_subject_processor = match self.subject {
            DragDropSubject::Processor(spid) => Some(spid),
            DragDropSubject::Plug(spid) => Some(spid),
            DragDropSubject::Socket(siid) => Some(siid.processor()),
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
        stash: &Stash,
        factories: &Factories,
        snapshot_flag: &SnapshotFlag,
    ) {
        let nearest_drop_site =
            find_closest_legal_drop_site(self.rect, positions, MIN_DROP_OVERLAP, &self.legal_sites);

        if let Some(nearest_drop_site) = nearest_drop_site {
            // No point in checking invariants later if they aren't
            // already upheld
            #[cfg(debug_assertions)]
            assert!(layout.check_invariants(graph));

            let drag_and_drop_result = graph.try_make_change(
                stash,
                factories.sound_objects(),
                factories.expression_objects(),
                |graph| {
                    Ok(drag_and_drop_in_graph(
                        graph,
                        layout,
                        self.subject,
                        nearest_drop_site,
                    ))
                },
            );

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

            drag_and_drop_in_layout(layout, graph, self.subject, nearest_drop_site, positions);

            snapshot_flag.request_snapshot();

            #[cfg(debug_assertions)]
            assert!(layout.check_invariants(graph));
        } else {
            // If a processor was dropped far away from anything, split
            // it into its own group
            if let DragDropSubject::Processor(spid) = self.subject {
                if !layout.is_processor_alone(spid) {
                    graph
                        .try_make_change(
                            stash,
                            factories.sound_objects(),
                            factories.expression_objects(),
                            |graph| {
                                disconnect_processor_in_graph(spid, graph);
                                Ok(())
                            },
                        )
                        .unwrap();

                    layout.split_processor_into_own_group(spid, positions);

                    snapshot_flag.request_snapshot();

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
                group.translate(self.rect.left_top() - self.original_rect.left_top());
                snapshot_flag.request_snapshot();
            }
        }
    }

    pub(crate) fn is_valid(&self, graph: &SoundGraph) -> bool {
        self.subject.is_valid(graph)
    }
}

// TODO: move these methods to soundgraph.rs?

fn disconnect_all_inputs_of_processor(processor_id: SoundProcessorId, graph: &mut SoundGraph) {
    graph
        .sound_processor_mut(processor_id)
        .unwrap()
        .foreach_input_mut(|input, _| input.set_target(None));
}

fn disconnect_processor_from_all_inputs(processor_id: SoundProcessorId, graph: &mut SoundGraph) {
    for i in graph.inputs_connected_to(processor_id) {
        graph.disconnect_sound_input(i).unwrap();
    }
}

fn disconnect_processor_in_graph(processor_id: SoundProcessorId, graph: &mut SoundGraph) {
    disconnect_all_inputs_of_processor(processor_id, graph);
    disconnect_processor_from_all_inputs(processor_id, graph);
}
