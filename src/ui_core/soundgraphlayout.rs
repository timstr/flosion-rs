use std::collections::HashMap;

use eframe::egui;

use crate::core::sound::{
    soundgraphid::SoundObjectId, soundgraphtopology::SoundGraphTopology,
    soundprocessor::SoundProcessorId,
};

/// A mapping between a portion of the sound processing timeline
/// and a spatial region on screen.
#[derive(Clone, Copy)]
pub struct TimeAxis {
    /// How many seconds each horizontal pixel corresponds to
    pub time_per_x_pixel: f32,
    // TODO: offset to allow scrolling?
}

/// The visual representation of a sequency of sound processors,
/// connected end-to-end in a linear fashion. Each processor in
/// the group must have exactly one sound input, with the exception
/// of the top/leaf processor, which may have any number.
pub struct StackedGroup {
    // TODO: why are these pub?
    pub width_pixels: usize,
    pub time_axis: TimeAxis,

    /// The processors in the stacked group, ordered with the
    /// deepest dependency first. The root/bottom processor is
    /// thus the last in the vec.
    processors: Vec<SoundProcessorId>,
    // TODO: on-screen location?
}

impl StackedGroup {
    pub(crate) fn new() {
        todo!()
    }

    pub(crate) fn draw(&self, ui: &mut egui::Ui) {
        todo!()
    }
}

/// Visual layout of all processor groups and the connections between them.
/// Intended to be the entry point of the main UI for all things pertaining
/// to sound processors, their inputs, connections, and numeric UIs.
pub struct SoundGraphLayout {
    /// The set of top-level stacked groups of sound processors
    groups: HashMap<SoundObjectId, StackedGroup>,
}

// TODO: let this render itself to the whole screen
impl SoundGraphLayout {
    const DEFAULT_WIDTH: usize = 600;
    const DEFAULT_DURATION: f32 = 4.0;

    pub(crate) fn new() -> SoundGraphLayout {
        SoundGraphLayout {
            groups: HashMap::new(),
        }
    }

    pub(crate) fn is_top_level(&self, object_id: SoundObjectId) -> bool {
        self.groups.contains_key(&object_id)
    }

    pub(crate) fn find_group(&self, id: SoundObjectId) -> Option<&StackedGroup> {
        // Easy case: object is top-level
        if let Some(g) = self.groups.get(&id) {
            Some(g)
        } else {
            // Otherwise, look for group containing object
            let id = id.as_sound_processor_id().unwrap();
            for (_, g) in &self.groups {
                if g.processors.contains(&id) {
                    return Some(g);
                }
            }
            None
        }
    }

    pub(crate) fn create_single_processor_group(&mut self, object_id: SoundObjectId) {
        self.groups.insert(
            object_id,
            StackedGroup {
                width_pixels: Self::DEFAULT_WIDTH,
                time_axis: TimeAxis {
                    time_per_x_pixel: Self::DEFAULT_DURATION / (Self::DEFAULT_WIDTH as f32),
                },
                processors: vec![object_id.as_sound_processor_id().unwrap()],
            },
        );
    }

    pub(crate) fn remove_single_processor_group(&mut self, object_id: SoundObjectId) {
        let g = self
            .groups
            .remove(&object_id)
            .expect("Group was not present");
        debug_assert_eq!(
            g.processors,
            vec![object_id.as_sound_processor_id().unwrap()],
            "Group did not consist of only the requested processor"
        );
    }

    /// Ensure the layout is up-to-date with respect to the latest
    /// provided sound graph topology by deleted data from unused
    /// processors and automatically generating data for newly-added
    /// processors as needed.
    pub(crate) fn regenerate(&mut self, topo: &SoundGraphTopology) {
        // precompute the number of sound inputs that each processor
        // is connected to
        let mut dependent_counts: HashMap<SoundProcessorId, usize> =
            topo.sound_processors().keys().map(|k| (*k, 0)).collect();

        for si in topo.sound_inputs().values() {
            if let Some(spid) = si.target() {
                *dependent_counts.entry(spid).or_insert(0) += 1;
            }
        }

        // helper function to create a new stacked group, starting
        // with the given root and stacking connected processors
        // in series on top until either a processor is encountered
        // that already is in a group, or a processor with zero or
        // multiple sound inputs is found.
        let insert_and_grow_stacked_group =
            |layout: &mut SoundGraphLayout, root: SoundProcessorId| {
                let mut next_spid = root;

                let mut processors = Vec::new();

                loop {
                    processors.insert(0, next_spid);

                    let inputs = topo.sound_processor(next_spid).unwrap().sound_inputs();

                    if inputs.len() != 1 {
                        break;
                    }

                    let input_target = topo.sound_input(inputs[0]).unwrap().target();

                    let Some(input_target) = input_target else {
                        break;
                    };

                    if layout.groups.contains_key(&input_target.into()) {
                        break;
                    }

                    next_spid = input_target;
                }

                layout.groups.insert(
                    root.into(),
                    StackedGroup {
                        width_pixels: Self::DEFAULT_WIDTH,
                        time_axis: TimeAxis {
                            time_per_x_pixel: Self::DEFAULT_DURATION / (Self::DEFAULT_WIDTH as f32),
                        },
                        processors,
                    },
                );
            };

        // Find all sound processors that don't have exactly one
        // sound input they're connected to and turn them into
        // the roots of new stacked groups
        for (spid, n_deps) in &dependent_counts {
            if *n_deps == 1 {
                continue;
            }

            insert_and_grow_stacked_group(self, *spid);
        }

        // Find any remaining processors and create groups
        // for them as needed. Processors here would have
        // been excluded earlier because of a connection
        // through one of multiple sound inputs, which
        // terminates a stack.
        loop {
            let processor_without_group = topo
                .sound_processors()
                .keys()
                .filter(|spid| !self.groups.contains_key(&(*spid).into()))
                .cloned()
                .next();

            let Some(spid) = processor_without_group else {
                break;
            };

            insert_and_grow_stacked_group(self, spid);
        }
    }

    pub(crate) fn draw(&self, ui: &mut egui::Ui) {
        for group in self.groups.values() {
            group.draw(ui);
        }
    }

    /// Remove any data associated with sound graph objects that
    /// no longer exist according to the given topology
    pub(crate) fn cleanup(&mut self, topo: &SoundGraphTopology) {
        self.groups.retain(|k, _v| topo.contains((*k).into()));
    }

    #[cfg(debug_assertions)]
    pub(crate) fn check_invariants(&self, topo: &SoundGraphTopology) -> bool {
        for spid in topo.sound_processors().keys().cloned() {
            let number_of_appearances: usize = self
                .groups
                .values()
                .map(|group| group.processors.iter().filter(|i| **i == spid).count())
                .sum();

            if number_of_appearances != 1 {
                return false;
            }
        }

        true
    }
}
