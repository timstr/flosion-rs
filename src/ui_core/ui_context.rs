use std::collections::{HashMap, HashSet};

use crate::core::{
    graphobject::{ObjectId, SoundGraphId},
    samplefrequency::SAMPLE_FREQUENCY,
    soundgraphtopology::SoundGraphTopology,
    soundprocessor::SoundProcessorId,
};

use super::{object_ui_states::ObjectUiStates, ui_factory::UiFactory};

#[derive(Clone, Copy)]
pub struct TimeAxis {
    pub samples_per_x_pixel: f32,
    // TODO: offset to allow scrolling?
}

pub struct TopLevelLayout {
    pub width_pixels: usize,
    pub time_axis: TimeAxis,
    pub nesting_depth: usize,
}

pub struct TemporalLayout {
    top_level_objects: HashMap<ObjectId, TopLevelLayout>,
}
impl TemporalLayout {
    pub(crate) fn new() -> TemporalLayout {
        TemporalLayout {
            top_level_objects: HashMap::new(),
        }
    }

    pub(crate) fn find_top_level_layout(&self, object_id: ObjectId) -> Option<&TopLevelLayout> {
        self.top_level_objects.get(&object_id)
    }

    pub(crate) fn regenerate(&mut self, topo: &SoundGraphTopology) {
        // TODO:
        // - compute all dependent counts for all processors
        // - create new top-level layouts for those processors with zero or multiple dependents
        // - recompute the nesting depths of all top-level layouts recursively, increasing for
        //   each transitive input until one calls upon a processor with zero or multiple dependents

        let mut dependent_counts: HashMap<SoundProcessorId, usize> =
            topo.sound_processors().keys().map(|k| (*k, 0)).collect();

        for si in topo.sound_inputs().values() {
            if let Some(spid) = si.target() {
                *dependent_counts.entry(spid).or_insert(0) += 1;
            }
        }

        const DEFAULT_WIDTH: usize = 300;

        for (spid, n_deps) in &dependent_counts {
            if *n_deps == 1 {
                continue;
            }
            self.top_level_objects
                .entry((*spid).into())
                .or_insert_with(|| TopLevelLayout {
                    width_pixels: DEFAULT_WIDTH,
                    time_axis: TimeAxis {
                        samples_per_x_pixel: SAMPLE_FREQUENCY as f32 / DEFAULT_WIDTH as f32,
                    },
                    nesting_depth: 0,
                });
        }

        fn count_nesting_depth(
            spid: SoundProcessorId,
            dependent_counts: &HashMap<SoundProcessorId, usize>,
            topo: &SoundGraphTopology,
        ) -> usize {
            let inputs = topo.sound_processor(spid).unwrap().sound_inputs();
            let mut max_depth = 0;
            for siid in inputs {
                if let Some(t_sp) = topo.sound_input(*siid).unwrap().target() {
                    let d = 1 + count_nesting_depth(t_sp, dependent_counts, topo);
                    max_depth = max_depth.max(d);
                }
            }
            max_depth
        }

        for (oid, layout) in &mut self.top_level_objects {
            match *oid {
                ObjectId::Sound(spid) => {
                    layout.nesting_depth = count_nesting_depth(spid, &dependent_counts, topo);
                }
            }
        }
    }

    pub(crate) fn retain(&mut self, remaining_ids: &HashSet<SoundGraphId>) {
        self.top_level_objects
            .retain(|k, _v| remaining_ids.contains(&(*k).into()));
    }
}

pub struct UiContext<'a> {
    ui_factory: &'a UiFactory,
    object_states: &'a ObjectUiStates,
    topology: &'a SoundGraphTopology,
    is_top_level: bool,
    time_axis: TimeAxis,
    width: f32,
    nesting_depth: usize,
}

impl<'a> UiContext<'a> {
    pub(crate) fn new(
        ui_factory: &'a UiFactory,
        object_states: &'a ObjectUiStates,
        topology: &'a SoundGraphTopology,
        is_top_level: bool,
        time_axis: TimeAxis,
        width: f32,
        nesting_depth: usize,
    ) -> UiContext<'a> {
        UiContext {
            ui_factory,
            object_states,
            topology,
            is_top_level,
            time_axis,
            width,
            nesting_depth,
        }
    }

    pub(crate) fn ui_factory(&self) -> &UiFactory {
        self.ui_factory
    }

    pub(crate) fn object_states(&self) -> &ObjectUiStates {
        self.object_states
    }

    pub(crate) fn topology(&self) -> &SoundGraphTopology {
        self.topology
    }

    pub fn time_axis(&self) -> &TimeAxis {
        &self.time_axis
    }

    pub fn width(&self) -> f32 {
        self.width
    }

    pub fn is_top_level(&self) -> bool {
        self.is_top_level
    }

    pub(crate) fn nest(&self, new_width: f32) -> UiContext {
        UiContext {
            ui_factory: self.ui_factory,
            object_states: self.object_states,
            topology: self.topology,
            is_top_level: false,
            time_axis: self.time_axis,
            width: new_width,
            nesting_depth: self.nesting_depth - 1,
        }
    }

    pub(crate) fn nesting_depth(&self) -> usize {
        self.nesting_depth
    }
}
