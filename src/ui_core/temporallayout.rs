use std::collections::{HashMap, HashSet};

use crate::core::{
    samplefrequency::SAMPLE_FREQUENCY,
    sound::{
        soundgraphid::{SoundGraphId, SoundObjectId},
        soundgraphtopology::SoundGraphTopology,
        soundprocessor::SoundProcessorId,
    },
};

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
    top_level_objects: HashMap<SoundObjectId, TopLevelLayout>,
}
impl TemporalLayout {
    const DEFAULT_WIDTH: usize = 600;

    pub(crate) fn new() -> TemporalLayout {
        TemporalLayout {
            top_level_objects: HashMap::new(),
        }
    }

    pub(crate) fn is_top_level(&self, object_id: SoundObjectId) -> bool {
        self.top_level_objects.contains_key(&object_id)
    }

    pub(crate) fn find_top_level_layout(
        &self,
        object_id: SoundObjectId,
    ) -> Option<&TopLevelLayout> {
        self.top_level_objects.get(&object_id)
    }

    pub(crate) fn create_top_level_layout(&mut self, object_id: SoundObjectId) {
        self.top_level_objects.insert(
            object_id,
            TopLevelLayout {
                width_pixels: Self::DEFAULT_WIDTH,
                time_axis: TimeAxis {
                    samples_per_x_pixel: SAMPLE_FREQUENCY as f32 / Self::DEFAULT_WIDTH as f32,
                },
                // nesting depth will be recomputed later
                nesting_depth: 0,
            },
        );
    }

    pub(crate) fn remove_top_level_layout(&mut self, object_id: SoundObjectId) {
        self.top_level_objects.remove(&object_id);
    }

    pub(crate) fn regenerate(&mut self, topo: &SoundGraphTopology) {
        let mut dependent_counts: HashMap<SoundProcessorId, usize> =
            topo.sound_processors().keys().map(|k| (*k, 0)).collect();

        for si in topo.sound_inputs().values() {
            if let Some(spid) = si.target() {
                *dependent_counts.entry(spid).or_insert(0) += 1;
            }
        }

        for (spid, n_deps) in &dependent_counts {
            if *n_deps == 1 {
                continue;
            }
            self.create_top_level_layout(spid.into());
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
                SoundObjectId::Sound(spid) => {
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
