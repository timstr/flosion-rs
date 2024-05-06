use std::collections::{HashMap, HashSet};

use crate::core::sound::{
    soundgraphid::{SoundGraphId, SoundObjectId},
    soundgraphtopology::SoundGraphTopology,
    soundgraphvalidation::available_sound_number_sources,
    soundnumberinput::SoundNumberInputId,
    soundnumbersource::SoundNumberSourceId,
    soundprocessor::SoundProcessorId,
};

#[derive(Clone, Copy)]
pub struct TimeAxis {
    pub time_per_x_pixel: f32,
    // TODO: offset to allow scrolling?
}

// TODO: rename.
pub struct TopLevelLayout {
    pub width_pixels: usize,
    pub time_axis: TimeAxis,
    // TODO: stack of processors?
}

pub struct TemporalLayout {
    top_level_objects: HashMap<SoundObjectId, TopLevelLayout>,
    // TODO: this caches which number depedencies are possible. It has nothing
    // to do with the UI layout and shouldn't be here.
    available_number_sources: HashMap<SoundNumberInputId, HashSet<SoundNumberSourceId>>,
}

impl TemporalLayout {
    const DEFAULT_WIDTH: usize = 600;
    const DEFAULT_DURATION: f32 = 4.0;

    pub(crate) fn new() -> TemporalLayout {
        TemporalLayout {
            top_level_objects: HashMap::new(),
            available_number_sources: HashMap::new(),
        }
    }

    pub(crate) fn is_top_level(&self, object_id: SoundObjectId) -> bool {
        self.top_level_objects.contains_key(&object_id)
    }

    pub(crate) fn find_layout(
        &self,
        id: SoundGraphId,
        topo: &SoundGraphTopology,
    ) -> Option<&TopLevelLayout> {
        // TODO:
        // - find the top-level stack containing the object
        // - return its layout
        todo!()
    }

    pub(crate) fn create_top_level_layout(&mut self, object_id: SoundObjectId) {
        self.top_level_objects.insert(
            object_id,
            TopLevelLayout {
                width_pixels: Self::DEFAULT_WIDTH,
                time_axis: TimeAxis {
                    time_per_x_pixel: Self::DEFAULT_DURATION / (Self::DEFAULT_WIDTH as f32),
                },
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
    }

    pub(crate) fn cleanup(&mut self, topo: &SoundGraphTopology) {
        self.top_level_objects
            .retain(|k, _v| topo.contains((*k).into()));

        self.available_number_sources = available_sound_number_sources(topo);
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
        input_id: SoundNumberInputId,
    ) -> &HashSet<SoundNumberSourceId> {
        self.available_number_sources.get(&input_id).unwrap()
    }
}
