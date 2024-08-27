use std::{collections::HashMap, hash::Hasher};

use eframe::egui;
use hashrevise::{Revisable, RevisedProperty, RevisionHash, RevisionHasher};

use crate::{
    core::sound::{
        soundgraph::SoundGraph, soundgraphtopology::SoundGraphTopology,
        soundgraphvalidation::find_sound_error, soundinput::SoundInputId,
        soundprocessor::SoundProcessorId,
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
}

impl DragDropSubject {
    fn as_processor(&self) -> Option<SoundProcessorId> {
        match self {
            DragDropSubject::Processor(spid) => Some(*spid),
            DragDropSubject::Plug(spid) => Some(*spid),
            DragDropSubject::Socket(_) => None,
        }
    }

    fn as_input(&self) -> Option<SoundInputId> {
        match self {
            DragDropSubject::Processor(_) => None,
            DragDropSubject::Plug(_) => None,
            DragDropSubject::Socket(siid) => Some(*siid),
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

fn drag_and_drop_in_graph(
    topo: &mut SoundGraphTopology,
    layout: &StackedLayout,
    drag_from: DragDropSubject,
    drop_onto: DragDropSubject,
) -> DragDropLegality {
    // Disconnect things as needed
    match drag_from {
        DragDropSubject::Processor(spid) => {
            // If dragging a processor, disconnect it from everything
            // (for now)
            disconnect_processor_in_graph(spid, topo);
        }
        DragDropSubject::Plug(_) => {
            // If dragging a processor plug, don't disconnect it
        }
        DragDropSubject::Socket(siid) => {
            // If dragging an input socket, disconnect it if occupied
            if topo.sound_input(siid).unwrap().target().is_some() {
                topo.disconnect_sound_input(siid).unwrap();
            }
        }
    }

    if let (Some(proc), Some(input)) = (drag_from.as_processor(), drop_onto.as_input()) {
        // Dropping a processor onto an input. Connect the two.

        // Disconnect the input if it's occupied
        if topo.sound_input(input).unwrap().target().is_some() {
            // TODO: re-connect the processor below in the layout after?
            // Only if dragging processor and not its plug?
            topo.disconnect_sound_input(input).unwrap();
        }

        // Connect the input to the processor
        topo.connect_sound_input(input, proc).unwrap();

        DragDropLegality::Legal
    } else if let (DragDropSubject::Processor(proc), DragDropSubject::Plug(plug)) =
        (drag_from, drop_onto)
    {
        // Dragging a processor onto a processor plug. Only
        // okay if the plug is at the bottom of a stack and
        // the processor being dragged has exactly one input
        if !layout.is_bottom_of_group(plug) {
            // TODO: soft error, e.g. just ignore this
            return DragDropLegality::Irrelevant;
        }

        // Disconnect any jumpers leaving the bottom processor.
        // Otherwise, the processor would not be able to be
        // inserted at the bottom of the stack.
        let inputs_to_disconnect: Vec<SoundInputId> = topo.sound_processor_targets(plug).collect();
        for siid in inputs_to_disconnect {
            topo.disconnect_sound_input(siid).unwrap();
        }

        let inputs = topo.sound_processor(proc).unwrap().sound_inputs();
        if inputs.len() != 1 {
            return DragDropLegality::Illegal;
        }

        // The input should already have been disconnected.
        // Connect it to the plug.
        topo.connect_sound_input(inputs[0], plug).unwrap();

        DragDropLegality::Legal
    } else if let (Some(input), Some(proc)) = (drag_from.as_input(), drop_onto.as_processor()) {
        // Dragging an input socket onto a processor or its
        // plug. Connect the two.

        // Disconnect the input if it's occupied
        if topo.sound_input(input).unwrap().target().is_some() {
            topo.disconnect_sound_input(input).unwrap();
        }

        // Connect the input to the processor
        topo.connect_sound_input(input, proc).unwrap();

        // Colouring all the processors is visually noisy,
        // and the effect is the same as dropping onto the
        // processor's plug right below it.
        if let DragDropSubject::Plug(_) = drop_onto {
            DragDropLegality::Legal
        } else {
            DragDropLegality::LegalButInvisible
        }
    } else {
        // Dragging and dropping an unsupported combination of things
        DragDropLegality::Irrelevant
    }
}

fn drag_and_drop_in_layout(
    layout: &mut StackedLayout,
    topo: &SoundGraphTopology,
    drag_from: DragDropSubject,
    drop_onto: DragDropSubject,
    positions: &SoundObjectPositions,
) {
    match (drag_from, drop_onto) {
        (DragDropSubject::Processor(proc), DragDropSubject::Socket(input)) => {
            // Dragging a processor onto an input. Move the processor
            // and insert it at the group under the input.
            layout.split_processor_into_own_group(proc, positions);
            let input_data = topo.sound_input(input).unwrap();
            let proc_below = input_data.owner();
            layout.split_group_above_processor(proc_below, positions);
            layout.insert_processor_above(proc, proc_below);
        }
        (DragDropSubject::Processor(proc), DragDropSubject::Plug(plug)) => {
            // Dragging a processor onto the bottom plug of a stacked group
            // (assuming that this is only being called after drag_and_drop_in_graph
            // has deemed it legal). Move the processor to the bottom of the group.
            layout.split_processor_into_own_group(proc, positions);
            layout.insert_processor_below(proc, plug);
        }
        _ => {
            // Otherwise, only plugs/sockets are being dragged, no layout changes
            // are needed that can't be resolved normally by SoundGraphLayout::regenerate
        }
    }

    layout.regenerate(topo, positions);
}

fn compute_legal_drop_sites(
    topo: &SoundGraphTopology,
    layout: &StackedLayout,
    drag_subject: DragDropSubject,
    drop_sites: &[DragDropSubject],
) -> HashMap<DragDropSubject, DragDropLegality> {
    let mut site_statuses = HashMap::new();
    for drop_site in drop_sites {
        let mut topo_clone = topo.clone();
        // drag_and_drop_in_graph only does superficial error
        // detection, here we additionally check whether the
        // resulting topology is valid.
        let status = match drag_and_drop_in_graph(&mut topo_clone, layout, drag_subject, *drop_site)
        {
            DragDropLegality::Legal => {
                if find_sound_error(&topo_clone).is_some() {
                    DragDropLegality::Illegal
                } else {
                    DragDropLegality::Legal
                }
            }
            DragDropLegality::LegalButInvisible => {
                if find_sound_error(&topo_clone).is_some() {
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
        topo: &SoundGraphTopology,
        object_states: &SoundObjectUiStates,
        layout: &StackedLayout,
        positions: &SoundObjectPositions,
    ) {
        // Ensure that the legal connections are up to date, since these are used
        // to highlight legal/illegal interconnects to drop onto
        self.legal_drop_sites.refresh4(
            compute_legal_drop_sites,
            topo,
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
            DragDropSubject::Processor(spid) => spid,
            DragDropSubject::Plug(spid) => spid,
            DragDropSubject::Socket(siid) => topo.sound_input(siid).unwrap().owner(),
        };
        let color = object_states.get_object_color(drag_subject_processor.into());
        let color = egui::Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 64);
        ui.painter()
            .rect_filled(self.rect, egui::Rounding::same(5.0), color);
    }

    pub(crate) fn translate(&mut self, delta: egui::Vec2) {
        self.rect = self.rect.translate(delta);
    }

    pub(crate) fn is_valid(&self, topo: &SoundGraphTopology) -> bool {
        match self.subject {
            DragDropSubject::Processor(spid) => topo.contains(spid),
            DragDropSubject::Plug(spid) => topo.contains(spid),
            DragDropSubject::Socket(siid) => topo.contains(siid),
        }
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
            assert!(layout.check_invariants(graph.topology()));

            let drag_and_drop_result = graph.edit_topology(|topo| {
                Ok(drag_and_drop_in_graph(
                    topo,
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
                    println!("Can't drop that there: {}", e.explain(graph.topology()));
                    return;
                }
            }

            drag_and_drop_in_layout(
                layout,
                graph.topology(),
                self.subject,
                *nearest_drop_site,
                positions,
            );

            #[cfg(debug_assertions)]
            assert!(layout.check_invariants(graph.topology()));
        } else {
            // If a processor was dropped far away from anything, split
            // it into its own group
            if let DragDropSubject::Processor(spid) = self.subject {
                if !layout.is_processor_alone(spid) {
                    graph
                        .edit_topology(|topo| {
                            disconnect_processor_in_graph(spid, topo);
                            Ok(())
                        })
                        .unwrap();

                    layout.split_processor_into_own_group(spid, positions);

                    #[cfg(debug_assertions)]
                    assert!(layout.check_invariants(graph.topology()));
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

    pub(crate) fn is_valid(&self, topo: &SoundGraphTopology) -> bool {
        match self.subject {
            DragDropSubject::Processor(spid) => topo.contains(spid),
            DragDropSubject::Plug(spid) => topo.contains(spid),
            DragDropSubject::Socket(siid) => topo.contains(siid),
        }
    }
}

fn disconnect_processor_in_graph(processor_id: SoundProcessorId, topo: &mut SoundGraphTopology) {
    let mut inputs_to_disconnect_from: Vec<SoundInputId> =
        topo.sound_processor_targets(processor_id).collect();
    for i in topo.sound_processor(processor_id).unwrap().sound_inputs() {
        if topo.sound_input(*i).unwrap().target().is_some() {
            inputs_to_disconnect_from.push(*i);
        }
    }
    for i in inputs_to_disconnect_from {
        topo.disconnect_sound_input(i).unwrap();
    }
}
