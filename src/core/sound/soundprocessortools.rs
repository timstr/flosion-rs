use std::sync::Arc;

use crate::core::{
    jit::wrappers::{ArrayReadFunc, ScalarReadFunc},
    sound::soundnumbersource::InputTimeNumberSource,
};

use super::{
    soundgraph::SoundGraphIdGenerators,
    soundgraphdata::{
        SoundInputData, SoundNumberInputData, SoundNumberInputScope, SoundNumberSourceData,
    },
    soundgraphtopology::SoundGraphTopology,
    soundinput::{InputOptions, SoundInputId},
    soundnumberinput::SoundNumberInputHandle,
    soundnumbersource::{
        ArrayInputNumberSource, ArrayProcessorNumberSource, ProcessorLocalArrayNumberSource,
        ScalarInputNumberSource, ScalarProcessorNumberSource, SoundNumberSource,
        SoundNumberSourceHandle, SoundNumberSourceId, SoundNumberSourceOwner,
    },
    soundprocessor::SoundProcessorId,
};

/// An interface for making changes to the sound graph from the view of
/// a single sound processor. Changes to the topology of a single
/// processor, such as adding and removing sound inputs and modifying
/// number sources and inputs, can be done through SoundProcessorTools.
/// This is largely for convenience, since doing the same changes through
/// the SoundGraph interface directly would mean to pass around the
/// processor's id and thus be juggling even more ids at once.
pub struct SoundProcessorTools<'a> {
    processor_id: SoundProcessorId,
    topology: &'a mut SoundGraphTopology,
    id_generators: &'a mut SoundGraphIdGenerators,
}

impl<'a> SoundProcessorTools<'a> {
    /// Construct a new tools instance from a mutable topology
    /// instance (for modifying the graph) and set of id generators
    /// (for allocating ids to any newly-created objects)
    pub(crate) fn new(
        id: SoundProcessorId,
        topology: &'a mut SoundGraphTopology,
        id_generators: &'a mut SoundGraphIdGenerators,
    ) -> SoundProcessorTools<'a> {
        SoundProcessorTools {
            processor_id: id,
            topology,
            id_generators,
        }
    }

    /// Add a sound input to the sound processor with the given
    /// options and number of keys. Usually you do not want to
    /// call this directly when creating a processor instance,
    /// and instead will want to use concrete sound input types
    /// which call this method internally as needed.
    pub fn add_sound_input(&mut self, options: InputOptions, num_keys: usize) -> SoundInputId {
        let id = self.id_generators.sound_input.next_id();

        let time_data = SoundNumberSourceData::new(
            self.id_generators.number_source.next_id(),
            Arc::new(InputTimeNumberSource::new(id)),
            SoundNumberSourceOwner::SoundInput(id),
        );

        let input_data =
            SoundInputData::new(id, options, num_keys, self.processor_id, time_data.id());

        self.topology.add_sound_input(input_data).unwrap();

        self.topology.add_number_source(time_data).unwrap();

        id
    }

    /// Remove a sound input from the sound processor
    pub fn remove_sound_input(&mut self, input_id: SoundInputId, owner: SoundProcessorId) {
        // TODO: wtf what is 'owner' doing here? Is it every possibly separate from self.processor_id?
        debug_assert!(owner == self.processor_id, "Huh.....");
        // TODO: also remove the input's number sources?
        self.topology.remove_sound_input(input_id, owner).unwrap();
    }

    /// Add a number source to the given sound input which
    /// reads a single scalar value from the sound input's state
    /// using the provided function.
    pub fn add_input_scalar_number_source(
        &mut self,
        input_id: SoundInputId,
        function: ScalarReadFunc,
    ) -> SoundNumberSourceHandle {
        self.add_input_number_source_helper(
            input_id,
            Arc::new(ScalarInputNumberSource::new(input_id, function)),
        )
    }

    /// Add a number source which reads an entire array from the
    /// given sound input's state using the provided function.
    /// NOTE that currently the length of that array must match
    /// the chunk length.
    pub fn add_input_array_number_source(
        &mut self,
        input_id: SoundInputId,
        function: ArrayReadFunc,
    ) -> SoundNumberSourceHandle {
        self.add_input_number_source_helper(
            input_id,
            Arc::new(ArrayInputNumberSource::new(input_id, function)),
        )
    }

    /// Add a number source which reads a single scalar value
    /// from the processor's state using the given function.
    pub fn add_processor_scalar_number_source(
        &mut self,
        function: ScalarReadFunc,
    ) -> SoundNumberSourceHandle {
        self.add_processor_number_source_helper(|spid, _| {
            Arc::new(ScalarProcessorNumberSource::new(spid, function))
        })
    }

    /// Add a number source which reads an entire array from the
    /// sound processor's state using the provided function.
    /// NOTE that currently the length of that array must match
    /// the chunk length.
    pub fn add_processor_array_number_source(
        &mut self,
        function: ArrayReadFunc,
    ) -> SoundNumberSourceHandle {
        self.add_processor_number_source_helper(|spid, _| {
            Arc::new(ArrayProcessorNumberSource::new(spid, function))
        })
    }

    /// Add a number source which reads an entire array that
    /// is local to the sound processor's audio processing
    /// routine and must be provided with Context::push_processor_state.
    /// NOTE that currently the length of that array must match
    /// the chunk length.
    pub fn add_local_array_number_source(&mut self) -> SoundNumberSourceHandle {
        self.add_processor_number_source_helper(|spid, id| {
            Arc::new(ProcessorLocalArrayNumberSource::new(id, spid))
        })
    }

    /// Add a number input to the sound processor.
    /// When compiled, that number input can be
    /// executed directly on the audio thread
    /// to compute the result. See `make_number_inputs`
    pub fn add_number_input(
        &mut self,
        default_value: f32,
        scope: SoundNumberInputScope,
    ) -> SoundNumberInputHandle {
        let id = self.id_generators.number_input.next_id();

        let data = SoundNumberInputData::new(id, self.processor_id, default_value, scope.clone());
        self.topology.add_number_input(data).unwrap();

        SoundNumberInputHandle::new(id, self.processor_id, scope)
    }

    /// The id of the sound processor that the tools were created for
    pub(super) fn processor_id(&self) -> SoundProcessorId {
        self.processor_id
    }

    /// Internal helper method for adding a number source to a sound input
    fn add_input_number_source_helper(
        &mut self,
        input_id: SoundInputId,
        instance: Arc<dyn SoundNumberSource>,
    ) -> SoundNumberSourceHandle {
        assert!(
            self.topology.sound_input(input_id).unwrap().owner() == self.processor_id,
            "The sound input should belong to the processor which the tools were created for"
        );

        let id = self.id_generators.number_source.next_id();

        let data =
            SoundNumberSourceData::new(id, instance, SoundNumberSourceOwner::SoundInput(input_id));

        self.topology.add_number_source(data).unwrap();

        SoundNumberSourceHandle::new(id)
    }

    /// Internal helper method for adding a number source to the processor
    fn add_processor_number_source_helper<
        F: FnOnce(SoundProcessorId, SoundNumberSourceId) -> Arc<dyn SoundNumberSource>,
    >(
        &mut self,
        f: F,
    ) -> SoundNumberSourceHandle {
        let id = self.id_generators.number_source.next_id();

        let data = SoundNumberSourceData::new(
            id,
            f(self.processor_id, id),
            SoundNumberSourceOwner::SoundProcessor(self.processor_id),
        );

        self.topology.add_number_source(data).unwrap();

        SoundNumberSourceHandle::new(id)
    }
}
