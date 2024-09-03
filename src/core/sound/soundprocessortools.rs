use std::rc::Rc;

use crate::core::jit::wrappers::{ArrayReadFunc, ScalarReadFunc};

use super::{
    expression::SoundExpressionHandle,
    expressionargument::{
        ArrayInputExpressionArgument, ArrayProcessorExpressionArgument,
        ProcessorLocalArrayExpressionArgument, ScalarInputExpressionArgument,
        ScalarProcessorExpressionArgument, SoundExpressionArgument, SoundExpressionArgumentHandle,
        SoundExpressionArgumentId, SoundExpressionArgumentOwner,
    },
    sounderror::SoundError,
    soundgraphdata::{
        SoundExpressionArgumentData, SoundExpressionData, SoundExpressionScope, SoundInputBranchId,
    },
    soundgraphtopology::SoundGraphTopology,
    soundinput::{InputOptions, SoundInputId},
    soundprocessor::SoundProcessorId,
    topologyedits::{build_sound_input, SoundGraphIdGenerators},
};

/// An interface for making changes to the sound graph from the view of
/// a single sound processor. Changes to the topology of a single
/// processor, such as adding and removing sound inputs and modifying
/// expressions and expression arguments, can be done through SoundProcessorTools.
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
    /// options and list of branches. Usually you do not want to
    /// call this directly when creating a processor instance,
    /// and instead will want to use concrete sound input types
    /// which call this method internally as needed.
    pub fn add_sound_input(
        &mut self,
        options: InputOptions,
        branches: Vec<SoundInputBranchId>,
    ) -> SoundInputId {
        build_sound_input(
            self.topology,
            self.id_generators,
            self.processor_id,
            options,
            branches,
        )
    }

    /// Remove a sound input from the sound processor
    pub fn remove_sound_input(&mut self, input_id: SoundInputId) -> Result<(), SoundError> {
        let input = self
            .topology
            .sound_input(input_id)
            .ok_or(SoundError::SoundInputNotFound(input_id))?;

        let has_target = input.target().is_some();

        let args = input.arguments().to_vec();

        if has_target {
            self.topology.disconnect_sound_input(input_id).unwrap();
        }

        for arg in args {
            self.topology.remove_expression_argument(arg).unwrap();
        }

        self.topology
            .remove_sound_input(input_id, self.processor_id)
            .unwrap();
        Ok(())
    }

    /// Add an expression argument to the given sound input which
    /// reads a single scalar value from the sound input's state
    /// using the provided function.
    pub fn add_input_scalar_argument(
        &mut self,
        input_id: SoundInputId,
        function: ScalarReadFunc,
    ) -> SoundExpressionArgumentHandle {
        self.add_input_argument_helper(
            input_id,
            Rc::new(ScalarInputExpressionArgument::new(input_id, function)),
        )
    }

    /// Add an expression argument which reads an entire array from the
    /// given sound input's state using the provided function.
    /// NOTE that currently the length of that array must match
    /// the chunk length.
    pub fn add_input_array_argument(
        &mut self,
        input_id: SoundInputId,
        function: ArrayReadFunc,
    ) -> SoundExpressionArgumentHandle {
        self.add_input_argument_helper(
            input_id,
            Rc::new(ArrayInputExpressionArgument::new(input_id, function)),
        )
    }

    /// Add an expression argument which reads a single scalar value
    /// from the processor's state using the given function.
    pub fn add_processor_scalar_argument(
        &mut self,
        function: ScalarReadFunc,
    ) -> SoundExpressionArgumentHandle {
        self.add_processor_argument_helper(|spid, _| {
            Rc::new(ScalarProcessorExpressionArgument::new(spid, function))
        })
    }

    /// Add an expression argument which reads an entire array from the
    /// sound processor's state using the provided function.
    /// NOTE that currently the length of that array must match
    /// the chunk length.
    pub fn add_processor_array_argument(
        &mut self,
        function: ArrayReadFunc,
    ) -> SoundExpressionArgumentHandle {
        self.add_processor_argument_helper(|spid, _| {
            Rc::new(ArrayProcessorExpressionArgument::new(spid, function))
        })
    }

    /// Add an expression argument which reads an entire array that
    /// is local to the sound processor's audio processing
    /// routine and must be provided with Context::push_processor_state.
    /// NOTE that currently the length of that array must match
    /// the chunk length.
    pub fn add_local_array_argument(&mut self) -> SoundExpressionArgumentHandle {
        self.add_processor_argument_helper(|spid, id| {
            Rc::new(ProcessorLocalArrayExpressionArgument::new(id, spid))
        })
    }

    /// Add an expression to the sound processor.
    /// When compiled, that expression can be
    /// executed directly on the audio thread
    /// to compute the result. See `make_expression_nodes`
    pub fn add_expression(
        &mut self,
        default_value: f32,
        scope: SoundExpressionScope,
    ) -> SoundExpressionHandle {
        let id = self.id_generators.expression.next_id();

        let data = SoundExpressionData::new(id, self.processor_id, default_value, scope.clone());
        self.topology.add_expression(data).unwrap();

        SoundExpressionHandle::new(id, self.processor_id, scope)
    }

    /// The id of the sound processor that the tools were created for
    pub(crate) fn processor_id(&self) -> SoundProcessorId {
        self.processor_id
    }

    /// Access the current sound graph topology
    pub(crate) fn topology(&self) -> &SoundGraphTopology {
        self.topology
    }

    /// Mutably access the current sound graph topology
    pub(crate) fn topology_mut(&mut self) -> &mut SoundGraphTopology {
        self.topology
    }

    /// Internal helper method for adding an expression argument to a sound input
    /// belonging to the sound processor that the tools were created for.
    fn add_input_argument_helper(
        &mut self,
        input_id: SoundInputId,
        instance: Rc<dyn SoundExpressionArgument>,
    ) -> SoundExpressionArgumentHandle {
        assert!(
            self.topology.sound_input(input_id).unwrap().owner() == self.processor_id,
            "The sound input should belong to the processor which the tools were created for"
        );

        let id = self.id_generators.expression_argument.next_id();

        let data = SoundExpressionArgumentData::new(
            id,
            instance,
            SoundExpressionArgumentOwner::SoundInput(input_id),
        );

        self.topology.add_expression_argument(data).unwrap();

        SoundExpressionArgumentHandle::new(id)
    }

    /// Internal helper method for adding an expression to the sound processor
    /// which the tools were created for.
    fn add_processor_argument_helper<
        F: FnOnce(SoundProcessorId, SoundExpressionArgumentId) -> Rc<dyn SoundExpressionArgument>,
    >(
        &mut self,
        f: F,
    ) -> SoundExpressionArgumentHandle {
        let id = self.id_generators.expression_argument.next_id();

        let data = SoundExpressionArgumentData::new(
            id,
            f(self.processor_id, id),
            SoundExpressionArgumentOwner::SoundProcessor(self.processor_id),
        );

        self.topology.add_expression_argument(data).unwrap();

        SoundExpressionArgumentHandle::new(id)
    }
}
