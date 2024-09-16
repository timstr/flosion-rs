use std::{collections::HashMap, rc::Rc};

use hashstash::{Order, Stashable};

use crate::{
    core::{sound::expression::SoundExpressionHandle, uniqueid::IdGenerator},
    ui_core::arguments::ParsedArguments,
};

use super::{
    expression::SoundExpressionId,
    expressionargument::{
        InputTimeExpressionArgument, ProcessorTimeExpressionArgument, SoundExpressionArgument,
        SoundExpressionArgumentId, SoundExpressionArgumentOwner,
    },
    sounderror::SoundError,
    soundgraphdata::{
        SoundExpressionArgumentData, SoundExpressionData, SoundExpressionScope, SoundInputBranchId,
        SoundInputData, SoundProcessorData,
    },
    soundgraphid::{SoundGraphId, SoundObjectId},
    soundgraphvalidation::find_sound_error,
    soundinput::{InputOptions, SoundInputId},
    soundprocessor::{
        DynamicSoundProcessor, DynamicSoundProcessorHandle, DynamicSoundProcessorWithId,
        SoundProcessorId, StaticSoundProcessor, StaticSoundProcessorHandle,
        StaticSoundProcessorWithId,
    },
    soundprocessortools::SoundProcessorTools,
};

#[derive(Clone)]
pub struct SoundGraph {
    sound_processors: HashMap<SoundProcessorId, SoundProcessorData>,
    sound_inputs: HashMap<SoundInputId, SoundInputData>,
    expression_arguments: HashMap<SoundExpressionArgumentId, SoundExpressionArgumentData>,
    expressions: HashMap<SoundExpressionId, SoundExpressionData>,

    sound_processor_idgen: IdGenerator<SoundProcessorId>,
    sound_input_idgen: IdGenerator<SoundInputId>,
    expression_argument_idgen: IdGenerator<SoundExpressionArgumentId>,
    expression_idgen: IdGenerator<SoundExpressionId>,
}

impl SoundGraph {
    pub fn new() -> SoundGraph {
        SoundGraph {
            sound_processors: HashMap::new(),
            sound_inputs: HashMap::new(),
            expression_arguments: HashMap::new(),
            expressions: HashMap::new(),
            sound_processor_idgen: IdGenerator::new(),
            sound_input_idgen: IdGenerator::new(),
            expression_argument_idgen: IdGenerator::new(),
            expression_idgen: IdGenerator::new(),
        }
    }

    /// Access the set of sound processors stored in the graph
    pub(crate) fn sound_processors(&self) -> &HashMap<SoundProcessorId, SoundProcessorData> {
        &self.sound_processors
    }

    /// Access the set of sound inputs stored in the graph
    pub(crate) fn sound_inputs(&self) -> &HashMap<SoundInputId, SoundInputData> {
        &self.sound_inputs
    }

    /// Access the set of expression arguments stored in the graph
    pub(crate) fn expression_arguments(
        &self,
    ) -> &HashMap<SoundExpressionArgumentId, SoundExpressionArgumentData> {
        &self.expression_arguments
    }

    /// Access the set of expressions stored in the graph
    pub(crate) fn expressions(&self) -> &HashMap<SoundExpressionId, SoundExpressionData> {
        &self.expressions
    }

    /// Look up a specific sound processor by its id
    pub(crate) fn sound_processor(&self, id: SoundProcessorId) -> Option<&SoundProcessorData> {
        self.sound_processors.get(&id)
    }

    /// Look up a specific sound input by its id
    pub(crate) fn sound_input(&self, id: SoundInputId) -> Option<&SoundInputData> {
        self.sound_inputs.get(&id)
    }

    /// Look up a specific sound input by its id with mutable access
    // TODO: remove?
    pub(crate) fn sound_input_mut(&mut self, id: SoundInputId) -> Option<&mut SoundInputData> {
        self.sound_inputs.get_mut(&id)
    }

    /// Look up a specific expression argument by its id
    pub(crate) fn expression_argument(
        &self,
        id: SoundExpressionArgumentId,
    ) -> Option<&SoundExpressionArgumentData> {
        self.expression_arguments.get(&id)
    }

