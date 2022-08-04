use std::sync::Arc;

use super::{
    numberinput::{NumberInputHandle, NumberInputOwner},
    numbersource::{
        NumberSource, NumberSourceOwner, ProcessorNumberSource, ProcessorTimeNumberSource,
        StateFunction, StateNumberSourceHandle,
    },
    soundgraphtopology::SoundGraphTopology,
    soundinput::{InputOptions, SoundInputId},
    soundprocessor::{SoundProcessor, SoundProcessorId},
};

pub struct SoundProcessorTools<'a> {
    processor_id: SoundProcessorId,
    topology: &'a mut SoundGraphTopology,
}

impl<'a> SoundProcessorTools<'a> {
    pub(super) fn new(
        id: SoundProcessorId,
        topology: &'a mut SoundGraphTopology,
    ) -> SoundProcessorTools<'a> {
        SoundProcessorTools {
            processor_id: id,
            topology,
        }
    }

    pub(super) fn add_sound_input(
        &mut self,
        options: InputOptions,
        num_keys: usize,
    ) -> SoundInputId {
        self.topology
            .add_sound_input(self.processor_id, options, num_keys)
    }

    pub(super) fn remove_sound_input(&mut self, input_id: SoundInputId) {
        self.topology.remove_sound_input(input_id);
    }

    pub fn add_processor_number_source<T: SoundProcessor, F: StateFunction<T::StateType>>(
        &mut self,
        function: F,
    ) -> StateNumberSourceHandle {
        let source = Arc::new(ProcessorNumberSource::new(self.processor_id, function));
        self.topology
            .add_state_number_source(source, NumberSourceOwner::SoundProcessor(self.processor_id))
    }

    pub(super) fn add_input_number_source(
        &mut self,
        input_id: SoundInputId,
        source: Arc<dyn NumberSource>,
    ) -> StateNumberSourceHandle {
        self.topology
            .add_state_number_source(source, NumberSourceOwner::SoundInput(input_id))
    }

    pub fn add_number_input(&mut self) -> NumberInputHandle {
        self.topology
            .add_number_input(NumberInputOwner::SoundProcessor(self.processor_id))
    }

    pub fn add_processor_time(&mut self) -> StateNumberSourceHandle {
        let source = Arc::new(ProcessorTimeNumberSource::new(self.processor_id));
        self.topology
            .add_state_number_source(source, NumberSourceOwner::SoundProcessor(self.processor_id))
    }
}
