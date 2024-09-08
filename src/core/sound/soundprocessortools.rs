use std::rc::Rc;

use crate::core::jit::wrappers::{ArrayReadFunc, ScalarReadFunc};

use super::{
    expression::SoundExpressionHandle,
    expressionargument::{
        ArrayInputExpressionArgument, ArrayProcessorExpressionArgument,
        ProcessorLocalArrayExpressionArgument, ScalarInputExpressionArgument,
        ScalarProcessorExpressionArgument, SoundExpressionArgument, SoundExpressionArgumentHandle,
        SoundExpressionArgumentOwner,
    },
    sounderror::SoundError,
    soundgraph::SoundGraph,
    soundgraphdata::{SoundExpressionScope, SoundInputBranchId},
    soundinput::{InputOptions, SoundInputId},
    soundprocessor::SoundProcessorId,
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
    topology: &'a mut SoundGraph,
}

impl<'a> SoundProcessorTools<'a> {
    /// Construct a new tools instance from a mutable topology
    /// instance (for modifying the graph) and set of id generators
    /// (for allocating ids to any newly-created objects)
    pub(crate) fn new(
        id: SoundProcessorId,
        topology: &'a mut SoundGraph,
    ) -> SoundProcessorTools<'a> {
        SoundProcessorTools {
            processor_id: id,
            topology,
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
        self.topology
            .add_sound_input(self.processor_id, options, branches)
            .unwrap()
    }

    /// Remove a sound input from the sound processor
    pub fn remove_sound_input(&mut self, input_id: SoundInputId) -> Result<(), SoundError> {
        let input_data = self
            .topology
            .sound_input(input_id)
            .ok_or(SoundError::SoundInputNotFound(input_id))?;
        if input_data.owner() != self.processor_id {
            return Err(SoundError::BadSoundInputCleanup(input_id));
        }
        self.topology
            .remove_sound_input(input_id, self.processor_id)
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
        self.add_processor_argument_helper(Rc::new(ScalarProcessorExpressionArgument::new(
            self.processor_id,
            function,
        )))
    }

    /// Add an expression argument which reads an entire array from the
    /// sound processor's state using the provided function.
    /// NOTE that currently the length of that array must match
    /// the chunk length.
    pub fn add_processor_array_argument(
        &mut self,
        function: ArrayReadFunc,
    ) -> SoundExpressionArgumentHandle {
        self.add_processor_argument_helper(Rc::new(ArrayProcessorExpressionArgument::new(
            self.processor_id,
            function,
        )))
    }

    /// Add an expression argument which reads an entire array that
    /// is local to the sound processor's audio processing
    /// routine and must be provided with Context::push_processor_state.
    /// NOTE that currently the length of that array must match
    /// the chunk length.
    pub fn add_local_array_argument(&mut self) -> SoundExpressionArgumentHandle {
        self.add_processor_argument_helper(Rc::new(ProcessorLocalArrayExpressionArgument::new(
            self.processor_id,
        )))
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
        self.topology
            .add_expression(self.processor_id, default_value, scope)
            .unwrap()
    }

    /// Access the current sound graph topology
    pub(crate) fn topology(&self) -> &SoundGraph {
        self.topology
    }

    /// Mutably access the current sound graph topology
    // TODO: why?
    pub(crate) fn topology_mut(&mut self) -> &mut SoundGraph {
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

        let id = self
            .topology
            .add_expression_argument(instance, SoundExpressionArgumentOwner::SoundInput(input_id))
            .unwrap();

        SoundExpressionArgumentHandle::new(id)
    }

    /// Internal helper method for adding an expression to the sound processor
    /// which the tools were created for.
    fn add_processor_argument_helper(
        &mut self,
        instance: Rc<dyn SoundExpressionArgument>,
    ) -> SoundExpressionArgumentHandle {
        let id = self
            .topology
            .add_expression_argument(
                instance,
                SoundExpressionArgumentOwner::SoundProcessor(self.processor_id),
            )
            .unwrap();

        SoundExpressionArgumentHandle::new(id)
    }
}