    /// Look up a specific expression by its id
    pub(crate) fn expression(&self, id: SoundExpressionId) -> Option<&SoundExpressionData> {
        self.expressions.get(&id)
    }

    /// Look up a specific expression by its id with mutable access
    #[cfg(test)]
    pub(crate) fn expression_mut(
        &mut self,
        id: SoundExpressionId,
    ) -> Option<&mut SoundExpressionData> {
        self.expressions.get_mut(&id)
    }

    /// Returns an iterator listing all the sound inputs that are connected
    /// to the given sound processor, if any.
    pub(crate) fn sound_processor_targets<'a>(
        &'a self,
        id: SoundProcessorId,
    ) -> impl 'a + Iterator<Item = SoundInputId> {
        self.sound_inputs.values().filter_map(move |i| {
            if i.target() == Some(id) {
                Some(i.id())
            } else {
                None
            }
        })
    }

    /// Returns an iterator over the ids of all graph objects in the graph.
    ///
    /// NOTE that currently the only graph objects are sound processors.
    /// This may be expanded upon in the future.
    pub(crate) fn graph_object_ids<'a>(&'a self) -> impl 'a + Iterator<Item = SoundObjectId> {
        let sound_objects = self.sound_processors.values().map(|x| x.id().into());
        sound_objects
    }

    /// Add a static sound processor to the sound graph,
    /// i.e. a sound processor which always has a single
    /// instance running in realtime and cannot be replicated.
    /// The type must be known statically and given.
    /// For other ways of creating a sound processor,
    /// see ObjectFactory.
    pub fn add_static_sound_processor<T: 'static + StaticSoundProcessor>(
        &mut self,
        args: &ParsedArguments,
    ) -> Result<StaticSoundProcessorHandle<T>, SoundError> {
        let id = self.sound_processor_idgen.next_id();

        // Add a new processor data item to the graph,
        // but without the processor instance. This allows
        // the graph to be modified within the processor's
        // new() method, e.g. to add inputs.
        self.sound_processors
            .insert(id, SoundProcessorData::new_empty(id));

        // Every sound processor gets a 'time' expression argument
        let time_arg_id = self
            .add_expression_argument(
                Rc::new(ProcessorTimeExpressionArgument::new(id)),
                SoundExpressionArgumentOwner::SoundProcessor(id),
            )
            .unwrap();

        // The tools which the processor can use to give itself
        // new inputs, etc
        let tools = SoundProcessorTools::new(id, self);

        // construct the actual processor instance by its
        // concrete type
        let processor = T::new(tools, args).map_err(|_| SoundError::BadProcessorInit(id))?;

        // wrap the processor in a type-erased Rc
        let processor = Rc::new(StaticSoundProcessorWithId::new(processor, id, time_arg_id));
        let processor2 = Rc::clone(&processor);

        // add the missing processor instance to the
        // newly created processor data in the graph
        self.sound_processors
            .get_mut(&id)
            .unwrap()
            .set_processor(processor);

        Ok(StaticSoundProcessorHandle::new(processor2))
    }

    /// Add a dynamic sound processor to the sound graph,
    /// i.e. a sound processor which is replicated for each
    /// input it is connected to, which are run on-demand.
    /// The type must be known statically and given.
    /// For other ways of creating a sound processor,
    /// see ObjectFactory.
    pub fn add_dynamic_sound_processor<T: 'static + DynamicSoundProcessor>(
        &mut self,
        args: &ParsedArguments,
    ) -> Result<DynamicSoundProcessorHandle<T>, SoundError> {
        let id = self.sound_processor_idgen.next_id();

        // Add a new processor data item to the graph,
        // but without the processor instance. This allows
        // the graph to be modified within the processor's
        // new() method, e.g. to add inputs.
        self.sound_processors
            .insert(id, SoundProcessorData::new_empty(id));

        // Every sound processor gets a 'time' expression argument
        let time_arg_id = self
            .add_expression_argument(
                Rc::new(ProcessorTimeExpressionArgument::new(id)),
                SoundExpressionArgumentOwner::SoundProcessor(id),
            )
            .unwrap();

        // The tools which the processor can use to give itself
        // new inputs, etc
        let tools = SoundProcessorTools::new(id, self);

        // construct the actual processor instance by its
        // concrete type
        let processor = T::new(tools, args).map_err(|_| SoundError::BadProcessorInit(id))?;

        // wrap the processor in a type-erased Rc
        let processor = Rc::new(DynamicSoundProcessorWithId::new(processor, id, time_arg_id));
        let processor2 = Rc::clone(&processor);

        // add the missing processor instance to the
        // newly created processor data in the graph
        self.sound_processors
            .get_mut(&id)
            .unwrap()
            .set_processor(processor);

        Ok(DynamicSoundProcessorHandle::new(processor2))
    }

    pub fn remove_sound_processor(
        &mut self,
        processor_id: SoundProcessorId,
    ) -> Result<(), SoundError> {
        let mut expressions_to_remove = Vec::new();
        let mut expr_arguments_to_remove = Vec::new();
        let mut sound_inputs_to_remove = Vec::new();
        let mut sound_inputs_to_disconnect = Vec::new();

        let proc = self
            .sound_processor(processor_id)
            .ok_or(SoundError::ProcessorNotFound(processor_id))?;

        for ni in proc.expressions() {
            expressions_to_remove.push(*ni);
        }

        for ns in proc.expression_arguments() {
            expr_arguments_to_remove.push(*ns);
        }

        for si in proc.sound_inputs() {
            sound_inputs_to_remove.push(*si);
            let input = self.sound_input(*si).unwrap();
            for ns in input.expression_arguments() {
                expr_arguments_to_remove.push(*ns);
            }
            if self.sound_input(*si).unwrap().target().is_some() {
                sound_inputs_to_disconnect.push(*si);
            }
        }

        for si in self.sound_inputs.values() {
            if si.target() == Some(processor_id) {
                sound_inputs_to_disconnect.push(si.id());
            }
        }

        // ---

        for si in sound_inputs_to_disconnect {
            self.disconnect_sound_input(si)?;
        }

        for ni in expressions_to_remove {
            self.remove_expression(ni, processor_id)?;
        }

        for ns in expr_arguments_to_remove {
            self.remove_expression_argument(ns)?;
        }

        for si in sound_inputs_to_remove {
            self.remove_sound_input(si, processor_id)?;
        }

        self.sound_processors.remove(&processor_id);

        Ok(())
    }

    /// Add a sound input to the graph. The provided SoundInputData
    /// must have no expression arguments, its id must not yet be in use,
    /// and the sound processor to which it belongs must exist.
    pub(crate) fn add_sound_input(
        &mut self,
        owner: SoundProcessorId,
        options: InputOptions,
        branches: Vec<SoundInputBranchId>,
    ) -> Result<SoundInputId, SoundError> {
        if !self.sound_processors.contains_key(&owner) {
            return Err(SoundError::ProcessorNotFound(owner));
        }

        let id = self.sound_input_idgen.next_id();

        self.sound_inputs
            .insert(id, SoundInputData::new(id, options, branches, owner));

        self.sound_processors
            .get_mut(&owner)
            .unwrap()
            .sound_inputs_mut()
            .push(id);

        // Every sound input gets a 'time' expression argument
        let time_arg_id = self
            .add_expression_argument(
                Rc::new(InputTimeExpressionArgument::new(id)),
                SoundExpressionArgumentOwner::SoundInput(id),
            )
            .unwrap();

        self.sound_inputs
            .get_mut(&id)
            .unwrap()
            .set_time_argument(time_arg_id);

        Ok(id)
    }

    /// Remove a sound input from the graph.
    pub(crate) fn remove_sound_input(
        &mut self,
        input_id: SoundInputId,
        owner: SoundProcessorId,
    ) -> Result<(), SoundError> {
        let input = self
            .sound_input(input_id)
            .ok_or(SoundError::SoundInputNotFound(input_id))?;

        let has_target = input.target().is_some();

        let args = input.arguments().to_vec();

        if has_target {
            self.disconnect_sound_input(input_id).unwrap();
        }

        for arg in args {
            self.remove_expression_argument(arg).unwrap();
        }

        // remove the input from its owner
        let proc_data = self.sound_processors.get_mut(&owner).unwrap();
        debug_assert!(proc_data.sound_inputs().contains(&input_id));
        proc_data.sound_inputs_mut().retain(|iid| *iid != input_id);

        // remove the input
        self.sound_inputs.remove(&input_id).unwrap();

        Ok(())
    }

    /// Connect the given sound input to the given sound processor.
    /// Both the input and the processor must exist and the input
    /// must be unoccupied. No additional checks are performed.
    /// It is possible to create cycles using this method, even
    /// though cycles are generally not permitted. It is also
    /// possible to invalidate existing expression that rely
    /// on state from higher up the audio call stack by creating
    /// a separate pathway through which that state is not available.
    pub(crate) fn connect_sound_input(
        &mut self,
        input_id: SoundInputId,
        processor_id: SoundProcessorId,
    ) -> Result<(), SoundError> {
        if !self.sound_processors.contains_key(&processor_id) {
            return Err(SoundError::ProcessorNotFound(processor_id));
        }
        if !self.sound_inputs.contains_key(&input_id) {
            return Err(SoundError::SoundInputNotFound(input_id));
        }
        let input_data = self.sound_inputs.get_mut(&input_id).unwrap();
        if let Some(current_target) = input_data.target() {
            return Err(SoundError::SoundInputOccupied {
                input_id,
                current_target,
            });
        }
        input_data.set_target(Some(processor_id));
        Ok(())
    }

    /// Disconnect the given sound input from the processor it points to.
    /// The sound input must exist and it must be pointing to a sound
    /// processor already. No additional error checking is performed. It
    /// is possible to invalidate expression arguments which rely on state from
    /// higher up the audio call stack by removing their access to that
    /// state. For additional error checking, use SoundGraph instead or
    /// see find_sound_error.
    pub(crate) fn disconnect_sound_input(
        &mut self,
        input_id: SoundInputId,
    ) -> Result<(), SoundError> {
        let input_data = self
            .sound_inputs
            .get_mut(&input_id)
            .ok_or(SoundError::SoundInputNotFound(input_id))?;
        if input_data.target().is_none() {
            return Err(SoundError::SoundInputUnoccupied(input_id));
        }
        input_data.set_target(None);
        Ok(())
    }

    /// Add an expression argument to the graph. The arguments's
    /// id must not be in use yet and its owner (i.e. the sound processor
    /// or input to which it belongs) must already exist.
    // TODO: remove data from interface
    pub(super) fn add_expression_argument(
        &mut self,
        instance: Rc<dyn SoundExpressionArgument>,
        owner: SoundExpressionArgumentOwner,
    ) -> Result<SoundExpressionArgumentId, SoundError> {
        match owner {
            SoundExpressionArgumentOwner::SoundProcessor(spid) => {
                if !self.sound_processors.contains_key(&spid) {
                    return Err(SoundError::ProcessorNotFound(spid));
                }
            }
            SoundExpressionArgumentOwner::SoundInput(siid) => {
                if !self.sound_inputs.contains_key(&siid) {
                    return Err(SoundError::SoundInputNotFound(siid));
                }
            }
        }

        let id = self.expression_argument_idgen.next_id();

        let data = SoundExpressionArgumentData::new(id, instance, owner);

        match owner {
            SoundExpressionArgumentOwner::SoundProcessor(spid) => {
                self.sound_processors
                    .get_mut(&spid)
                    .unwrap()
                    .arguments_mut()
                    .push(id);
            }
            SoundExpressionArgumentOwner::SoundInput(siid) => {
                self.sound_inputs
                    .get_mut(&siid)
                    .unwrap()
                    .arguments_mut()
                    .push(id);
            }
        }

        let prev = self.expression_arguments.insert(id, data);
        debug_assert!(prev.is_none());

        Ok(id)
    }

    /// Remove an expression argument from the graph.
    pub(crate) fn remove_expression_argument(
        &mut self,
        argument_id: SoundExpressionArgumentId,
    ) -> Result<(), SoundError> {
        let owner = self
            .expression_arguments
            .get(&argument_id)
            .ok_or(SoundError::ArgumentNotFound(argument_id))?
            .owner();

        // remove the argument from its owner
        match owner {
            SoundExpressionArgumentOwner::SoundProcessor(spid) => {
                let proc_data = self.sound_processors.get_mut(&spid).unwrap();
                proc_data.arguments_mut().retain(|iid| *iid != argument_id);
            }
            SoundExpressionArgumentOwner::SoundInput(siid) => {
                let input_data = self.sound_inputs.get_mut(&siid).unwrap();
                input_data.arguments_mut().retain(|iid| *iid != argument_id);
            }
        }

        // remove the argument
        self.expression_arguments.remove(&argument_id).unwrap();

        Ok(())
    }

    /// Add an expression to the graph. The expressions's
    /// id must not yet be in use and it must not yet be connected
    /// to any expression arguments in its parameter mapping. The sound
    /// processor to which the input belongs must exist.
    pub(crate) fn add_expression(
        &mut self,
        owner: SoundProcessorId,
        default_value: f32,
        scope: SoundExpressionScope,
    ) -> Result<SoundExpressionHandle, SoundError> {
        if !self.sound_processors.contains_key(&owner) {
            return Err(SoundError::ProcessorNotFound(owner));
        }

        let id = self.expression_idgen.next_id();

        let data = SoundExpressionData::new(id, owner, default_value, scope.clone());

        let proc_data = self
            .sound_processors
            .get_mut(&data.owner())
            .ok_or(SoundError::ProcessorNotFound(data.owner()))?;
        debug_assert!(!proc_data.expressions().contains(&id));

        proc_data.expressions_mut().push(id);

        let prev = self.expressions.insert(id, data);
        debug_assert!(prev.is_none());

        Ok(SoundExpressionHandle::new(id, owner, scope))
    }

    /// Remove an expression from the graph.
    pub(crate) fn remove_expression(
        &mut self,
        id: SoundExpressionId,
        owner: SoundProcessorId,
    ) -> Result<(), SoundError> {
        self.expressions
            .remove(&id)
            .ok_or(SoundError::ExpressionNotFound(id))?;

        let proc_data = self.sound_processors.get_mut(&owner).unwrap();
        proc_data.expressions_mut().retain(|niid| *niid != id);

        Ok(())
    }

    /// Check whether the entity referred to by the given id exists in the graph
    pub fn contains<I: Into<SoundGraphId>>(&self, id: I) -> bool {
        let graph_id: SoundGraphId = id.into();
        match graph_id {
            SoundGraphId::SoundInput(siid) => self.sound_inputs.contains_key(&siid),
            SoundGraphId::SoundProcessor(spid) => self.sound_processors.contains_key(&spid),
            SoundGraphId::Expression(niid) => self.expressions.contains_key(&niid),
            SoundGraphId::ExpressionArgument(nsid) => self.expression_arguments.contains_key(&nsid),
        }
    }

    /// Create a SoundProcessorTools instance for making
    /// changes to the given sound processor and pass the tools to the
    /// provided closure. This is useful, for example, for example,
    /// for modifying sound inputs and expressions and arguments after
    /// the sound processor has been created.
    pub fn with_processor_tools<R, F: FnOnce(SoundProcessorTools) -> Result<R, SoundError>>(
        &mut self,
        processor_id: SoundProcessorId,
        f: F,
    ) -> Result<R, SoundError> {
        if !self.sound_processors.contains_key(&processor_id) {
            return Err(SoundError::ProcessorNotFound(processor_id));
        }
        self.try_make_change(|graph| {
            let tools = SoundProcessorTools::new(processor_id, graph);
            f(tools)
        })
    }

    /// Make changes to an expression using the given closure,
    /// which is passed a mutable instance of the input's
    /// SoundExpressionData.
    pub fn edit_expression<R, F: FnOnce(&mut SoundExpressionData) -> R>(
        &mut self,
        input_id: SoundExpressionId,
        f: F,
    ) -> Result<R, SoundError> {
        let expr = self
            .expressions
            .get_mut(&input_id)
            .ok_or(SoundError::ExpressionNotFound(input_id))?;

        Ok(f(expr))
    }

    /// Helper method for editing the sound graph, detecting errors,
    /// rolling back the changes if any errors were found, and otherwise
    /// keeping the change.
    pub fn try_make_change<R, F: FnOnce(&mut SoundGraph) -> Result<R, SoundError>>(
        &mut self,
        f: F,
    ) -> Result<R, SoundError> {
        if let Err(e) = self.validate() {
            panic!(
                "Called try_make_change() on a SoundGraph which is already invalid: {:?}",
                e.explain(self)
            );
        }
        let previous_graph = self.clone();
        let res = f(self);
        if res.is_err() {
            *self = previous_graph;
            return res;
        } else if let Err(e) = self.validate() {
            *self = previous_graph;
            return Err(e);
        }
        res
    }

    pub fn validate(&self) -> Result<(), SoundError> {
        match find_sound_error(self) {
            Some(e) => Err(e),
            None => Ok(()),
        }
    }
}

