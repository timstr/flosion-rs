use std::collections::{HashMap, HashSet};

use crate::core::{
    samplefrequency::SAMPLE_FREQUENCY,
    sound::{
        soundgraphid::{SoundGraphId, SoundObjectId},
        soundgraphtopology::SoundGraphTopology,
        soundgraphvalidation::available_sound_number_sources,
        soundnumbersource::SoundNumberSourceId,
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
    available_number_sources: HashMap<SoundProcessorId, HashSet<SoundNumberSourceId>>,
}

impl TemporalLayout {
    const DEFAULT_WIDTH: usize = 600;

    pub(crate) fn new() -> TemporalLayout {
        TemporalLayout {
            top_level_objects: HashMap::new(),
            available_number_sources: HashMap::new(),
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

    pub(crate) fn cleanup(&mut self, topo: &SoundGraphTopology) {
        self.top_level_objects
            .retain(|k, _v| topo.contains((*k).into()));

        self.available_number_sources = available_sound_number_sources(topo);
    }

    pub(crate) fn find_root_processor(
        &self,
        id: SoundGraphId,
        topo: &SoundGraphTopology,
    ) -> SoundProcessorId {
        match id {
            SoundGraphId::SoundInput(siid) => {
                self.find_root_processor(topo.sound_input(siid).unwrap().owner().into(), topo)
            }
            SoundGraphId::SoundProcessor(spid) => {
                if self.is_top_level(spid.into()) {
                    spid
                } else {
                    let mut target_iter = topo.sound_processor_targets(spid);
                    let target = target_iter.next().unwrap();
                    // A sound processor without a top level layout should be connected
                    // to exactly one sound input
                    debug_assert!(target_iter.next().is_none());
                    self.find_root_processor(target.into(), topo)
                }
            }
            SoundGraphId::SoundNumberInput(sniid) => {
                self.find_root_processor(topo.number_input(sniid).unwrap().owner().into(), topo)
            }
            SoundGraphId::SoundNumberSource(snsid) => {
                self.find_root_processor(topo.number_source(snsid).unwrap().owner().into(), topo)
            }
        }
    }

    pub(crate) fn get_stack_items(
        &self,
        spid: SoundProcessorId,
        topo: &SoundGraphTopology,
    ) -> Vec<SoundGraphId> {
        fn visitor(
            spid: SoundProcessorId,
            temporal_layout: &TemporalLayout,
            topo: &SoundGraphTopology,
            items: &mut Vec<SoundGraphId>,
        ) {
            let sp_data = topo.sound_processor(spid).unwrap();
            for siid in sp_data.sound_inputs() {
                let si_data = topo.sound_input(*siid).unwrap();
                if let Some(target_spid) = si_data.target() {
                    if !temporal_layout.is_top_level(target_spid.into()) {
                        visitor(target_spid, temporal_layout, topo, items);
                    } else {
                        items.push((*siid).into());
                    }
                } else {
                    items.push((*siid).into());
                }
            }
            for niid in sp_data.number_inputs() {
                items.push((*niid).into());
            }
            items.push(spid.into());
        }

        let mut items = Vec::new();
        visitor(spid, self, topo, &mut items);
        items
    }

    pub(super) fn available_number_sources(
        &self,
        processor_id: SoundProcessorId,
    ) -> &HashSet<SoundNumberSourceId> {
        self.available_number_sources.get(&processor_id).unwrap()
    }
}
