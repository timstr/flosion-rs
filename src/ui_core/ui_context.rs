use std::collections::HashMap;

use crate::core::{
    graphobject::ObjectId, samplefrequency::SAMPLE_FREQUENCY,
    soundgraphtopology::SoundGraphTopology,
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

    pub(crate) fn create_state_for(&mut self, object_id: ObjectId, topo: &SoundGraphTopology) {
        // don't add a top-level layout if the object has exactly one dependant
        let num_dependents = match object_id {
            ObjectId::Sound(spid) => topo
                .sound_inputs()
                .values()
                .filter(|d| d.target() == Some(spid))
                .count(),
            ObjectId::Number(nsid) => topo
                .number_inputs()
                .values()
                .filter(|d| d.target() == Some(nsid))
                .count(),
        };
        if num_dependents == 1 {
            return;
        }

        const DEFAULT_WIDTH: usize = 300;

        self.top_level_objects.insert(
            object_id,
            TopLevelLayout {
                width_pixels: DEFAULT_WIDTH,
                time_axis: TimeAxis {
                    samples_per_x_pixel: SAMPLE_FREQUENCY as f32 / DEFAULT_WIDTH as f32,
                },
            },
        );
    }
}

pub struct UiContext<'a> {
    ui_factory: &'a UiFactory,
    object_states: &'a ObjectUiStates,
    topology: &'a SoundGraphTopology,
    is_top_level: bool,
    time_axis: TimeAxis,
    width: usize,
}

impl<'a> UiContext<'a> {
    pub(crate) fn new(
        ui_factory: &'a UiFactory,
        object_states: &'a ObjectUiStates,
        topology: &'a SoundGraphTopology,
        is_top_level: bool,
        time_axis: TimeAxis,
        width: usize,
    ) -> UiContext<'a> {
        UiContext {
            ui_factory,
            object_states,
            topology,
            is_top_level,
            time_axis,
            width,
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

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn is_top_level(&self) -> bool {
        self.is_top_level
    }

    pub fn nest(&self) -> UiContext {
        UiContext {
            ui_factory: self.ui_factory,
            object_states: self.object_states,
            topology: self.topology,
            is_top_level: false,
            time_axis: self.time_axis,
            width: self.width,
        }
    }
}