impl Stashable for SoundGraph {
    fn stash(&self, stasher: &mut hashstash::Stasher) {
        // sound processors
        stasher.array_of_proxy_objects(
            self.sound_processors.values(),
            |proc_data, stasher| {
                stasher.u64(proc_data.id().value() as u64);
                // TODO: processor instance?
                stasher
                    .array_of_u64_iter(proc_data.sound_inputs().iter().map(|i| i.value() as u64));
                stasher.array_of_u64_iter(proc_data.arguments().iter().map(|i| i.value() as u64));
                stasher.array_of_u64_iter(proc_data.expressions().iter().map(|i| i.value() as u64));
            },
            Order::Unordered,
        );

        // sound inputs
        stasher.array_of_proxy_objects(
            self.sound_inputs.values(),
            |input_data, stasher| {
                stasher.u64(input_data.id().value() as u64);
                stasher.u8(match input_data.options() {
                    InputOptions::Synchronous => 1,
                    InputOptions::NonSynchronous => 2,
                });
                stasher.array_of_u64_iter(input_data.branches().iter().map(|i| i.value() as u64));
                match input_data.target() {
                    Some(spid) => {
                        stasher.u8(1);
                        stasher.u64(spid.value() as u64);
                    }
                    None => stasher.u8(0),
                }
                stasher.u64(input_data.owner().value() as u64);
                stasher.array_of_u64_iter(input_data.arguments().iter().map(|i| i.value() as u64));
                // Not stashing time argument because it's assumed to be fixed
            },
            Order::Unordered,
        );

        // expression arguments
        stasher.array_of_proxy_objects(
            self.expression_arguments.values(),
            |arg_data, stasher| {
                stasher.u64(arg_data.id().value() as u64);
                // TODO: argument instance?
                match arg_data.owner() {
                    SoundExpressionArgumentOwner::SoundProcessor(spid) => {
                        stasher.u8(1);
                        stasher.u64(spid.value() as u64);
                    }
                    SoundExpressionArgumentOwner::SoundInput(siid) => {
                        stasher.u8(2);
                        stasher.u64(siid.value() as u64);
                    }
                }
            },
            Order::Unordered,
        );

        // expressions
        stasher.array_of_proxy_objects(
            self.expressions.values(),
            |expr_data, stasher| {
                stasher.u64(expr_data.id().value() as u64);

                stasher.array_of_proxy_objects(
                    expr_data.parameter_mapping().items().iter(),
                    |(param_id, arg_id), stasher| {
                        stasher.u64(param_id.value() as u64);
                        stasher.u64(arg_id.value() as u64);
                    },
                    Order::Unordered,
                );

                // TODO
                // expr_data.expression_graph()

                stasher.u64(expr_data.owner().value() as u64);

                stasher.object_proxy(|stasher| {
                    let scope = expr_data.scope();
                    stasher.bool(scope.processor_state_available());
                    stasher.array_of_u64_iter(
                        scope
                            .available_local_arguments()
                            .iter()
                            .map(|i| i.value() as u64),
                    );
                });
            },
            Order::Unordered,
        );
    }
}
